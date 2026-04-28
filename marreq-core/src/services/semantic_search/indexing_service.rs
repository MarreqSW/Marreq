// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Indexing service for managing requirement embeddings.
//!
//! Handles embedding generation, storage, and synchronization with requirements.

use super::config::SemanticSearchConfig;
use super::document_builder::{build_embedding_document, compute_content_hash, needs_reindex};
use super::embedding_provider::{create_embedding_provider, EmbeddingError};
use crate::app::{AppState, DieselCachedRepo};
use crate::models::{
    DecoratedRequirement, EmbeddingIndexStatus, NewEmbeddingIndexQueueEntry,
    NewRequirementEmbedding, ProjectIndexStatus, RequirementEmbedding,
};
use crate::repository::errors::RepoError;
use crate::schema::{embedding_index_queue, requirement_embeddings, requirements};
use crate::services::DecoratedRequirementService;
use chrono::Utc;
use diesel::prelude::*;
use pgvector::Vector;

/// Service for managing requirement embeddings.
pub struct IndexingService<'a> {
    state: &'a AppState<DieselCachedRepo>,
    config: SemanticSearchConfig,
}

impl<'a> IndexingService<'a> {
    /// Create a new indexing service.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self {
            state,
            config: SemanticSearchConfig::global().clone(),
        }
    }

    /// Create with custom configuration (for testing).
    pub fn with_config(
        state: &'a AppState<DieselCachedRepo>,
        config: SemanticSearchConfig,
    ) -> Self {
        Self { state, config }
    }

    /// Get the embedding configuration.
    pub fn config(&self) -> &SemanticSearchConfig {
        &self.config
    }

    /// Check if embeddings are enabled and configured.
    pub fn is_enabled(&self) -> bool {
        self.config.embeddings_enabled && self.config.is_valid_for_embeddings().is_ok()
    }

    /// Get the index status for a project.
    pub fn get_index_status(&self, project_id: i32) -> Result<ProjectIndexStatus, RepoError> {
        let repo = self.state.repo_read();

        // Get total requirements count
        let total_requirements: i64 = {
            let mut conn = repo.inner_repo().get_conn()?;
            requirements::table
                .filter(requirements::project_id.eq(project_id))
                .count()
                .get_result(conn.as_mut())
                .map_err(RepoError::Db)?
        };

        // Get indexed count
        let indexed_count: i64 = {
            let mut conn = repo.inner_repo().get_conn()?;
            requirement_embeddings::table
                .filter(requirement_embeddings::project_id.eq(project_id))
                .filter(requirement_embeddings::embedding.is_not_null())
                .count()
                .get_result(conn.as_mut())
                .map_err(RepoError::Db)?
        };

        // Get pending count
        let pending_count: i64 = {
            let mut conn = repo.inner_repo().get_conn()?;
            embedding_index_queue::table
                .filter(embedding_index_queue::project_id.eq(project_id))
                .filter(embedding_index_queue::status.eq("pending"))
                .count()
                .get_result(conn.as_mut())
                .map_err(RepoError::Db)?
        };

        // Get failed count
        let failed_count: i64 = {
            let mut conn = repo.inner_repo().get_conn()?;
            embedding_index_queue::table
                .filter(embedding_index_queue::project_id.eq(project_id))
                .filter(embedding_index_queue::status.eq("failed"))
                .count()
                .get_result(conn.as_mut())
                .map_err(RepoError::Db)?
        };

        // Get last indexed time
        let last_indexed_at: Option<chrono::NaiveDateTime> = {
            let mut conn = repo.inner_repo().get_conn()?;
            requirement_embeddings::table
                .filter(requirement_embeddings::project_id.eq(project_id))
                .select(diesel::dsl::max(requirement_embeddings::updated_at))
                .first(conn.as_mut())
                .map_err(RepoError::Db)?
        };

        Ok(ProjectIndexStatus {
            project_id,
            total_requirements,
            indexed_count,
            pending_count,
            failed_count,
            last_indexed_at,
            embeddings_enabled: self.config.embeddings_enabled,
            embedding_model: self.config.embedding_model.clone(),
        })
    }

    /// Get existing embedding for a requirement.
    pub fn get_embedding(
        &self,
        requirement_id: i32,
    ) -> Result<Option<RequirementEmbedding>, RepoError> {
        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn()?;

        requirement_embeddings::table
            .find(requirement_id)
            .select(RequirementEmbedding::as_select())
            .first(conn.as_mut())
            .optional()
            .map_err(RepoError::Db)
    }

    /// Index a single requirement.
    ///
    /// Generates embedding if needed (content changed or not indexed).
    pub async fn index_requirement(
        &self,
        req: &DecoratedRequirement,
    ) -> Result<bool, EmbeddingError> {
        if !self.is_enabled() {
            return Err(EmbeddingError::NotConfigured(
                "Embeddings are disabled".into(),
            ));
        }

        // Check if re-indexing is needed
        let existing = self.get_embedding(req.id).map_err(|e| {
            EmbeddingError::ApiError(format!("Failed to get existing embedding: {}", e))
        })?;

        let current_hash = existing.as_ref().map(|e| e.content_hash.as_str());

        if !needs_reindex(req, current_hash, &self.config.embedding_model) {
            return Ok(false); // No re-indexing needed
        }

        // Build document and compute hash
        let document = build_embedding_document(req);
        let content_hash = compute_content_hash(&document, &self.config.embedding_model);

        // Generate embedding
        let provider = create_embedding_provider(&self.config)?;
        let embedding_vec = provider.embed(&document).await?;
        let embedding = Vector::from(embedding_vec);

        // Upsert embedding
        let new_embedding = NewRequirementEmbedding {
            requirement_id: req.id,
            project_id: req.project_id,
            embedding: Some(embedding),
            embedding_model: self.config.embedding_model.clone(),
            content_hash,
            updated_at: Utc::now().naive_utc(),
        };

        let repo = self.state.repo_read();
        let mut conn = repo
            .inner_repo()
            .get_conn()
            .map_err(|e| EmbeddingError::ApiError(format!("Failed to get connection: {}", e)))?;

        diesel::insert_into(requirement_embeddings::table)
            .values(&new_embedding)
            .on_conflict(requirement_embeddings::requirement_id)
            .do_update()
            .set(&new_embedding)
            .execute(conn.as_mut())
            .map_err(|e| EmbeddingError::ApiError(format!("Failed to upsert embedding: {}", e)))?;

        Ok(true)
    }

    /// Reindex all requirements for a project.
    ///
    /// Returns (indexed_count, skipped_count, failed_count).
    pub async fn reindex_project(
        &self,
        project_id: i32,
    ) -> Result<(usize, usize, usize), EmbeddingError> {
        if !self.is_enabled() {
            return Err(EmbeddingError::NotConfigured(
                "Embeddings are disabled".into(),
            ));
        }

        // Get all decorated requirements for the project
        let requirements = DecoratedRequirementService::new(self.state)
            .list_by_project(project_id)
            .map_err(|e| EmbeddingError::ApiError(format!("Failed to list requirements: {}", e)))?;

        let mut indexed = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for req in &requirements {
            match self.index_requirement(req).await {
                Ok(true) => indexed += 1,
                Ok(false) => skipped += 1,
                Err(e) => {
                    eprintln!("Failed to index requirement {}: {}", req.id, e);
                    failed += 1;
                }
            }
        }

        Ok((indexed, skipped, failed))
    }

    /// Queue a requirement for background indexing.
    pub fn queue_for_indexing(
        &self,
        requirement_id: i32,
        project_id: i32,
    ) -> Result<(), RepoError> {
        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn()?;

        let entry = NewEmbeddingIndexQueueEntry {
            requirement_id,
            project_id,
            status: EmbeddingIndexStatus::Pending.as_str().to_string(),
        };

        diesel::insert_into(embedding_index_queue::table)
            .values(&entry)
            .on_conflict(embedding_index_queue::requirement_id)
            .do_update()
            .set((
                embedding_index_queue::status.eq(EmbeddingIndexStatus::Pending.as_str()),
                embedding_index_queue::error_message.eq::<Option<String>>(None),
                embedding_index_queue::created_at.eq(Utc::now().naive_utc()),
            ))
            .execute(conn.as_mut())
            .map_err(RepoError::Db)?;

        Ok(())
    }

    /// Process pending items in the index queue.
    ///
    /// Returns (processed_count, failed_count).
    pub async fn process_queue(&self, limit: i64) -> Result<(usize, usize), EmbeddingError> {
        if !self.is_enabled() {
            return Ok((0, 0));
        }

        let decorated_service = DecoratedRequirementService::new(self.state);

        // Get pending items
        let pending_ids: Vec<i32> = {
            let repo = self.state.repo_read();
            let mut conn = repo.inner_repo().get_conn().map_err(|e| {
                EmbeddingError::ApiError(format!("Failed to get connection: {}", e))
            })?;

            embedding_index_queue::table
                .filter(embedding_index_queue::status.eq("pending"))
                .order(embedding_index_queue::created_at.asc())
                .limit(limit)
                .select(embedding_index_queue::requirement_id)
                .load(conn.as_mut())
                .map_err(|e| EmbeddingError::ApiError(format!("Failed to get queue: {}", e)))?
        };

        let mut processed = 0;
        let mut failed = 0;

        for req_id in pending_ids {
            // Mark as processing
            self.update_queue_status(req_id, EmbeddingIndexStatus::Processing, None)
                .ok();

            // Get decorated requirement
            let req = match decorated_service.get_by_id(req_id) {
                Ok(r) => r,
                Err(e) => {
                    self.update_queue_status(
                        req_id,
                        EmbeddingIndexStatus::Failed,
                        Some(format!("Failed to get requirement: {}", e)),
                    )
                    .ok();
                    failed += 1;
                    continue;
                }
            };

            // Index the requirement
            match self.index_requirement(&req).await {
                Ok(_) => {
                    self.update_queue_status(req_id, EmbeddingIndexStatus::Completed, None)
                        .ok();
                    processed += 1;
                }
                Err(e) => {
                    self.update_queue_status(
                        req_id,
                        EmbeddingIndexStatus::Failed,
                        Some(e.to_string()),
                    )
                    .ok();
                    failed += 1;
                }
            }
        }

        Ok((processed, failed))
    }

    /// Update the status of a queue entry.
    fn update_queue_status(
        &self,
        requirement_id: i32,
        status: EmbeddingIndexStatus,
        error_message: Option<String>,
    ) -> Result<(), RepoError> {
        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn()?;

        let processed_at = if status == EmbeddingIndexStatus::Completed
            || status == EmbeddingIndexStatus::Failed
        {
            Some(Utc::now().naive_utc())
        } else {
            None
        };

        diesel::update(
            embedding_index_queue::table
                .filter(embedding_index_queue::requirement_id.eq(requirement_id)),
        )
        .set((
            embedding_index_queue::status.eq(status.as_str()),
            embedding_index_queue::error_message.eq(error_message),
            embedding_index_queue::processed_at.eq(processed_at),
        ))
        .execute(conn.as_mut())
        .map_err(RepoError::Db)?;

        Ok(())
    }

    /// Clear completed/failed entries from the queue.
    pub fn clear_processed_queue(&self, project_id: i32) -> Result<usize, RepoError> {
        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn()?;

        let deleted = diesel::delete(
            embedding_index_queue::table
                .filter(embedding_index_queue::project_id.eq(project_id))
                .filter(
                    embedding_index_queue::status
                        .eq("completed")
                        .or(embedding_index_queue::status.eq("failed")),
                ),
        )
        .execute(conn.as_mut())
        .map_err(RepoError::Db)?;

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EmbeddingIndexStatus;

    // Note: Full integration tests require database setup
    // These tests verify the service configuration logic

    #[test]
    fn service_disabled_by_default() {
        let config = SemanticSearchConfig::default();
        assert!(!config.embeddings_enabled);
    }

    #[test]
    fn service_enabled_with_mock() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_ok());
    }

    #[test]
    fn service_enabled_with_ollama() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "ollama".into(),
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_ok());
    }

    #[test]
    fn embedding_index_status_as_str() {
        assert_eq!(EmbeddingIndexStatus::Pending.as_str(), "pending");
        assert_eq!(EmbeddingIndexStatus::Processing.as_str(), "processing");
        assert_eq!(EmbeddingIndexStatus::Completed.as_str(), "completed");
        assert_eq!(EmbeddingIndexStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn embedding_index_status_from_str() {
        assert_eq!(
            EmbeddingIndexStatus::parse("pending"),
            Some(EmbeddingIndexStatus::Pending)
        );
        assert_eq!(
            EmbeddingIndexStatus::parse("processing"),
            Some(EmbeddingIndexStatus::Processing)
        );
        assert_eq!(
            EmbeddingIndexStatus::parse("completed"),
            Some(EmbeddingIndexStatus::Completed)
        );
        assert_eq!(
            EmbeddingIndexStatus::parse("failed"),
            Some(EmbeddingIndexStatus::Failed)
        );
        assert_eq!(EmbeddingIndexStatus::parse("unknown"), None);
        assert_eq!(EmbeddingIndexStatus::parse(""), None);
        assert_eq!(EmbeddingIndexStatus::parse("PENDING"), None); // Case sensitive
    }

    #[test]
    fn embedding_index_status_equality() {
        assert_eq!(EmbeddingIndexStatus::Pending, EmbeddingIndexStatus::Pending);
        assert_ne!(EmbeddingIndexStatus::Pending, EmbeddingIndexStatus::Failed);
    }

    #[test]
    fn config_validation_disabled() {
        let config = SemanticSearchConfig {
            embeddings_enabled: false,
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_err());
        assert!(config
            .is_valid_for_embeddings()
            .unwrap_err()
            .contains("disabled"));
    }

    #[test]
    fn config_validation_unknown_provider() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "unknown_provider".into(),
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_err());
        assert!(config
            .is_valid_for_embeddings()
            .unwrap_err()
            .contains("Unknown"));
    }

    #[test]
    fn config_rag_requires_embeddings() {
        let config = SemanticSearchConfig {
            embeddings_enabled: false,
            rag_enabled: true,
            ..Default::default()
        };
        assert!(config.is_valid_for_rag().is_err());
    }

    #[test]
    fn config_rag_disabled_error() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            rag_enabled: false,
            ..Default::default()
        };
        assert!(config.is_valid_for_rag().is_err());
        assert!(config.is_valid_for_rag().unwrap_err().contains("disabled"));
    }

    #[test]
    fn config_rag_valid() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            rag_enabled: true,
            ..Default::default()
        };
        assert!(config.is_valid_for_rag().is_ok());
    }

    #[test]
    fn new_embedding_index_queue_entry_fields() {
        let entry = NewEmbeddingIndexQueueEntry {
            requirement_id: 42,
            project_id: 1,
            status: EmbeddingIndexStatus::Pending.as_str().to_string(),
        };
        assert_eq!(entry.requirement_id, 42);
        assert_eq!(entry.project_id, 1);
        assert_eq!(entry.status, "pending");
    }

    #[test]
    fn project_index_status_fields() {
        let status = ProjectIndexStatus {
            project_id: 1,
            total_requirements: 100,
            indexed_count: 80,
            pending_count: 15,
            failed_count: 5,
            last_indexed_at: None,
            embeddings_enabled: true,
            embedding_model: "nomic-embed-text".into(),
        };

        assert_eq!(status.project_id, 1);
        assert_eq!(status.total_requirements, 100);
        assert_eq!(status.indexed_count, 80);
        assert_eq!(status.pending_count, 15);
        assert_eq!(status.failed_count, 5);
        assert!(status.last_indexed_at.is_none());
        assert!(status.embeddings_enabled);
        assert_eq!(status.embedding_model, "nomic-embed-text");
    }

    #[test]
    fn project_index_status_with_timestamp() {
        use chrono::NaiveDate;
        let timestamp = NaiveDate::from_ymd_opt(2025, 1, 30)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        let status = ProjectIndexStatus {
            project_id: 1,
            total_requirements: 50,
            indexed_count: 50,
            pending_count: 0,
            failed_count: 0,
            last_indexed_at: Some(timestamp),
            embeddings_enabled: true,
            embedding_model: "mxbai-embed-large".into(),
        };

        assert!(status.last_indexed_at.is_some());
        assert_eq!(status.last_indexed_at.unwrap(), timestamp);
    }

    #[test]
    fn embedding_error_not_configured() {
        let err = EmbeddingError::NotConfigured("test error".into());
        assert!(err.to_string().contains("not configured"));
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn embedding_error_api_error() {
        let err = EmbeddingError::ApiError("network timeout".into());
        assert!(err.to_string().contains("API request failed"));
        assert!(err.to_string().contains("network timeout"));
    }

    #[test]
    fn indexing_service_with_config_stores_config() {
        use crate::app::AppState;
        use crate::repository::diesel_repo_mock::DieselRepoMock;
        use crate::repository::CacheRepository;
        use std::sync::{Arc, RwLock};

        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                0,
            ))),
        };
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            ..Default::default()
        };
        let service = IndexingService::with_config(&state, config.clone());
        assert!(service.config().embeddings_enabled);
        assert_eq!(service.config().embedding_provider, "mock");
    }

    #[test]
    fn indexing_service_is_enabled_false_when_disabled() {
        use crate::app::AppState;
        use crate::repository::diesel_repo_mock::DieselRepoMock;
        use crate::repository::CacheRepository;
        use std::sync::{Arc, RwLock};

        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                0,
            ))),
        };
        let service = IndexingService::new(&state);
        assert!(!service.is_enabled());
    }

    #[test]
    fn indexing_service_is_enabled_true_with_valid_config() {
        use crate::app::AppState;
        use crate::repository::diesel_repo_mock::DieselRepoMock;
        use crate::repository::CacheRepository;
        use std::sync::{Arc, RwLock};

        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                0,
            ))),
        };
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            ..Default::default()
        };
        let service = IndexingService::with_config(&state, config);
        assert!(service.is_enabled());
    }
}
