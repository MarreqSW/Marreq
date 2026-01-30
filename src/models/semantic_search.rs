//! Models for semantic search functionality.
//!
//! These structures support the RAG-powered semantic search feature,
//! including embeddings storage and search indexing queue.

use crate::schema::{embedding_index_queue, requirement_embeddings};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use pgvector::Vector;
use serde::{Deserialize, Serialize};

/// Embedding status for the indexing queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingIndexStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl EmbeddingIndexStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "processing" => Some(Self::Processing),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Stored embedding for a requirement.
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = requirement_embeddings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RequirementEmbedding {
    pub requirement_id: i32,
    pub project_id: i32,
    #[serde(skip_serializing)]
    pub embedding: Option<Vector>,
    pub embedding_model: String,
    pub content_hash: String,
    pub updated_at: NaiveDateTime,
}

/// New embedding to insert or update.
#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = requirement_embeddings)]
pub struct NewRequirementEmbedding {
    pub requirement_id: i32,
    pub project_id: i32,
    pub embedding: Option<Vector>,
    pub embedding_model: String,
    pub content_hash: String,
    pub updated_at: NaiveDateTime,
}

/// Entry in the embedding index queue.
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = embedding_index_queue)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EmbeddingIndexQueueEntry {
    pub id: i32,
    pub requirement_id: i32,
    pub project_id: i32,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
    pub processed_at: Option<NaiveDateTime>,
}

/// New queue entry to insert.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = embedding_index_queue)]
pub struct NewEmbeddingIndexQueueEntry {
    pub requirement_id: i32,
    pub project_id: i32,
    pub status: String,
}

/// Result from semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub id: i32,
    pub reference_code: String,
    pub title: String,
    pub description: String,
    pub snippet: String,
    pub score: f32,
    pub rank: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lexical_rank: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_rank: Option<i32>,
    pub status: String,
    pub category: String,
    pub applicability: String,
    pub verification: String,
}

/// Index status for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndexStatus {
    pub project_id: i32,
    pub total_requirements: i64,
    pub indexed_count: i64,
    pub pending_count: i64,
    pub failed_count: i64,
    pub last_indexed_at: Option<NaiveDateTime>,
    pub embeddings_enabled: bool,
    pub embedding_model: String,
}

/// RAG answer response with citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagAnswerResponse {
    pub answer: String,
    pub citations: Vec<RagCitation>,
    pub results: Vec<SemanticSearchResult>,
}

/// Citation in a RAG answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagCitation {
    pub requirement_id: i32,
    pub reference_code: String,
    pub title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_index_status_roundtrip() {
        for status in [
            EmbeddingIndexStatus::Pending,
            EmbeddingIndexStatus::Processing,
            EmbeddingIndexStatus::Completed,
            EmbeddingIndexStatus::Failed,
        ] {
            let s = status.as_str();
            let parsed = EmbeddingIndexStatus::from_str(s);
            assert_eq!(parsed, Some(status));
        }
    }

    #[test]
    fn embedding_index_status_unknown_returns_none() {
        assert_eq!(EmbeddingIndexStatus::from_str("unknown"), None);
    }
}
