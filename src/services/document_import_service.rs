// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(clippy::too_many_lines)]

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{
    ActionType, Category, CustomFieldDefinition, CustomFieldValue, CustomFieldValueInput,
    EntityType, NewLog, NewMatrixLink, NewRequirement, NewRequirementContainer,
    NewRequirementVersionLink, NewVerification, Requirement, RequirementVersionVerificationMethod,
    User, Verification, VerificationMethod, VerificationStatus,
};
use crate::permissions::Permission;
use crate::repository::errors::RepoError;
use crate::repository::{
    CacheRepository, CustomFieldRepository, LogRepository, LookupRepository, MatrixRepository,
    RequirementVersionLinksRepository, RequirementsRepository, VerificationsRepository,
};
use crate::schema;
use crate::services::semantic_search::{
    create_llm_provider, ChatMessage, IndexingService, SearchFilters, SemanticSearchConfig,
    SemanticSearchService,
};
use crate::services::{
    ApplicabilityService, CategoryService, CustomFieldService, StatusService, UserService,
};
use crate::validation::validate_requirement;
use chrono::{DateTime, Utc};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};
use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use zip::ZipArchive;

const IMPORT_SESSION_TTL_SECS: u64 = 60 * 60;
const MAX_AI_BLOCKS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportedDocumentFormat {
    Pdf,
    Docx,
}

#[derive(Debug, thiserror::Error)]
pub enum DocumentImportError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Repo(#[from] RepoError),
}

impl From<diesel::result::Error> for DocumentImportError {
    fn from(value: diesel::result::Error) -> Self {
        RepoError::from(value).into()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Blocker,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
    pub candidate_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Requirement,
    Verification,
    TraceLink,
    RequirementLink,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFragment {
    pub source_ref: String,
    pub text: String,
    pub page_or_part: String,
    pub order: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateBlock {
    pub id: String,
    pub raw_text: String,
    pub source_refs: Vec<String>,
    pub kind_guess: BlockKind,
    pub confidence: f32,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedBlock {
    pub id: String,
    pub normalized_text: String,
    pub source_block_ids: Vec<String>,
    pub final_kind: BlockKind,
    pub context: Option<String>,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateSuggestion {
    pub match_kind: String,
    pub existing_id: i32,
    pub reference_code: String,
    pub title: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportDefaults {
    pub reviewer_id: Option<i32>,
    pub category_id: Option<i32>,
    pub applicability_id: Option<i32>,
    pub verification_status_id: Option<i32>,
    pub verification_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReviewState {
    pub defaults: ImportDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementCandidate {
    pub id: String,
    pub block_id: String,
    pub include: bool,
    pub title: String,
    pub description: String,
    pub reference_code: String,
    pub reviewer_id: Option<i32>,
    pub category_id: Option<i32>,
    pub applicability_id: Option<i32>,
    pub verification_method_ids: Vec<i32>,
    pub custom_fields: Vec<CustomFieldValueInput>,
    pub source_refs: Vec<String>,
    pub lineage_preview: String,
    pub confidence: f32,
    pub issues: Vec<ImportIssue>,
    pub duplicate_suggestions: Vec<DuplicateSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCandidate {
    pub id: String,
    pub block_id: String,
    pub include: bool,
    pub name: String,
    pub description: String,
    pub reference_code: String,
    pub source: Option<String>,
    pub status_id: Option<i32>,
    pub verification_method_id: Option<i32>,
    pub parent_reference_code: Option<String>,
    pub source_refs: Vec<String>,
    pub lineage_preview: String,
    pub confidence: f32,
    pub issues: Vec<ImportIssue>,
    pub duplicate_suggestions: Vec<DuplicateSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLinkCandidate {
    pub id: String,
    pub block_id: String,
    pub include: bool,
    pub requirement_reference_code: String,
    pub verification_reference_code: String,
    pub source_refs: Vec<String>,
    pub lineage_preview: String,
    pub confidence: f32,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementLinkCandidate {
    pub id: String,
    pub block_id: String,
    pub include: bool,
    pub source_requirement_reference_code: String,
    pub target_requirement_reference_code: String,
    pub link_type: Option<String>,
    pub rationale: Option<String>,
    pub source_refs: Vec<String>,
    pub lineage_preview: String,
    pub confidence: f32,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportCandidates {
    pub requirements: Vec<RequirementCandidate>,
    pub verifications: Vec<VerificationCandidate>,
    pub trace_links: Vec<TraceLinkCandidate>,
    pub requirement_links: Vec<RequirementLinkCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportDiagnostics {
    pub blockers: Vec<ImportIssue>,
    pub warnings: Vec<ImportIssue>,
    pub infos: Vec<ImportIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportSessionSummary {
    pub requirement_candidates: usize,
    pub verification_candidates: usize,
    pub trace_link_candidates: usize,
    pub requirement_link_candidates: usize,
    pub included_requirements: usize,
    pub included_verifications: usize,
    pub blockers: usize,
    pub warnings: usize,
    pub ready_to_commit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedRequirementCreate {
    pub candidate_id: String,
    pub title: String,
    pub description: String,
    pub reference_code: String,
    pub reviewer_id: i32,
    pub category_id: i32,
    pub applicability_id: i32,
    pub verification_method_ids: Vec<i32>,
    pub custom_fields: Vec<CustomFieldValueInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedVerificationCreate {
    pub candidate_id: String,
    pub name: String,
    pub description: String,
    pub reference_code: String,
    pub source: String,
    pub status_id: i32,
    pub verification_method_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTraceLinkCreate {
    pub candidate_id: String,
    pub requirement_reference_code: String,
    pub verification_reference_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedRequirementLinkCreate {
    pub candidate_id: String,
    pub source_requirement_reference_code: String,
    pub target_requirement_reference_code: String,
    pub link_type: String,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportPlan {
    pub requirements_to_create: Vec<PlannedRequirementCreate>,
    pub verifications_to_create: Vec<PlannedVerificationCreate>,
    pub trace_links_to_create: Vec<PlannedTraceLinkCreate>,
    pub requirement_links_to_create: Vec<PlannedRequirementLinkCreate>,
    pub blockers: Vec<ImportIssue>,
    pub warnings: Vec<ImportIssue>,
    pub ready_to_commit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewLookups {
    pub users: Vec<User>,
    pub categories: Vec<Category>,
    pub applicability: Vec<crate::models::Applicability>,
    pub verification_methods: Vec<VerificationMethod>,
    pub verification_statuses: Vec<VerificationStatus>,
    pub custom_fields: Vec<CustomFieldDefinition>,
    pub requirement_link_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSession {
    pub session_id: String,
    pub project_id: i32,
    pub user_id: i32,
    pub filename: String,
    pub ai_enabled: bool,
    pub extracted_fragments: Vec<ExtractedFragment>,
    pub blocks: Vec<CandidateBlock>,
    pub normalized_blocks: Vec<NormalizedBlock>,
    pub candidates: ImportCandidates,
    pub review_state: ReviewState,
    pub diagnostics: ImportDiagnostics,
    pub plan: ImportPlan,
    pub summary: ImportSessionSummary,
    pub lookups: PreviewLookups,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequirementReviewPatch {
    pub candidate_id: String,
    pub include: Option<bool>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub reference_code: Option<String>,
    pub reviewer_id: Option<i32>,
    pub category_id: Option<i32>,
    pub applicability_id: Option<i32>,
    pub verification_method_ids: Option<Vec<i32>>,
    pub custom_fields: Option<Vec<CustomFieldValueInput>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerificationReviewPatch {
    pub candidate_id: String,
    pub include: Option<bool>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub reference_code: Option<String>,
    pub source: Option<String>,
    pub status_id: Option<i32>,
    pub verification_method_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceLinkReviewPatch {
    pub candidate_id: String,
    pub include: Option<bool>,
    pub requirement_reference_code: Option<String>,
    pub verification_reference_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequirementLinkReviewPatch {
    pub candidate_id: String,
    pub include: Option<bool>,
    pub source_requirement_reference_code: Option<String>,
    pub target_requirement_reference_code: Option<String>,
    pub link_type: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReviewPatch {
    pub defaults: Option<ImportDefaults>,
    pub requirements: Option<Vec<RequirementReviewPatch>>,
    pub verifications: Option<Vec<VerificationReviewPatch>>,
    pub trace_links: Option<Vec<TraceLinkReviewPatch>>,
    pub requirement_links: Option<Vec<RequirementLinkReviewPatch>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    pub confirm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommitResult {
    pub created_requirement_ids: Vec<i32>,
    pub created_verification_ids: Vec<i32>,
    pub created_trace_links: usize,
    pub created_requirement_links: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiSuggestion {
    block_id: String,
    kind: BlockKind,
    reference_code: Option<String>,
    link_type: Option<String>,
}

#[derive(Debug, Clone)]
struct ProjectContextData {
    draft_status_id: Option<i32>,
    pending_verification_status_id: Option<i32>,
    users: Vec<User>,
    categories: Vec<Category>,
    applicability: Vec<crate::models::Applicability>,
    verification_methods: Vec<VerificationMethod>,
    verification_statuses: Vec<VerificationStatus>,
    custom_fields: Vec<CustomFieldDefinition>,
    existing_requirements: Vec<Requirement>,
    existing_verifications: Vec<Verification>,
}

pub struct DocumentImportService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> DocumentImportService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    pub fn ensure_project_edit_access(
        &self,
        user: &User,
        project_id: i32,
    ) -> Result<(), DocumentImportError> {
        let repo = self.state.repo_read();
        if crate::permissions::has_permission(
            &*repo,
            user,
            project_id,
            Permission::EditRequirements,
        ) {
            Ok(())
        } else {
            Err(DocumentImportError::BadRequest(
                "permission denied".to_string(),
            ))
        }
    }

    pub async fn create_session_from_bytes(
        &self,
        project_id: i32,
        user: &User,
        filename: &str,
        bytes: &[u8],
        ai_enabled: bool,
    ) -> Result<ImportSession, DocumentImportError> {
        let lookups = self.load_project_context(project_id)?;
        let extracted_fragments = self.extract_fragments(filename, bytes)?;
        if extracted_fragments.is_empty() {
            return Err(DocumentImportError::BadRequest(
                "No extractable text was found in the uploaded document.".to_string(),
            ));
        }

        let blocks = self.detect_candidate_blocks(&extracted_fragments);
        let normalized_blocks = self.normalize_blocks(&blocks);
        let mut candidates = self.tag_candidates(&normalized_blocks, &lookups, filename);
        let mut diagnostics = ImportDiagnostics::default();
        if ai_enabled {
            diagnostics.warnings.extend(
                self.apply_ai_suggestions(&mut candidates, &normalized_blocks, &lookups, filename)
                    .await,
            );
        }

        let session_id = make_session_id(project_id, user.id, filename);
        let mut session = ImportSession {
            session_id: session_id.clone(),
            project_id,
            user_id: user.id,
            filename: filename.to_string(),
            ai_enabled,
            extracted_fragments,
            blocks,
            normalized_blocks,
            candidates,
            review_state: ReviewState {
                defaults: ImportDefaults {
                    verification_status_id: lookups.pending_verification_status_id,
                    verification_source: Some(filename.to_string()),
                    ..ImportDefaults::default()
                },
            },
            diagnostics,
            plan: ImportPlan::default(),
            summary: ImportSessionSummary::default(),
            lookups: self.preview_lookups(&lookups),
            expires_at: Utc::now() + chrono::Duration::seconds(IMPORT_SESSION_TTL_SECS as i64),
        };
        self.rebuild_session(&mut session, &lookups).await?;
        self.store_session(&session)?;
        Ok(session)
    }

    pub fn get_session(
        &self,
        project_id: i32,
        user_id: i32,
        session_id: &str,
    ) -> Result<ImportSession, DocumentImportError> {
        let key = session_cache_key(project_id, user_id, session_id);
        let cache = self.state.repo_read().cache();
        let raw = cache.get(&key).ok_or_else(|| {
            DocumentImportError::NotFound("Document import session not found or expired.".into())
        })?;
        let session: ImportSession = serde_json::from_str(&raw).map_err(|e| {
            DocumentImportError::Internal(format!("Failed to read import session: {e}"))
        })?;
        if session.project_id != project_id || session.user_id != user_id {
            return Err(DocumentImportError::NotFound(
                "Document import session not found.".into(),
            ));
        }
        Ok(session)
    }

    pub async fn apply_review_patch(
        &self,
        project_id: i32,
        user_id: i32,
        session_id: &str,
        patch: ReviewPatch,
    ) -> Result<ImportSession, DocumentImportError> {
        let lookups = self.load_project_context(project_id)?;
        let mut session = self.get_session(project_id, user_id, session_id)?;

        if let Some(defaults) = patch.defaults {
            session.review_state.defaults = defaults;
        }

        if let Some(patches) = patch.requirements {
            for patch in patches {
                if let Some(candidate) = session
                    .candidates
                    .requirements
                    .iter_mut()
                    .find(|candidate| candidate.id == patch.candidate_id)
                {
                    apply_requirement_patch(candidate, patch);
                }
            }
        }

        if let Some(patches) = patch.verifications {
            for patch in patches {
                if let Some(candidate) = session
                    .candidates
                    .verifications
                    .iter_mut()
                    .find(|candidate| candidate.id == patch.candidate_id)
                {
                    apply_verification_patch(candidate, patch);
                }
            }
        }

        if let Some(patches) = patch.trace_links {
            for patch in patches {
                if let Some(candidate) = session
                    .candidates
                    .trace_links
                    .iter_mut()
                    .find(|candidate| candidate.id == patch.candidate_id)
                {
                    apply_trace_link_patch(candidate, patch);
                }
            }
        }

        if let Some(patches) = patch.requirement_links {
            for patch in patches {
                if let Some(candidate) = session
                    .candidates
                    .requirement_links
                    .iter_mut()
                    .find(|candidate| candidate.id == patch.candidate_id)
                {
                    apply_requirement_link_patch(candidate, patch);
                }
            }
        }

        self.rebuild_session(&mut session, &lookups).await?;
        self.store_session(&session)?;
        Ok(session)
    }

    pub fn delete_session(
        &self,
        project_id: i32,
        user_id: i32,
        session_id: &str,
    ) -> Result<(), DocumentImportError> {
        let key = session_cache_key(project_id, user_id, session_id);
        self.state.repo_read().cache().remove(&key);
        Ok(())
    }

    pub async fn commit_session(
        &self,
        project_id: i32,
        user: &User,
        session_id: &str,
        request: CommitRequest,
    ) -> Result<CommitResult, DocumentImportError> {
        if !request.confirm {
            return Err(DocumentImportError::BadRequest(
                "Explicit confirmation is required before import commit.".into(),
            ));
        }

        let session = self.get_session(project_id, user.id, session_id)?;
        if !session.plan.ready_to_commit {
            return Err(DocumentImportError::BadRequest(
                "The import session still has unresolved blockers.".into(),
            ));
        }

        #[cfg(test)]
        eprintln!("commit_session: executing commit");
        let result = self.execute_commit(project_id, user, &session)?;
        #[cfg(test)]
        eprintln!("commit_session: deleting session");
        self.delete_session(project_id, user.id, session_id)?;
        #[cfg(test)]
        eprintln!("commit_session: done");
        Ok(result)
    }

    async fn rebuild_session(
        &self,
        session: &mut ImportSession,
        lookups: &ProjectContextData,
    ) -> Result<(), DocumentImportError> {
        resolve_requirement_candidates(
            &mut session.candidates.requirements,
            &session.review_state.defaults,
            lookups,
        )?;
        resolve_verification_candidates(
            &mut session.candidates.verifications,
            &session.review_state.defaults,
            lookups,
        );
        resolve_trace_links(
            &mut session.candidates.trace_links,
            &session.candidates.requirements,
            &session.candidates.verifications,
            lookups,
        );
        resolve_requirement_links(
            &mut session.candidates.requirement_links,
            &session.candidates.requirements,
            lookups,
        );

        add_similarity_suggestions(
            &mut session.candidates.requirements,
            &mut session.candidates.verifications,
            lookups,
        );
        self.add_semantic_suggestions(session, lookups).await;

        session.plan = build_import_plan(&session.candidates, lookups);
        session.diagnostics = build_diagnostics(
            &session.candidates,
            &session.plan.blockers,
            &session.plan.warnings,
        );
        session.summary = build_summary(&session.candidates, &session.diagnostics);
        session.summary.ready_to_commit = session.plan.ready_to_commit;
        session.lookups = self.preview_lookups(lookups);
        session.expires_at = Utc::now() + chrono::Duration::seconds(IMPORT_SESSION_TTL_SECS as i64);
        Ok(())
    }

    fn preview_lookups(&self, lookups: &ProjectContextData) -> PreviewLookups {
        PreviewLookups {
            users: lookups.users.clone(),
            categories: lookups.categories.clone(),
            applicability: lookups.applicability.clone(),
            verification_methods: lookups.verification_methods.clone(),
            verification_statuses: lookups.verification_statuses.clone(),
            custom_fields: lookups.custom_fields.clone(),
            requirement_link_types:
                crate::services::requirement_service::REQUIREMENT_VERSION_LINK_TYPES
                    .iter()
                    .map(|item| (*item).to_string())
                    .collect(),
        }
    }

    fn store_session(&self, session: &ImportSession) -> Result<(), DocumentImportError> {
        let cache = self.state.repo_read().cache();
        let key = session_cache_key(session.project_id, session.user_id, &session.session_id);
        let value = serde_json::to_string(session).map_err(|e| {
            DocumentImportError::Internal(format!("Failed to serialize import session: {e}"))
        })?;
        cache.set_with_ttl(&key, value, Duration::from_secs(IMPORT_SESSION_TTL_SECS));
        Ok(())
    }

    fn load_project_context(
        &self,
        project_id: i32,
    ) -> Result<ProjectContextData, DocumentImportError> {
        let status_service = StatusService::new(self.state);
        let draft_status_id = status_service
            .resolve_requirement_status_by_title(project_id, "Draft")
            .map(|status| status.id);
        let pending_verification_status_id = status_service
            .resolve_verification_status_by_title(project_id, "Pending")
            .map(|status| status.id);
        let users = UserService::new(self.state).get_by_project(project_id)?;
        let categories = CategoryService::new(self.state).list_by_project(project_id)?;
        let applicability = ApplicabilityService::new(self.state).list_by_project(project_id)?;
        let custom_fields = CustomFieldService::new(self.state).list_by_project(project_id)?;
        let repo = self.state.repo_read();
        let verification_methods = repo.get_verification_methods_by_project(project_id)?;
        let verification_statuses = repo.get_verification_status_by_project(project_id)?;
        let existing_requirements = repo.get_requirements_by_project(project_id)?;
        let existing_verifications = repo.get_verifications_by_project(project_id)?;
        Ok(ProjectContextData {
            draft_status_id,
            pending_verification_status_id,
            users,
            categories,
            applicability,
            verification_methods,
            verification_statuses,
            custom_fields,
            existing_requirements,
            existing_verifications,
        })
    }

    fn extract_fragments(
        &self,
        filename: &str,
        bytes: &[u8],
    ) -> Result<Vec<ExtractedFragment>, DocumentImportError> {
        match detect_document_format(filename, bytes)? {
            ImportedDocumentFormat::Pdf => extract_pdf_fragments(bytes),
            ImportedDocumentFormat::Docx => extract_docx_fragments(bytes),
        }
    }

    fn detect_candidate_blocks(&self, fragments: &[ExtractedFragment]) -> Vec<CandidateBlock> {
        let mut blocks = Vec::new();
        let mut block_index = 1usize;
        let mut paragraph_buffer = Vec::new();
        let mut source_refs = Vec::new();

        let flush = |blocks: &mut Vec<CandidateBlock>,
                     paragraph_buffer: &mut Vec<String>,
                     source_refs: &mut Vec<String>,
                     block_index: &mut usize| {
            let joined = paragraph_buffer.join(" ");
            let text = normalize_whitespace(&joined);
            if text.is_empty() {
                paragraph_buffer.clear();
                source_refs.clear();
                return;
            }
            let (kind_guess, confidence, mut issues) = guess_block_kind(&text);
            if text.len() < 20 {
                issues.push(warning(
                    "short_block",
                    "Detected a very short block that may be noise.",
                    None,
                ));
            }
            blocks.push(CandidateBlock {
                id: format!("block-{block_index}"),
                raw_text: text,
                source_refs: source_refs.clone(),
                kind_guess,
                confidence,
                issues,
            });
            *block_index += 1;
            paragraph_buffer.clear();
            source_refs.clear();
        };

        for fragment in fragments {
            for raw_line in fragment.text.lines() {
                let line = raw_line.trim();
                if line.is_empty() {
                    flush(
                        &mut blocks,
                        &mut paragraph_buffer,
                        &mut source_refs,
                        &mut block_index,
                    );
                    continue;
                }
                paragraph_buffer.push(line.to_string());
                source_refs.push(fragment.source_ref.clone());
            }
            flush(
                &mut blocks,
                &mut paragraph_buffer,
                &mut source_refs,
                &mut block_index,
            );
        }

        blocks
    }

    fn normalize_blocks(&self, blocks: &[CandidateBlock]) -> Vec<NormalizedBlock> {
        let mut normalized = Vec::new();
        let mut next_id = 1usize;

        for block in blocks {
            let split_segments = split_normalized_segments(&block.raw_text);
            if split_segments.is_empty() {
                continue;
            }
            for segment in split_segments {
                let normalized_text = normalize_whitespace(&segment);
                if normalized_text.is_empty() {
                    continue;
                }
                let (final_kind, _, issues) = guess_block_kind(&normalized_text);
                normalized.push(NormalizedBlock {
                    id: format!("nblock-{next_id}"),
                    normalized_text,
                    source_block_ids: vec![block.id.clone()],
                    final_kind,
                    context: Some(block.raw_text.clone()),
                    issues,
                });
                next_id += 1;
            }
        }

        normalized
    }

    fn tag_candidates(
        &self,
        blocks: &[NormalizedBlock],
        lookups: &ProjectContextData,
        filename: &str,
    ) -> ImportCandidates {
        let mut candidates = ImportCandidates::default();
        let category_hint = match lookups.categories.as_slice() {
            [single] => Some(single.id),
            _ => None,
        };
        let applicability_hint = match lookups.applicability.as_slice() {
            [single] => Some(single.id),
            _ => None,
        };
        let verification_status_hint = lookups.pending_verification_status_id;

        for block in blocks {
            match block.final_kind {
                BlockKind::Requirement => candidates.requirements.push(
                    build_requirement_candidate(block, lookups, category_hint, applicability_hint),
                ),
                BlockKind::Verification => {
                    candidates.verifications.push(build_verification_candidate(
                        block,
                        lookups,
                        verification_status_hint,
                        filename,
                    ))
                }
                BlockKind::TraceLink => {
                    if let Some(candidate) = build_trace_link_candidate(block) {
                        candidates.trace_links.push(candidate);
                    }
                }
                BlockKind::RequirementLink => {
                    if let Some(candidate) = build_requirement_link_candidate(block) {
                        candidates.requirement_links.push(candidate);
                    }
                }
                BlockKind::Unknown => {}
            }
        }
        candidates
    }

    async fn apply_ai_suggestions(
        &self,
        candidates: &mut ImportCandidates,
        blocks: &[NormalizedBlock],
        lookups: &ProjectContextData,
        filename: &str,
    ) -> Vec<ImportIssue> {
        let config = SemanticSearchConfig::global();
        if !config.rag_enabled {
            return vec![warning(
                "ai_disabled",
                "AI-assisted classification was requested but RAG is disabled in configuration.",
                None,
            )];
        }

        let provider = match create_llm_provider(config) {
            Ok(provider) => provider,
            Err(err) => {
                return vec![warning(
                    "ai_unavailable",
                    &format!("AI-assisted classification is unavailable: {err}"),
                    None,
                )]
            }
        };

        let ambiguous: Vec<&NormalizedBlock> = blocks
            .iter()
            .filter(|block| block.final_kind == BlockKind::Unknown)
            .take(MAX_AI_BLOCKS)
            .collect();
        if ambiguous.is_empty() {
            return Vec::new();
        }

        let user_prompt = ambiguous
            .iter()
            .map(|block| format!("{}: {}", block.id, block.normalized_text))
            .collect::<Vec<_>>()
            .join("\n");
        let system_prompt = "Classify each block as requirement, verification, trace_link, requirement_link, or unknown. Reply with JSON array only. Each item must contain block_id, kind, optional reference_code, optional link_type.";

        let response = match provider
            .chat(
                &[
                    ChatMessage::system(system_prompt),
                    ChatMessage::user(user_prompt),
                ],
                512,
            )
            .await
        {
            Ok(response) => response,
            Err(err) => {
                return vec![warning(
                    "ai_failed",
                    &format!("AI-assisted classification failed: {err}"),
                    None,
                )]
            }
        };

        let suggestions = match extract_ai_suggestions(&response) {
            Ok(suggestions) => suggestions,
            Err(err) => {
                return vec![warning(
                    "ai_invalid_response",
                    &format!("AI response could not be parsed and was ignored: {err}"),
                    None,
                )]
            }
        };

        for suggestion in suggestions {
            let Some(block) = blocks.iter().find(|block| block.id == suggestion.block_id) else {
                continue;
            };
            match suggestion.kind {
                BlockKind::Requirement => {
                    if candidates
                        .requirements
                        .iter()
                        .any(|candidate| candidate.block_id == block.id)
                    {
                        continue;
                    }
                    let category_hint = match lookups.categories.as_slice() {
                        [single] => Some(single.id),
                        _ => None,
                    };
                    let applicability_hint = match lookups.applicability.as_slice() {
                        [single] => Some(single.id),
                        _ => None,
                    };
                    let mut candidate = build_requirement_candidate(
                        block,
                        lookups,
                        category_hint,
                        applicability_hint,
                    );
                    if let Some(reference_code) = suggestion.reference_code {
                        candidate.reference_code = reference_code;
                    }
                    candidate.issues.push(info(
                        "ai_suggested",
                        "AI suggested classifying this block as a requirement candidate.",
                        Some(candidate.id.clone()),
                    ));
                    candidates.requirements.push(candidate);
                }
                BlockKind::Verification => {
                    if candidates
                        .verifications
                        .iter()
                        .any(|candidate| candidate.block_id == block.id)
                    {
                        continue;
                    }
                    let mut candidate = build_verification_candidate(
                        block,
                        lookups,
                        lookups.pending_verification_status_id,
                        filename,
                    );
                    if let Some(reference_code) = suggestion.reference_code {
                        candidate.reference_code = reference_code;
                    }
                    candidate.issues.push(info(
                        "ai_suggested",
                        "AI suggested classifying this block as a verification candidate.",
                        Some(candidate.id.clone()),
                    ));
                    candidates.verifications.push(candidate);
                }
                _ => {}
            }
        }

        vec![info(
            "ai_applied",
            "AI-assisted suggestions were applied to ambiguous blocks.",
            None,
        )]
    }

    async fn add_semantic_suggestions(
        &self,
        session: &mut ImportSession,
        _lookups: &ProjectContextData,
    ) {
        let semantic = SemanticSearchService::new(self.state);
        if !semantic.is_enabled() {
            return;
        }

        for candidate in session
            .candidates
            .requirements
            .iter_mut()
            .filter(|candidate| candidate.include)
            .take(3)
        {
            let query = if candidate.reference_code.trim().is_empty() {
                candidate.title.clone()
            } else {
                format!("{} {}", candidate.reference_code, candidate.title)
            };
            let results = match semantic
                .search(session.project_id, &query, &SearchFilters::default(), 3)
                .await
            {
                Ok(results) => results,
                Err(_) => continue,
            };
            for result in results {
                if result.id <= 0 {
                    continue;
                }
                candidate.duplicate_suggestions.push(DuplicateSuggestion {
                    match_kind: "semantic".to_string(),
                    existing_id: result.id,
                    reference_code: result.reference_code,
                    title: result.title,
                    score: result.score,
                });
            }
        }
    }

    fn execute_commit(
        &self,
        project_id: i32,
        user: &User,
        session: &ImportSession,
    ) -> Result<CommitResult, DocumentImportError> {
        let project_context = self.load_project_context(project_id)?;
        let conn_attempt = self.state.repo_read().inner_repo().get_conn();
        if let Ok(mut conn) = conn_attempt {
            let result = conn
                .as_mut()
                .transaction::<CommitResult, DocumentImportError, _>(|tx| {
                    self.execute_commit_tx(project_id, tx, user, session, &project_context)
                })?;
            self.invalidate_project_cache(project_id);
            self.queue_requirements_for_indexing(project_id, &result.created_requirement_ids);
            return Ok(result);
        }

        let result = {
            let mut repo = self.state.repo_write();
            let snapshot = repo.inner_repo().clone();
            let result =
                self.execute_commit_repo(project_id, &mut *repo, user, session, &project_context);
            if result.is_err() {
                *repo.inner_repo_mut() = snapshot;
            }
            result
        };
        if let Ok(ref result) = result {
            self.invalidate_project_cache(project_id);
            self.queue_requirements_for_indexing(project_id, &result.created_requirement_ids);
        }
        result
    }

    fn execute_commit_tx(
        &self,
        project_id: i32,
        conn: &mut diesel::pg::PgConnection,
        user: &User,
        session: &ImportSession,
        project_context: &ProjectContextData,
    ) -> Result<CommitResult, DocumentImportError> {
        let repo = self.state.repo_read();
        let existing_requirements = repo.get_requirements_by_project(project_id)?;
        let existing_verifications = repo.get_verifications_by_project(project_id)?;
        drop(repo);
        let draft_status_id = project_context.draft_status_id.ok_or_else(|| {
            DocumentImportError::BadRequest(
                "Project Draft status is required for document import.".into(),
            )
        })?;

        let existing_req_map: HashMap<String, (i32, Option<i32>)> = existing_requirements
            .into_iter()
            .map(|requirement| {
                (
                    requirement.reference_code.to_ascii_uppercase(),
                    (requirement.id, requirement.current_version_id),
                )
            })
            .collect();
        let existing_verification_map: HashMap<String, i32> = existing_verifications
            .into_iter()
            .map(|verification| {
                (
                    verification.reference_code.to_ascii_uppercase(),
                    verification.id,
                )
            })
            .collect();

        let mut created_requirement_ids = Vec::new();
        let mut created_requirement_versions = HashMap::new();
        let mut created_requirement_refs = HashMap::new();
        let mut created_verification_ids = Vec::new();
        let mut created_verification_refs = HashMap::new();
        let mut created_trace_links = 0usize;
        let mut created_requirement_links = 0usize;

        for planned in dedupe_verifications(&session.plan.verifications_to_create) {
            let payload = NewVerification {
                id: None,
                name: planned.name.clone(),
                reference_code: planned.reference_code.clone(),
                description: planned.description.clone(),
                source: planned.source.clone(),
                status_id: planned.status_id,
                parent_id: None,
                project_id,
                verification_method_id: planned.verification_method_id,
            };
            let verification_id: i32 = diesel::insert_into(schema::verifications::table)
                .values(&payload)
                .returning(schema::verifications::id)
                .get_result(conn)
                .map_err(RepoError::from)?;
            created_verification_ids.push(verification_id);
            created_verification_refs
                .insert(planned.reference_code.to_ascii_uppercase(), verification_id);
        }

        for planned in dedupe_requirements(&session.plan.requirements_to_create) {
            let container = NewRequirementContainer {
                project_id,
                stable_code: planned.reference_code.clone(),
                current_version_id: None,
            };
            let requirement_id: i32 = diesel::insert_into(schema::requirements::table)
                .values(&container)
                .returning(schema::requirements::id)
                .get_result(conn)
                .map_err(RepoError::from)?;

            let payload = NewRequirement {
                id: Some(requirement_id),
                title: planned.title.clone(),
                description: planned.description.clone(),
                author_id: user.id,
                category_id: planned.category_id,
                status_id: draft_status_id,
                reference_code: planned.reference_code.clone(),
                reviewer_id: planned.reviewer_id,
                applicability_id: planned.applicability_id,
                justification: None,
                project_id,
            };
            let requirement_payload = payload;
            let version = requirement_payload.to_new_version(requirement_id);
            let (version_id, created_at): (i32, chrono::NaiveDateTime) =
                diesel::insert_into(schema::requirement_versions::table)
                    .values(&version)
                    .returning((
                        schema::requirement_versions::id,
                        schema::requirement_versions::created_at,
                    ))
                    .get_result(conn)
                    .map_err(RepoError::from)?;
            diesel::update(
                schema::requirements::table.filter(schema::requirements::id.eq(requirement_id)),
            )
            .set((
                schema::requirements::current_version_id.eq(version_id),
                schema::requirements::first_created_at.eq(created_at),
            ))
            .execute(conn)
            .map_err(RepoError::from)?;

            let verification_methods: Vec<RequirementVersionVerificationMethod> = planned
                .verification_method_ids
                .iter()
                .copied()
                .filter(|verification_method_id| *verification_method_id > 0)
                .collect::<HashSet<_>>()
                .into_iter()
                .map(
                    |verification_method_id| RequirementVersionVerificationMethod {
                        requirement_version_id: version_id,
                        verification_method_id,
                    },
                )
                .collect();
            if !verification_methods.is_empty() {
                diesel::insert_into(schema::requirement_version_verification_methods::table)
                    .values(&verification_methods)
                    .execute(conn)
                    .map_err(RepoError::from)?;
            }

            let custom_fields: Vec<CustomFieldValue> = planned
                .custom_fields
                .iter()
                .map(|field| CustomFieldValue {
                    requirement_version_id: version_id,
                    custom_field_definition_id: field.field_id,
                    value: field.value.clone(),
                })
                .collect();
            if !custom_fields.is_empty() {
                diesel::insert_into(schema::custom_field_values::table)
                    .values(&custom_fields)
                    .execute(conn)
                    .map_err(RepoError::from)?;
            }

            created_requirement_ids.push(requirement_id);
            created_requirement_versions
                .insert(planned.reference_code.to_ascii_uppercase(), version_id);
            created_requirement_refs
                .insert(planned.reference_code.to_ascii_uppercase(), requirement_id);
        }

        for planned in dedupe_trace_links(&session.plan.trace_links_to_create) {
            let requirement_key = planned.requirement_reference_code.to_ascii_uppercase();
            let verification_key = planned.verification_reference_code.to_ascii_uppercase();
            let req_id = created_requirement_refs
                .get(&requirement_key)
                .copied()
                .or_else(|| {
                    existing_req_map
                        .get(&requirement_key)
                        .map(|(req_id, _)| *req_id)
                })
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve requirement reference '{}'.",
                        planned.requirement_reference_code
                    ))
                })?;
            let verification_id = created_verification_refs
                .get(&verification_key)
                .copied()
                .or_else(|| existing_verification_map.get(&verification_key).copied())
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve verification reference '{}'.",
                        planned.verification_reference_code
                    ))
                })?;
            let payload = NewMatrixLink {
                req_id,
                verification_id,
                project_id,
                triggering_version_id: created_requirement_versions.get(&requirement_key).copied(),
                triggering_user_id: Some(user.id),
            };
            diesel::insert_into(schema::matrix::table)
                .values(&payload)
                .on_conflict_do_nothing()
                .execute(conn)
                .map_err(RepoError::from)?;
            created_trace_links += 1;
        }

        for planned in dedupe_requirement_links(&session.plan.requirement_links_to_create) {
            let source_key = planned
                .source_requirement_reference_code
                .to_ascii_uppercase();
            let target_key = planned
                .target_requirement_reference_code
                .to_ascii_uppercase();
            let source_version_id = created_requirement_versions
                .get(&source_key)
                .copied()
                .or_else(|| {
                    existing_req_map
                        .get(&source_key)
                        .and_then(|(_, version_id)| *version_id)
                })
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve source requirement reference '{}'.",
                        planned.source_requirement_reference_code
                    ))
                })?;
            let target_version_id = created_requirement_versions
                .get(&target_key)
                .copied()
                .or_else(|| {
                    existing_req_map
                        .get(&target_key)
                        .and_then(|(_, version_id)| *version_id)
                })
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve target requirement reference '{}'.",
                        planned.target_requirement_reference_code
                    ))
                })?;
            let payload = NewRequirementVersionLink {
                source_version_id,
                target_version_id,
                link_type: planned.link_type.clone(),
                rationale: planned.rationale.clone(),
                project_id,
                metadata: None,
            };
            diesel::insert_into(schema::requirement_version_links::table)
                .values(&payload)
                .on_conflict_do_nothing()
                .execute(conn)
                .map_err(RepoError::from)?;
            created_requirement_links += 1;
        }

        Logger::log_custom(
            conn,
            &LogCtx::new(user.id),
            ActionType::Import,
            EntityType::Project,
            Some(project_id),
            Some(project_id),
            None,
            Some(
                serde_json::json!({
                    "session_id": session.session_id,
                    "requirements": created_requirement_ids,
                    "verifications": created_verification_ids,
                })
                .to_string(),
            ),
            Some("Imported data from a dry-run document import session.".into()),
        )
        .map_err(|err| DocumentImportError::Internal(err.to_string()))?;

        Ok(CommitResult {
            created_requirement_ids,
            created_verification_ids,
            created_trace_links,
            created_requirement_links,
            warnings: session
                .diagnostics
                .warnings
                .iter()
                .map(|issue| issue.message.clone())
                .collect(),
        })
    }

    fn execute_commit_repo<R>(
        &self,
        project_id: i32,
        repo: &mut CacheRepository<R>,
        user: &User,
        session: &ImportSession,
        project_context: &ProjectContextData,
    ) -> Result<CommitResult, DocumentImportError>
    where
        R: crate::repository::Repository + Clone,
    {
        let existing_requirements = repo.get_requirements_by_project(project_id)?;
        let existing_verifications = repo.get_verifications_by_project(project_id)?;
        let draft_status_id = project_context.draft_status_id.ok_or_else(|| {
            DocumentImportError::BadRequest(
                "Project Draft status is required for document import.".into(),
            )
        })?;

        let existing_req_map: HashMap<String, (i32, Option<i32>)> = existing_requirements
            .into_iter()
            .map(|requirement| {
                (
                    requirement.reference_code.to_ascii_uppercase(),
                    (requirement.id, requirement.current_version_id),
                )
            })
            .collect();
        let existing_verification_map: HashMap<String, i32> = existing_verifications
            .into_iter()
            .map(|verification| {
                (
                    verification.reference_code.to_ascii_uppercase(),
                    verification.id,
                )
            })
            .collect();

        let mut created_requirement_ids = Vec::new();
        let mut created_requirement_versions = HashMap::new();
        let mut created_requirement_refs = HashMap::new();
        let mut created_verification_ids = Vec::new();
        let mut created_verification_refs = HashMap::new();

        for planned in dedupe_verifications(&session.plan.verifications_to_create) {
            let payload = NewVerification {
                id: None,
                name: planned.name.clone(),
                reference_code: planned.reference_code.clone(),
                description: planned.description.clone(),
                source: planned.source.clone(),
                status_id: planned.status_id,
                parent_id: None,
                project_id,
                verification_method_id: planned.verification_method_id,
            };
            let verification_id = repo.insert_verification(&payload)?;
            created_verification_ids.push(verification_id);
            created_verification_refs
                .insert(planned.reference_code.to_ascii_uppercase(), verification_id);
        }

        for planned in dedupe_requirements(&session.plan.requirements_to_create) {
            let payload = NewRequirement {
                id: None,
                title: planned.title.clone(),
                description: planned.description.clone(),
                author_id: user.id,
                category_id: planned.category_id,
                status_id: draft_status_id,
                reference_code: planned.reference_code.clone(),
                reviewer_id: planned.reviewer_id,
                applicability_id: planned.applicability_id,
                justification: None,
                project_id,
            };
            let requirement_id = repo.insert_new_requirement(&payload)?;
            repo.set_requirement_verification_methods(
                requirement_id,
                &planned.verification_method_ids,
            )?;
            let stored = repo.get_requirement_by_id(requirement_id)?;
            if let Some(version_id) = stored.current_version_id {
                let custom_values: Vec<(i32, Option<String>)> = planned
                    .custom_fields
                    .iter()
                    .map(|field| (field.field_id, field.value.clone()))
                    .collect();
                if !custom_values.is_empty() {
                    repo.set_custom_field_values_for_version(version_id, &custom_values)?;
                }
                created_requirement_versions
                    .insert(planned.reference_code.to_ascii_uppercase(), version_id);
            }
            created_requirement_ids.push(requirement_id);
            created_requirement_refs
                .insert(planned.reference_code.to_ascii_uppercase(), requirement_id);
        }

        let mut created_trace_links = 0usize;
        for planned in dedupe_trace_links(&session.plan.trace_links_to_create) {
            let requirement_key = planned.requirement_reference_code.to_ascii_uppercase();
            let verification_key = planned.verification_reference_code.to_ascii_uppercase();
            let req_id = created_requirement_refs
                .get(&requirement_key)
                .copied()
                .or_else(|| {
                    existing_req_map
                        .get(&requirement_key)
                        .map(|(req_id, _)| *req_id)
                })
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve requirement reference '{}'.",
                        planned.requirement_reference_code
                    ))
                })?;
            let verification_id = created_verification_refs
                .get(&verification_key)
                .copied()
                .or_else(|| existing_verification_map.get(&verification_key).copied())
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve verification reference '{}'.",
                        planned.verification_reference_code
                    ))
                })?;
            repo.insert_new_matrix_item(&NewMatrixLink {
                req_id,
                verification_id,
                project_id,
                triggering_version_id: created_requirement_versions.get(&requirement_key).copied(),
                triggering_user_id: Some(user.id),
            })?;
            created_trace_links += 1;
        }

        let mut created_requirement_links = 0usize;
        for planned in dedupe_requirement_links(&session.plan.requirement_links_to_create) {
            let source_key = planned
                .source_requirement_reference_code
                .to_ascii_uppercase();
            let target_key = planned
                .target_requirement_reference_code
                .to_ascii_uppercase();
            let source_version_id = created_requirement_versions
                .get(&source_key)
                .copied()
                .or_else(|| {
                    existing_req_map
                        .get(&source_key)
                        .and_then(|(_, version_id)| *version_id)
                })
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve source requirement reference '{}'.",
                        planned.source_requirement_reference_code
                    ))
                })?;
            let target_version_id = created_requirement_versions
                .get(&target_key)
                .copied()
                .or_else(|| {
                    existing_req_map
                        .get(&target_key)
                        .and_then(|(_, version_id)| *version_id)
                })
                .ok_or_else(|| {
                    DocumentImportError::BadRequest(format!(
                        "Unable to resolve target requirement reference '{}'.",
                        planned.target_requirement_reference_code
                    ))
                })?;
            let _ = repo.insert_requirement_version_link(&NewRequirementVersionLink {
                source_version_id,
                target_version_id,
                link_type: planned.link_type.clone(),
                rationale: planned.rationale.clone(),
                project_id,
                metadata: None,
            })?;
            created_requirement_links += 1;
        }

        repo.insert_log(&NewLog {
            user_id: user.id,
            action_type: ActionType::Import.to_string(),
            entity_type: EntityType::Project.to_string(),
            entity_id: Some(project_id),
            project_id: Some(project_id),
            old_values: None,
            new_values: Some(
                serde_json::json!({
                    "session_id": session.session_id,
                    "requirements": created_requirement_ids,
                    "verifications": created_verification_ids,
                })
                .to_string(),
            ),
            description: Some("Imported data from a dry-run document import session.".into()),
            ip_address: None,
            user_agent: None,
        })?;

        Ok(CommitResult {
            created_requirement_ids,
            created_verification_ids,
            created_trace_links,
            created_requirement_links,
            warnings: session
                .diagnostics
                .warnings
                .iter()
                .map(|issue| issue.message.clone())
                .collect(),
        })
    }

    fn invalidate_project_cache(&self, project_id: i32) {
        let cache = self.state.repo_read().cache();
        cache.invalidate_project(project_id);
    }

    fn queue_requirements_for_indexing(&self, project_id: i32, requirement_ids: &[i32]) {
        let config = SemanticSearchConfig::global();
        if !config.embeddings_enabled || requirement_ids.is_empty() {
            return;
        }

        let indexing_service = IndexingService::new(self.state);
        for requirement_id in requirement_ids {
            let _ = indexing_service.queue_for_indexing(*requirement_id, project_id);
        }
    }
}

fn extract_pdf_fragments(bytes: &[u8]) -> Result<Vec<ExtractedFragment>, DocumentImportError> {
    let pages = pdf_extract::extract_text_from_mem_by_pages(bytes).map_err(|err| {
        let message = err.to_string();
        if message.to_ascii_lowercase().contains("encrypted") {
            DocumentImportError::BadRequest(
                "Encrypted PDF documents are not supported for import.".into(),
            )
        } else {
            DocumentImportError::BadRequest(format!("Unable to read PDF text: {message}"))
        }
    })?;
    let fragments = pages
        .into_iter()
        .enumerate()
        .filter_map(|(index, text)| {
            let normalized = normalize_whitespace(&text);
            if normalized.is_empty() {
                None
            } else {
                Some(ExtractedFragment {
                    source_ref: format!("pdf-page-{}", index + 1),
                    page_or_part: format!("Page {}", index + 1),
                    order: index,
                    text,
                })
            }
        })
        .collect::<Vec<_>>();
    if fragments.is_empty() {
        return Err(DocumentImportError::BadRequest(
            "The PDF did not contain extractable text. Scanned PDFs are not supported in v1."
                .into(),
        ));
    }
    Ok(fragments)
}

fn detect_document_format(
    filename: &str,
    bytes: &[u8],
) -> Result<ImportedDocumentFormat, DocumentImportError> {
    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    match extension.as_str() {
        "pdf" => return Ok(ImportedDocumentFormat::Pdf),
        "docx" => return Ok(ImportedDocumentFormat::Docx),
        "doc" => {
            return Err(DocumentImportError::BadRequest(
                "Legacy .doc uploads are not supported. Please convert the file to .docx.".into(),
            ))
        }
        _ => {}
    }

    if bytes.starts_with(b"%PDF-") {
        return Ok(ImportedDocumentFormat::Pdf);
    }

    if looks_like_docx(bytes) {
        return Ok(ImportedDocumentFormat::Docx);
    }

    if extension.is_empty() {
        Err(DocumentImportError::BadRequest(
            "Unsupported file type. Use .pdf or .docx.".into(),
        ))
    } else {
        Err(DocumentImportError::BadRequest(format!(
            "Unsupported file type '{extension}'. Use .pdf or .docx."
        )))
    }
}

fn looks_like_docx(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"PK") {
        return false;
    }

    let Ok(mut archive) = ZipArchive::new(Cursor::new(bytes)) else {
        return false;
    };
    let has_document_xml = archive.by_name("word/document.xml").is_ok();
    has_document_xml
}

fn extract_docx_fragments(bytes: &[u8]) -> Result<Vec<ExtractedFragment>, DocumentImportError> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)
        .map_err(|err| DocumentImportError::BadRequest(format!("Invalid DOCX archive: {err}")))?;
    let mut document = archive.by_name("word/document.xml").map_err(|err| {
        DocumentImportError::BadRequest(format!("DOCX document.xml is missing: {err}"))
    })?;
    let mut xml = String::new();
    document.read_to_string(&mut xml).map_err(|err| {
        DocumentImportError::BadRequest(format!("Could not read DOCX content: {err}"))
    })?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current_paragraph = Vec::new();
    let mut fragments = Vec::new();
    let mut paragraph_index = 1usize;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref event)) if event.local_name().as_ref() == b"p" => {
                current_paragraph.clear();
            }
            Ok(Event::End(ref event)) if event.local_name().as_ref() == b"p" => {
                let text = normalize_whitespace(&current_paragraph.join(" "));
                if !text.is_empty() {
                    fragments.push(ExtractedFragment {
                        source_ref: format!("docx-paragraph-{paragraph_index}"),
                        text,
                        page_or_part: format!("Paragraph {paragraph_index}"),
                        order: paragraph_index - 1,
                    });
                    paragraph_index += 1;
                }
                current_paragraph.clear();
            }
            Ok(Event::Text(text)) => {
                let unescaped = text
                    .decode()
                    .map_err(|err| {
                        DocumentImportError::BadRequest(format!(
                            "Could not decode DOCX text node: {err}"
                        ))
                    })?
                    .into_owned();
                current_paragraph.push(unescaped);
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(DocumentImportError::BadRequest(format!(
                    "Could not parse DOCX XML: {err}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    if fragments.is_empty() {
        return Err(DocumentImportError::BadRequest(
            "The DOCX document did not contain extractable text.".into(),
        ));
    }
    Ok(fragments)
}

fn guess_block_kind(text: &str) -> (BlockKind, f32, Vec<ImportIssue>) {
    let lower = text.to_ascii_lowercase();
    let codes = extract_reference_codes(text);
    if codes.len() >= 2
        && (lower.contains("verifies")
            || lower.contains("verified by")
            || lower.contains("tested by")
            || lower.contains("test covers"))
    {
        return (BlockKind::TraceLink, 0.82, Vec::new());
    }
    if codes.len() >= 2
        && (lower.contains("derives from")
            || lower.contains("depends on")
            || lower.contains("relates to")
            || lower.contains("refines")
            || lower.contains("satisfies"))
    {
        return (BlockKind::RequirementLink, 0.8, Vec::new());
    }
    if lower.contains("shall")
        || lower.contains("must")
        || lower.starts_with("requirement")
        || text.starts_with("REQ-")
    {
        return (BlockKind::Requirement, 0.78, Vec::new());
    }
    if lower.contains("test")
        || lower.contains("verification")
        || lower.contains("inspect")
        || lower.contains("demonstration")
        || text.starts_with("TEST-")
        || text.starts_with("VER-")
    {
        return (BlockKind::Verification, 0.7, Vec::new());
    }
    (
        BlockKind::Unknown,
        0.35,
        vec![warning(
            "unknown_block",
            "Could not confidently classify this text block.",
            None,
        )],
    )
}

fn split_normalized_segments(text: &str) -> Vec<String> {
    let numbered_lines = text
        .lines()
        .filter(|line| numbered_prefix(line.trim()))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if numbered_lines.len() > 1 {
        return numbered_lines;
    }

    if text.matches(" shall ").count() > 1 {
        if let Some(reference_marker) = reference_segment_boundary_regex() {
            let starts = reference_marker
                .find_iter(text)
                .map(|match_| match_.start())
                .collect::<Vec<_>>();
            if let Some(segments) = segments_from_starts(text, starts, false) {
                return segments;
            }
        }

        if let Some(shall_marker) = shall_boundary_regex() {
            let starts = shall_marker
                .find_iter(text)
                .map(|match_| match_.start())
                .collect::<Vec<_>>();
            if let Some(segments) = segments_from_starts(text, starts, true) {
                return segments;
            }
        }
    }

    vec![text.to_string()]
}

fn segments_from_starts(
    text: &str,
    mut starts: Vec<usize>,
    attach_prefix_to_first_segment: bool,
) -> Option<Vec<String>> {
    starts.sort_unstable();
    starts.dedup();
    if starts.len() <= 1 {
        return None;
    }

    if attach_prefix_to_first_segment && starts[0] > 0 {
        starts[0] = 0;
    }

    let mut segments = Vec::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(text.len());
        let segment = normalize_whitespace(&text[*start..end]);
        if !segment.is_empty() {
            segments.push(segment);
        }
    }

    if segments.len() > 1 {
        Some(segments)
    } else {
        None
    }
}

fn build_requirement_candidate(
    block: &NormalizedBlock,
    lookups: &ProjectContextData,
    category_hint: Option<i32>,
    applicability_hint: Option<i32>,
) -> RequirementCandidate {
    let reference_code = extract_reference_codes(&block.normalized_text)
        .into_iter()
        .next()
        .unwrap_or_default();
    let (title, description) = split_title_and_body(&block.normalized_text);
    let category_id =
        detect_category_match(&block.normalized_text, &lookups.categories).or(category_hint);
    let applicability_id =
        detect_applicability_match(&block.normalized_text, &lookups.applicability)
            .or(applicability_hint);
    let verification_method_ids =
        detect_verification_methods(&block.normalized_text, &lookups.verification_methods);
    let custom_fields = detect_custom_field_values(&block.normalized_text, &lookups.custom_fields);

    RequirementCandidate {
        id: format!("reqcand-{}", block.id),
        block_id: block.id.clone(),
        include: true,
        title,
        description,
        reference_code,
        reviewer_id: None,
        category_id,
        applicability_id,
        verification_method_ids,
        custom_fields,
        source_refs: block.source_block_ids.clone(),
        lineage_preview: block
            .context
            .clone()
            .unwrap_or_else(|| block.normalized_text.clone()),
        confidence: 0.8,
        issues: Vec::new(),
        duplicate_suggestions: Vec::new(),
    }
}

fn build_verification_candidate(
    block: &NormalizedBlock,
    lookups: &ProjectContextData,
    verification_status_hint: Option<i32>,
    filename: &str,
) -> VerificationCandidate {
    let reference_code = extract_reference_codes(&block.normalized_text)
        .into_iter()
        .next()
        .unwrap_or_default();
    let (name, description) = split_title_and_body(&block.normalized_text);
    let verification_method_id =
        detect_verification_method(&block.normalized_text, &lookups.verification_methods);

    VerificationCandidate {
        id: format!("vercand-{}", block.id),
        block_id: block.id.clone(),
        include: true,
        name,
        description,
        reference_code,
        source: Some(filename.to_string()),
        status_id: verification_status_hint,
        verification_method_id,
        parent_reference_code: None,
        source_refs: block.source_block_ids.clone(),
        lineage_preview: block
            .context
            .clone()
            .unwrap_or_else(|| block.normalized_text.clone()),
        confidence: 0.72,
        issues: Vec::new(),
        duplicate_suggestions: Vec::new(),
    }
}

fn build_trace_link_candidate(block: &NormalizedBlock) -> Option<TraceLinkCandidate> {
    let refs = extract_reference_codes(&block.normalized_text);
    if refs.len() < 2 {
        return None;
    }
    Some(TraceLinkCandidate {
        id: format!("tracelink-{}", block.id),
        block_id: block.id.clone(),
        include: true,
        requirement_reference_code: refs[0].clone(),
        verification_reference_code: refs[1].clone(),
        source_refs: block.source_block_ids.clone(),
        lineage_preview: block
            .context
            .clone()
            .unwrap_or_else(|| block.normalized_text.clone()),
        confidence: 0.75,
        issues: Vec::new(),
    })
}

fn build_requirement_link_candidate(block: &NormalizedBlock) -> Option<RequirementLinkCandidate> {
    let refs = extract_reference_codes(&block.normalized_text);
    if refs.len() < 2 {
        return None;
    }
    let lower = block.normalized_text.to_ascii_lowercase();
    let link_type = if lower.contains("derives from") {
        Some("DERIVES_FROM".to_string())
    } else if lower.contains("depends on") {
        Some("DEPENDS_ON".to_string())
    } else if lower.contains("relates to") {
        Some("RELATES_TO".to_string())
    } else if lower.contains("refines") {
        Some("REFINES".to_string())
    } else if lower.contains("satisfies") {
        Some("SATISFIES".to_string())
    } else {
        None
    };
    Some(RequirementLinkCandidate {
        id: format!("reqlink-{}", block.id),
        block_id: block.id.clone(),
        include: true,
        source_requirement_reference_code: refs[0].clone(),
        target_requirement_reference_code: refs[1].clone(),
        link_type,
        rationale: None,
        source_refs: block.source_block_ids.clone(),
        lineage_preview: block
            .context
            .clone()
            .unwrap_or_else(|| block.normalized_text.clone()),
        confidence: 0.76,
        issues: Vec::new(),
    })
}

fn resolve_requirement_candidates(
    candidates: &mut [RequirementCandidate],
    defaults: &ImportDefaults,
    lookups: &ProjectContextData,
) -> Result<(), DocumentImportError> {
    let draft_status_id = lookups.draft_status_id.ok_or_else(|| {
        DocumentImportError::BadRequest("Project Draft status is required for import.".into())
    })?;
    let existing_refs: HashSet<String> = lookups
        .existing_requirements
        .iter()
        .map(|requirement| requirement.reference_code.to_ascii_uppercase())
        .collect();
    let mut seen_refs = HashMap::<String, String>::new();

    for candidate in candidates.iter_mut() {
        let effective_category = candidate.category_id.or(defaults.category_id);
        let effective_applicability = candidate.applicability_id.or(defaults.applicability_id);
        let effective_reviewer = candidate.reviewer_id.or(defaults.reviewer_id);
        candidate.category_id = effective_category;
        candidate.applicability_id = effective_applicability;
        candidate.reviewer_id = effective_reviewer;

        candidate.issues.clear();
        if !candidate.include {
            continue;
        }
        if candidate.title.trim().is_empty() {
            candidate.issues.push(blocker(
                "missing_title",
                "Requirement title is required.",
                Some(candidate.id.clone()),
            ));
        }
        if candidate.description.trim().is_empty() {
            candidate.issues.push(blocker(
                "missing_description",
                "Requirement statement is required.",
                Some(candidate.id.clone()),
            ));
        }
        if effective_reviewer.unwrap_or_default() <= 0 {
            candidate.issues.push(blocker(
                "missing_reviewer",
                "A reviewer must be selected before import commit.",
                Some(candidate.id.clone()),
            ));
        }
        if effective_category.unwrap_or_default() <= 0 {
            candidate.issues.push(blocker(
                "missing_category",
                "Category is required for imported requirements.",
                Some(candidate.id.clone()),
            ));
        }
        if effective_applicability.unwrap_or_default() <= 0 {
            candidate.issues.push(blocker(
                "missing_applicability",
                "Applicability is required for imported requirements.",
                Some(candidate.id.clone()),
            ));
        }
        let payload = NewRequirement {
            id: None,
            title: candidate.title.clone(),
            description: candidate.description.clone(),
            author_id: 1,
            category_id: effective_category.unwrap_or_default(),
            status_id: draft_status_id,
            reference_code: candidate.reference_code.clone(),
            reviewer_id: effective_reviewer.unwrap_or_default(),
            applicability_id: effective_applicability.unwrap_or_default(),
            justification: None,
            project_id: 1,
        };
        if let Err(err) = validate_requirement(&payload) {
            candidate.issues.push(blocker(
                "invalid_requirement",
                &err.to_string(),
                Some(candidate.id.clone()),
            ));
        }
        if candidate.reference_code.trim().is_empty() {
            candidate.issues.push(warning(
                "blank_reference",
                "Requirement reference is blank; a blank stable code will be imported.",
                Some(candidate.id.clone()),
            ));
        } else {
            let normalized_ref = candidate.reference_code.to_ascii_uppercase();
            if existing_refs.contains(&normalized_ref) {
                candidate.issues.push(blocker(
                    "duplicate_reference",
                    "Requirement reference code already exists in this project.",
                    Some(candidate.id.clone()),
                ));
            }
            if let Some(existing_candidate) = seen_refs.get(&normalized_ref) {
                candidate.issues.push(blocker(
                    "duplicate_import_reference",
                    &format!(
                        "Requirement reference duplicates another selected import candidate ({existing_candidate})."
                    ),
                    Some(candidate.id.clone()),
                ));
            } else {
                seen_refs.insert(normalized_ref, candidate.id.clone());
            }
        }
    }

    Ok(())
}

fn resolve_verification_candidates(
    candidates: &mut [VerificationCandidate],
    defaults: &ImportDefaults,
    lookups: &ProjectContextData,
) {
    let existing_refs: HashSet<String> = lookups
        .existing_verifications
        .iter()
        .map(|verification| verification.reference_code.to_ascii_uppercase())
        .collect();
    let mut seen_refs = HashMap::<String, String>::new();

    for candidate in candidates.iter_mut() {
        candidate.issues.clear();
        if !candidate.include {
            continue;
        }
        if candidate.name.trim().is_empty() {
            candidate.issues.push(blocker(
                "missing_name",
                "Verification name is required.",
                Some(candidate.id.clone()),
            ));
        }
        if candidate.reference_code.trim().is_empty() {
            candidate.issues.push(blocker(
                "missing_reference",
                "Verification reference code is required.",
                Some(candidate.id.clone()),
            ));
        } else {
            let normalized_ref = candidate.reference_code.to_ascii_uppercase();
            if existing_refs.contains(&normalized_ref) {
                candidate.issues.push(blocker(
                    "duplicate_reference",
                    "Verification reference code already exists in this project.",
                    Some(candidate.id.clone()),
                ));
            }
            if let Some(existing_candidate) = seen_refs.get(&normalized_ref) {
                candidate.issues.push(blocker(
                    "duplicate_import_reference",
                    &format!(
                        "Verification reference duplicates another selected import candidate ({existing_candidate})."
                    ),
                    Some(candidate.id.clone()),
                ));
            } else {
                seen_refs.insert(normalized_ref, candidate.id.clone());
            }
        }
        let effective_status = candidate.status_id.or(defaults.verification_status_id);
        candidate.status_id = effective_status;
        if effective_status.unwrap_or_default() <= 0 {
            candidate.issues.push(blocker(
                "missing_status",
                "A verification status must be selected before commit.",
                Some(candidate.id.clone()),
            ));
        }
        let effective_source = candidate
            .source
            .clone()
            .or_else(|| defaults.verification_source.clone())
            .unwrap_or_default();
        candidate.source = Some(effective_source.clone());
        if effective_source.trim().is_empty() {
            candidate.issues.push(blocker(
                "missing_source",
                "Verification source is required.",
                Some(candidate.id.clone()),
            ));
        }
    }
}

fn resolve_trace_links(
    candidates: &mut [TraceLinkCandidate],
    requirement_candidates: &[RequirementCandidate],
    verification_candidates: &[VerificationCandidate],
    lookups: &ProjectContextData,
) {
    let req_refs = requirement_candidates
        .iter()
        .filter(|candidate| candidate.include && !candidate.reference_code.trim().is_empty())
        .map(|candidate| candidate.reference_code.to_ascii_uppercase())
        .collect::<HashSet<_>>();
    let ver_refs = verification_candidates
        .iter()
        .filter(|candidate| candidate.include && !candidate.reference_code.trim().is_empty())
        .map(|candidate| candidate.reference_code.to_ascii_uppercase())
        .collect::<HashSet<_>>();
    let existing_req_refs = lookups
        .existing_requirements
        .iter()
        .map(|requirement| requirement.reference_code.to_ascii_uppercase())
        .collect::<HashSet<_>>();
    let existing_ver_refs = lookups
        .existing_verifications
        .iter()
        .map(|verification| verification.reference_code.to_ascii_uppercase())
        .collect::<HashSet<_>>();

    for candidate in candidates.iter_mut() {
        candidate.issues.clear();
        if !candidate.include {
            continue;
        }
        if candidate.requirement_reference_code.trim().is_empty() {
            candidate.issues.push(blocker(
                "missing_requirement_ref",
                "Trace link requirement reference is required.",
                Some(candidate.id.clone()),
            ));
        } else {
            let key = candidate.requirement_reference_code.to_ascii_uppercase();
            if !req_refs.contains(&key) && !existing_req_refs.contains(&key) {
                candidate.issues.push(blocker(
                    "unresolved_requirement_ref",
                    "Trace link requirement reference could not be resolved.",
                    Some(candidate.id.clone()),
                ));
            }
        }
        if candidate.verification_reference_code.trim().is_empty() {
            candidate.issues.push(blocker(
                "missing_verification_ref",
                "Trace link verification reference is required.",
                Some(candidate.id.clone()),
            ));
        } else {
            let key = candidate.verification_reference_code.to_ascii_uppercase();
            if !ver_refs.contains(&key) && !existing_ver_refs.contains(&key) {
                candidate.issues.push(blocker(
                    "unresolved_verification_ref",
                    "Trace link verification reference could not be resolved.",
                    Some(candidate.id.clone()),
                ));
            }
        }
    }
}

fn resolve_requirement_links(
    candidates: &mut [RequirementLinkCandidate],
    requirement_candidates: &[RequirementCandidate],
    lookups: &ProjectContextData,
) {
    let requirement_refs = requirement_candidates
        .iter()
        .filter(|candidate| candidate.include && !candidate.reference_code.trim().is_empty())
        .map(|candidate| candidate.reference_code.to_ascii_uppercase())
        .collect::<HashSet<_>>();
    let existing_refs = lookups
        .existing_requirements
        .iter()
        .map(|requirement| requirement.reference_code.to_ascii_uppercase())
        .collect::<HashSet<_>>();

    for candidate in candidates.iter_mut() {
        candidate.issues.clear();
        if !candidate.include {
            continue;
        }
        if candidate
            .source_requirement_reference_code
            .trim()
            .is_empty()
        {
            candidate.issues.push(blocker(
                "missing_source_ref",
                "Requirement link source reference is required.",
                Some(candidate.id.clone()),
            ));
        } else {
            let key = candidate
                .source_requirement_reference_code
                .to_ascii_uppercase();
            if !requirement_refs.contains(&key) && !existing_refs.contains(&key) {
                candidate.issues.push(blocker(
                    "unresolved_source_ref",
                    "Requirement link source reference could not be resolved.",
                    Some(candidate.id.clone()),
                ));
            }
        }
        if candidate
            .target_requirement_reference_code
            .trim()
            .is_empty()
        {
            candidate.issues.push(blocker(
                "missing_target_ref",
                "Requirement link target reference is required.",
                Some(candidate.id.clone()),
            ));
        } else {
            let key = candidate
                .target_requirement_reference_code
                .to_ascii_uppercase();
            if !requirement_refs.contains(&key) && !existing_refs.contains(&key) {
                candidate.issues.push(blocker(
                    "unresolved_target_ref",
                    "Requirement link target reference could not be resolved.",
                    Some(candidate.id.clone()),
                ));
            }
        }
        if candidate
            .link_type
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            candidate.issues.push(blocker(
                "missing_link_type",
                "Requirement link type is required.",
                Some(candidate.id.clone()),
            ));
        }
    }
}

fn add_similarity_suggestions(
    requirement_candidates: &mut [RequirementCandidate],
    verification_candidates: &mut [VerificationCandidate],
    lookups: &ProjectContextData,
) {
    for candidate in requirement_candidates.iter_mut() {
        candidate
            .duplicate_suggestions
            .extend(best_requirement_matches(
                candidate,
                &lookups.existing_requirements,
            ));
    }
    for candidate in verification_candidates.iter_mut() {
        candidate
            .duplicate_suggestions
            .extend(best_verification_matches(
                candidate,
                &lookups.existing_verifications,
            ));
    }
}

fn build_import_plan(candidates: &ImportCandidates, lookups: &ProjectContextData) -> ImportPlan {
    let draft_status_id = lookups.draft_status_id;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut requirements_to_create = Vec::new();
    let mut verifications_to_create = Vec::new();
    let mut trace_links_to_create = Vec::new();
    let mut requirement_links_to_create = Vec::new();

    if draft_status_id.is_none() {
        blockers.push(blocker(
            "missing_draft_status",
            "Project Draft status is required for document import.",
            None,
        ));
    }

    for candidate in &candidates.requirements {
        collect_issues(&mut blockers, &mut warnings, &candidate.issues);
        if candidate.include && !candidate_has_blocker(&candidate.issues) {
            requirements_to_create.push(PlannedRequirementCreate {
                candidate_id: candidate.id.clone(),
                title: candidate.title.clone(),
                description: candidate.description.clone(),
                reference_code: candidate.reference_code.clone(),
                reviewer_id: candidate.reviewer_id.unwrap_or_default(),
                category_id: candidate.category_id.unwrap_or_default(),
                applicability_id: candidate.applicability_id.unwrap_or_default(),
                verification_method_ids: candidate.verification_method_ids.clone(),
                custom_fields: candidate.custom_fields.clone(),
            });
        }
    }

    for candidate in &candidates.verifications {
        collect_issues(&mut blockers, &mut warnings, &candidate.issues);
        if candidate.include && !candidate_has_blocker(&candidate.issues) {
            verifications_to_create.push(PlannedVerificationCreate {
                candidate_id: candidate.id.clone(),
                name: candidate.name.clone(),
                description: candidate.description.clone(),
                reference_code: candidate.reference_code.clone(),
                source: candidate.source.clone().unwrap_or_default(),
                status_id: candidate.status_id.unwrap_or_default(),
                verification_method_id: candidate.verification_method_id,
            });
        }
    }

    for candidate in &candidates.trace_links {
        collect_issues(&mut blockers, &mut warnings, &candidate.issues);
        if candidate.include && !candidate_has_blocker(&candidate.issues) {
            trace_links_to_create.push(PlannedTraceLinkCreate {
                candidate_id: candidate.id.clone(),
                requirement_reference_code: candidate.requirement_reference_code.clone(),
                verification_reference_code: candidate.verification_reference_code.clone(),
            });
        }
    }

    for candidate in &candidates.requirement_links {
        collect_issues(&mut blockers, &mut warnings, &candidate.issues);
        if candidate.include && !candidate_has_blocker(&candidate.issues) {
            requirement_links_to_create.push(PlannedRequirementLinkCreate {
                candidate_id: candidate.id.clone(),
                source_requirement_reference_code: candidate
                    .source_requirement_reference_code
                    .clone(),
                target_requirement_reference_code: candidate
                    .target_requirement_reference_code
                    .clone(),
                link_type: candidate.link_type.clone().unwrap_or_default(),
                rationale: candidate.rationale.clone(),
            });
        }
    }

    let ready_to_commit = blockers.is_empty();
    ImportPlan {
        requirements_to_create,
        verifications_to_create,
        trace_links_to_create,
        requirement_links_to_create,
        blockers,
        warnings,
        ready_to_commit,
    }
}

fn build_diagnostics(
    candidates: &ImportCandidates,
    plan_blockers: &[ImportIssue],
    plan_warnings: &[ImportIssue],
) -> ImportDiagnostics {
    let mut diagnostics = ImportDiagnostics {
        blockers: plan_blockers.to_vec(),
        warnings: plan_warnings.to_vec(),
        infos: Vec::new(),
    };
    for candidate in &candidates.requirements {
        append_issue_buckets(&mut diagnostics, &candidate.issues);
    }
    for candidate in &candidates.verifications {
        append_issue_buckets(&mut diagnostics, &candidate.issues);
    }
    for candidate in &candidates.trace_links {
        append_issue_buckets(&mut diagnostics, &candidate.issues);
    }
    for candidate in &candidates.requirement_links {
        append_issue_buckets(&mut diagnostics, &candidate.issues);
    }
    dedupe_issue_bucket(&mut diagnostics.blockers);
    dedupe_issue_bucket(&mut diagnostics.warnings);
    dedupe_issue_bucket(&mut diagnostics.infos);
    diagnostics
}

fn build_summary(
    candidates: &ImportCandidates,
    diagnostics: &ImportDiagnostics,
) -> ImportSessionSummary {
    ImportSessionSummary {
        requirement_candidates: candidates.requirements.len(),
        verification_candidates: candidates.verifications.len(),
        trace_link_candidates: candidates.trace_links.len(),
        requirement_link_candidates: candidates.requirement_links.len(),
        included_requirements: candidates
            .requirements
            .iter()
            .filter(|candidate| candidate.include)
            .count(),
        included_verifications: candidates
            .verifications
            .iter()
            .filter(|candidate| candidate.include)
            .count(),
        blockers: diagnostics.blockers.len(),
        warnings: diagnostics.warnings.len(),
        ready_to_commit: diagnostics.blockers.is_empty(),
    }
}

fn candidate_has_blocker(issues: &[ImportIssue]) -> bool {
    issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Blocker)
}

fn collect_issues(
    blockers: &mut Vec<ImportIssue>,
    warnings: &mut Vec<ImportIssue>,
    issues: &[ImportIssue],
) {
    for issue in issues {
        match issue.severity {
            IssueSeverity::Blocker => blockers.push(issue.clone()),
            IssueSeverity::Warning => warnings.push(issue.clone()),
            IssueSeverity::Info => {}
        }
    }
}

fn append_issue_buckets(diagnostics: &mut ImportDiagnostics, issues: &[ImportIssue]) {
    for issue in issues {
        match issue.severity {
            IssueSeverity::Blocker => diagnostics.blockers.push(issue.clone()),
            IssueSeverity::Warning => diagnostics.warnings.push(issue.clone()),
            IssueSeverity::Info => diagnostics.infos.push(issue.clone()),
        }
    }
}

fn dedupe_issue_bucket(issues: &mut Vec<ImportIssue>) {
    let mut seen = HashSet::new();
    issues.retain(|issue| {
        let key = format!(
            "{:?}:{}:{}:{}",
            issue.severity,
            issue.code,
            issue.message,
            issue.candidate_id.clone().unwrap_or_default()
        );
        seen.insert(key)
    });
}

fn apply_requirement_patch(candidate: &mut RequirementCandidate, patch: RequirementReviewPatch) {
    if let Some(include) = patch.include {
        candidate.include = include;
    }
    if let Some(title) = patch.title {
        candidate.title = normalize_whitespace(&title);
    }
    if let Some(description) = patch.description {
        candidate.description = normalize_whitespace(&description);
    }
    if let Some(reference_code) = patch.reference_code {
        candidate.reference_code = reference_code.trim().to_string();
    }
    if let Some(reviewer_id) = patch.reviewer_id {
        candidate.reviewer_id = positive_id_option(reviewer_id);
    }
    if let Some(category_id) = patch.category_id {
        candidate.category_id = positive_id_option(category_id);
    }
    if let Some(applicability_id) = patch.applicability_id {
        candidate.applicability_id = positive_id_option(applicability_id);
    }
    if let Some(verification_method_ids) = patch.verification_method_ids {
        candidate.verification_method_ids = verification_method_ids
            .into_iter()
            .filter(|id| *id > 0)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        candidate.verification_method_ids.sort_unstable();
    }
    if let Some(custom_fields) = patch.custom_fields {
        candidate.custom_fields = custom_fields;
    }
}

fn apply_verification_patch(candidate: &mut VerificationCandidate, patch: VerificationReviewPatch) {
    if let Some(include) = patch.include {
        candidate.include = include;
    }
    if let Some(name) = patch.name {
        candidate.name = normalize_whitespace(&name);
    }
    if let Some(description) = patch.description {
        candidate.description = normalize_whitespace(&description);
    }
    if let Some(reference_code) = patch.reference_code {
        candidate.reference_code = reference_code.trim().to_string();
    }
    if let Some(source) = patch.source {
        candidate.source = Some(normalize_whitespace(&source));
    }
    if let Some(status_id) = patch.status_id {
        candidate.status_id = positive_id_option(status_id);
    }
    if let Some(verification_method_id) = patch.verification_method_id {
        candidate.verification_method_id = positive_id_option(verification_method_id);
    }
}

fn apply_trace_link_patch(candidate: &mut TraceLinkCandidate, patch: TraceLinkReviewPatch) {
    if let Some(include) = patch.include {
        candidate.include = include;
    }
    if let Some(reference) = patch.requirement_reference_code {
        candidate.requirement_reference_code = reference.trim().to_string();
    }
    if let Some(reference) = patch.verification_reference_code {
        candidate.verification_reference_code = reference.trim().to_string();
    }
}

fn apply_requirement_link_patch(
    candidate: &mut RequirementLinkCandidate,
    patch: RequirementLinkReviewPatch,
) {
    if let Some(include) = patch.include {
        candidate.include = include;
    }
    if let Some(reference) = patch.source_requirement_reference_code {
        candidate.source_requirement_reference_code = reference.trim().to_string();
    }
    if let Some(reference) = patch.target_requirement_reference_code {
        candidate.target_requirement_reference_code = reference.trim().to_string();
    }
    if let Some(link_type) = patch.link_type {
        candidate.link_type = non_empty_string(link_type);
    }
    if let Some(rationale) = patch.rationale {
        candidate.rationale = non_empty_string(rationale);
    }
}

fn extract_reference_codes(text: &str) -> Vec<String> {
    let Some(regex) = reference_code_regex() else {
        return Vec::new();
    };
    regex
        .find_iter(text)
        .map(|capture| capture.as_str().to_string())
        .collect()
}

fn split_title_and_body(text: &str) -> (String, String) {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if let Some((first, rest)) = lines.split_first() {
        let title = strip_leading_reference(first);
        if rest.is_empty() {
            let sentence = title
                .split('.')
                .next()
                .map(str::trim)
                .unwrap_or(title.as_str());
            let short_title = if sentence.len() > 80 {
                sentence.chars().take(80).collect::<String>()
            } else {
                sentence.to_string()
            };
            return (short_title, text.to_string());
        }
        return (title, text.to_string());
    }
    ("Imported item".to_string(), text.to_string())
}

fn strip_leading_reference(line: &str) -> String {
    let Some(regex) = leading_reference_regex() else {
        return line.trim().to_string();
    };
    regex.replace(line, "").trim().to_string()
}

fn reference_segment_boundary_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+\s*:?\s*").ok())
        .as_ref()
}

fn shall_boundary_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"(?i)The\s+\w+(?:\s+\w+){0,3}\s+shall\b").ok())
        .as_ref()
}

fn reference_code_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"\b[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+\b").ok())
        .as_ref()
}

fn leading_reference_regex() -> Option<&'static Regex> {
    static REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"^(?:[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+\s*[:\-]\s*)").ok())
        .as_ref()
}

fn detect_category_match(text: &str, categories: &[Category]) -> Option<i32> {
    let lower = text.to_ascii_lowercase();
    categories.iter().find_map(|category| {
        let title = category.title.to_ascii_lowercase();
        let tag = category.tag.to_ascii_lowercase();
        if lower.contains(&title) || (!tag.is_empty() && lower.contains(&tag)) {
            Some(category.id)
        } else {
            None
        }
    })
}

fn detect_applicability_match(
    text: &str,
    applicability: &[crate::models::Applicability],
) -> Option<i32> {
    let lower = text.to_ascii_lowercase();
    applicability.iter().find_map(|item| {
        let title = item.title.to_ascii_lowercase();
        let tag = item.tag.to_ascii_lowercase();
        if lower.contains(&title) || (!tag.is_empty() && lower.contains(&tag)) {
            Some(item.id)
        } else {
            None
        }
    })
}

fn detect_verification_methods(text: &str, methods: &[VerificationMethod]) -> Vec<i32> {
    let lower = text.to_ascii_lowercase();
    methods
        .iter()
        .filter(|method| {
            let title = method.title.to_ascii_lowercase();
            let tag = method.tag.to_ascii_lowercase();
            lower.contains(&title) || (!tag.is_empty() && lower.contains(&tag))
        })
        .map(|method| method.id)
        .collect()
}

fn detect_verification_method(text: &str, methods: &[VerificationMethod]) -> Option<i32> {
    detect_verification_methods(text, methods)
        .into_iter()
        .next()
}

fn detect_custom_field_values(
    text: &str,
    fields: &[CustomFieldDefinition],
) -> Vec<CustomFieldValueInput> {
    let lower = text.to_ascii_lowercase();
    fields
        .iter()
        .filter_map(|field| {
            let needle = format!("{}:", field.label.to_ascii_lowercase());
            let start = lower.find(&needle)?;
            let remainder = &text[start + needle.len()..];
            let value = remainder
                .lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToString::to_string);
            Some(CustomFieldValueInput {
                field_id: field.id,
                value,
            })
        })
        .collect()
}

fn best_requirement_matches(
    candidate: &RequirementCandidate,
    requirements: &[Requirement],
) -> Vec<DuplicateSuggestion> {
    let mut matches = requirements
        .iter()
        .filter_map(|existing| {
            let score = word_overlap_score(&candidate.title, &existing.title);
            if score < 0.45 {
                return None;
            }
            Some(DuplicateSuggestion {
                match_kind: "fuzzy".to_string(),
                existing_id: existing.id,
                reference_code: existing.reference_code.clone(),
                title: existing.title.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| b.score.total_cmp(&a.score));
    matches.truncate(3);
    matches
}

fn best_verification_matches(
    candidate: &VerificationCandidate,
    verifications: &[Verification],
) -> Vec<DuplicateSuggestion> {
    let mut matches = verifications
        .iter()
        .filter_map(|existing| {
            let score = word_overlap_score(&candidate.name, &existing.name);
            if score < 0.45 {
                return None;
            }
            Some(DuplicateSuggestion {
                match_kind: "fuzzy".to_string(),
                existing_id: existing.id,
                reference_code: existing.reference_code.clone(),
                title: existing.name.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| b.score.total_cmp(&a.score));
    matches.truncate(3);
    matches
}

fn word_overlap_score(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }
    let intersection = left_tokens.intersection(&right_tokens).count() as f32;
    let union = left_tokens.union(&right_tokens).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn tokenize(input: &str) -> HashSet<String> {
    input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn dedupe_requirements(items: &[PlannedRequirementCreate]) -> Vec<PlannedRequirementCreate> {
    let mut seen = HashSet::new();
    items
        .iter()
        .filter(|item| {
            seen.insert(format!(
                "{}:{}",
                item.reference_code.to_ascii_uppercase(),
                item.candidate_id
            ))
        })
        .cloned()
        .collect()
}

fn dedupe_verifications(items: &[PlannedVerificationCreate]) -> Vec<PlannedVerificationCreate> {
    let mut seen = HashSet::new();
    items
        .iter()
        .filter(|item| {
            seen.insert(format!(
                "{}:{}",
                item.reference_code.to_ascii_uppercase(),
                item.candidate_id
            ))
        })
        .cloned()
        .collect()
}

fn dedupe_trace_links(items: &[PlannedTraceLinkCreate]) -> Vec<PlannedTraceLinkCreate> {
    let mut seen = HashSet::new();
    items
        .iter()
        .filter(|item| {
            seen.insert(format!(
                "{}:{}",
                item.requirement_reference_code.to_ascii_uppercase(),
                item.verification_reference_code.to_ascii_uppercase()
            ))
        })
        .cloned()
        .collect()
}

fn dedupe_requirement_links(
    items: &[PlannedRequirementLinkCreate],
) -> Vec<PlannedRequirementLinkCreate> {
    let mut seen = HashSet::new();
    items
        .iter()
        .filter(|item| {
            seen.insert(format!(
                "{}:{}:{}",
                item.source_requirement_reference_code.to_ascii_uppercase(),
                item.target_requirement_reference_code.to_ascii_uppercase(),
                item.link_type
            ))
        })
        .cloned()
        .collect()
}

fn extract_ai_suggestions(response: &str) -> Result<Vec<AiSuggestion>, serde_json::Error> {
    let trimmed = response.trim();
    if let Ok(suggestions) = serde_json::from_str::<Vec<AiSuggestion>>(trimmed) {
        return Ok(suggestions);
    }
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        serde_json::from_str::<Vec<AiSuggestion>>(&trimmed[start..=end])
    } else {
        serde_json::from_str::<Vec<AiSuggestion>>(trimmed)
    }
}

fn session_cache_key(project_id: i32, user_id: i32, session_id: &str) -> String {
    format!("document_import:{project_id}:{user_id}:{session_id}")
}

fn make_session_id(project_id: i32, user_id: i32, filename: &str) -> String {
    let hash = sha2::Sha256::digest(
        format!(
            "{project_id}:{user_id}:{filename}:{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
        .as_bytes(),
    );
    format!("{:x}", hash)[..16].to_string()
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn numbered_prefix(line: &str) -> bool {
    let mut chars = line.chars();
    let mut saw_digit = false;
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        return saw_digit && (ch == '.' || ch == ')');
    }
    false
}

fn positive_id_option(value: i32) -> Option<i32> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

fn non_empty_string(value: String) -> Option<String> {
    let value = normalize_whitespace(&value);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn blocker(code: &str, message: &str, candidate_id: Option<String>) -> ImportIssue {
    ImportIssue {
        severity: IssueSeverity::Blocker,
        code: code.to_string(),
        message: message.to_string(),
        candidate_id,
    }
}

fn warning(code: &str, message: &str, candidate_id: Option<String>) -> ImportIssue {
    ImportIssue {
        severity: IssueSeverity::Warning,
        code: code.to_string(),
        message: message.to_string(),
        candidate_id,
    }
}

fn info(code: &str, message: &str, candidate_id: Option<String>) -> ImportIssue {
    ImportIssue {
        severity: IssueSeverity::Info,
        code: code.to_string(),
        message: message.to_string(),
        candidate_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use std::path::PathBuf;
    use std::sync::{Arc, RwLock};

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn sample_user() -> User {
        DieselRepoMock::make_user(7, "importer", "")
    }

    fn fixture_bytes(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/document_import")
            .join(name);
        std::fs::read(path).expect("fixture bytes")
    }

    fn project_context_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.projects.insert(
            1,
            crate::models::Project {
                id: 1,
                name: "Project".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: crate::status_enums::ProjectStatus::Active,
                owner_id: Some(1),
            },
        );
        repo.project_members.push(crate::models::ProjectMember {
            project_id: 1,
            user_id: 7,
            role: crate::permissions::ROLE_AUTHOR,
            created_at: chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            updated_at: chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        });
        repo.users.insert(7, sample_user());
        repo.requirement_statuses.insert(
            1,
            crate::models::RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "draft".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        repo.verification_statuses.insert(
            2,
            VerificationStatus {
                id: 2,
                title: "Pending".into(),
                description: "".into(),
                tag: "pending".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Safety".into(),
                description: "".into(),
                tag: "SAFETY".into(),
                project_id: 1,
            },
        );
        repo.applicability.insert(
            1,
            crate::models::Applicability {
                id: 1,
                title: "All".into(),
                description: "".into(),
                tag: "ALL".into(),
                project_id: 1,
            },
        );
        repo
    }

    #[test]
    fn normalize_whitespace_collapses_spaces() {
        assert_eq!(normalize_whitespace("a   b\n c"), "a b c");
    }

    #[test]
    fn guess_block_kind_detects_requirement() {
        let (kind, _, _) = guess_block_kind("REQ-001 The system shall log events.");
        assert_eq!(kind, BlockKind::Requirement);
    }

    #[test]
    fn split_normalized_segments_splits_multiple_requirement_statements() {
        let segments = split_normalized_segments(
            "REQ-001: The system shall log faults. REQ-002: The system shall raise an alert.",
        );
        assert_eq!(segments.len(), 2);
        assert!(segments[0].contains("REQ-001"));
        assert!(segments[1].contains("REQ-002"));
    }

    #[test]
    fn build_requirement_candidate_detects_custom_field_values() {
        let mut repo = project_context_repo();
        repo.custom_field_definitions.insert(
            5,
            CustomFieldDefinition {
                id: 5,
                project_id: 1,
                label: "Safety Integrity".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: 1,
                created_at: chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            },
        );
        let state = state_with_repo(repo);
        let service = DocumentImportService::new(&state);
        let lookups = service.load_project_context(1).unwrap();
        let candidate = build_requirement_candidate(
            &NormalizedBlock {
                id: "n1".into(),
                normalized_text: "REQ-001 The system shall stop. Safety Integrity: SIL2".into(),
                source_block_ids: vec!["block-1".into()],
                final_kind: BlockKind::Requirement,
                context: None,
                issues: vec![],
            },
            &lookups,
            Some(1),
            Some(1),
        );
        assert_eq!(candidate.custom_fields.len(), 1);
        assert_eq!(candidate.custom_fields[0].field_id, 5);
        assert_eq!(candidate.custom_fields[0].value.as_deref(), Some("SIL2"));
    }

    #[test]
    fn extract_docx_fragments_reads_fixture() {
        let fragments = extract_docx_fragments(&fixture_bytes("sample-import.docx")).unwrap();
        assert_eq!(fragments.len(), 3);
        assert!(fragments[0].text.contains("REQ-001"));
    }

    #[test]
    fn extract_pdf_fragments_reads_fixture() {
        let fragments = extract_pdf_fragments(&fixture_bytes("sample-import.pdf")).unwrap();
        assert_eq!(fragments.len(), 1);
        assert!(fragments[0].text.contains("REQ-001"));
        assert!(fragments[0].text.contains("TEST-1"));
    }

    #[test]
    fn detect_document_format_sniffs_docx_when_filename_is_blank() {
        let format = detect_document_format("", &fixture_bytes("sample-import.docx")).unwrap();
        assert_eq!(format, ImportedDocumentFormat::Docx);
    }

    #[test]
    fn detect_document_format_sniffs_pdf_when_filename_is_blank() {
        let format = detect_document_format("", &fixture_bytes("sample-import.pdf")).unwrap();
        assert_eq!(format, ImportedDocumentFormat::Pdf);
    }

    #[test]
    fn build_import_plan_blocks_missing_reviewer() {
        let lookups = ProjectContextData {
            draft_status_id: Some(1),
            pending_verification_status_id: Some(2),
            users: vec![],
            categories: vec![],
            applicability: vec![],
            verification_methods: vec![],
            verification_statuses: vec![],
            custom_fields: vec![],
            existing_requirements: vec![],
            existing_verifications: vec![],
        };
        let mut requirements = vec![RequirementCandidate {
            id: "req-1".into(),
            block_id: "b1".into(),
            include: true,
            title: "Valid title".into(),
            description: "Valid statement".into(),
            reference_code: "REQ-001".into(),
            reviewer_id: None,
            category_id: Some(1),
            applicability_id: Some(1),
            verification_method_ids: vec![],
            custom_fields: vec![],
            source_refs: vec![],
            lineage_preview: "".into(),
            confidence: 0.8,
            issues: vec![],
            duplicate_suggestions: vec![],
        }];
        resolve_requirement_candidates(
            &mut requirements,
            &ImportDefaults::default(),
            &ProjectContextData {
                categories: vec![Category {
                    id: 1,
                    title: "Safety".into(),
                    description: "".into(),
                    tag: "SAFETY".into(),
                    project_id: 1,
                }],
                applicability: vec![crate::models::Applicability {
                    id: 1,
                    title: "All".into(),
                    description: "".into(),
                    tag: "ALL".into(),
                    project_id: 1,
                }],
                ..lookups
            },
        )
        .unwrap();
        let plan = build_import_plan(
            &ImportCandidates {
                requirements,
                verifications: vec![],
                trace_links: vec![],
                requirement_links: vec![],
            },
            &ProjectContextData {
                draft_status_id: Some(1),
                pending_verification_status_id: Some(2),
                users: vec![],
                categories: vec![],
                applicability: vec![],
                verification_methods: vec![],
                verification_statuses: vec![],
                custom_fields: vec![],
                existing_requirements: vec![],
                existing_verifications: vec![],
            },
        );
        assert!(!plan.ready_to_commit);
        assert!(!plan.blockers.is_empty());
    }

    #[test]
    fn resolve_verification_candidates_blocks_existing_reference() {
        let mut candidates = vec![VerificationCandidate {
            id: "ver-1".into(),
            block_id: "b1".into(),
            include: true,
            name: "Imported verification".into(),
            description: "Verification description".into(),
            reference_code: "TEST-1".into(),
            source: Some("sample.docx".into()),
            status_id: Some(2),
            verification_method_id: None,
            parent_reference_code: None,
            source_refs: vec![],
            lineage_preview: String::new(),
            confidence: 0.9,
            issues: vec![],
            duplicate_suggestions: vec![],
        }];
        let lookups = ProjectContextData {
            draft_status_id: Some(1),
            pending_verification_status_id: Some(2),
            users: vec![],
            categories: vec![],
            applicability: vec![],
            verification_methods: vec![],
            verification_statuses: vec![],
            custom_fields: vec![],
            existing_requirements: vec![],
            existing_verifications: vec![Verification {
                id: 22,
                name: "Existing verification".into(),
                reference_code: "TEST-1".into(),
                description: "Already present".into(),
                source: "seed".into(),
                status_id: 2,
                parent_id: None,
                project_id: 1,
                verification_method_id: None,
            }],
        };

        resolve_verification_candidates(&mut candidates, &ImportDefaults::default(), &lookups);

        assert!(candidates[0]
            .issues
            .iter()
            .any(|issue| issue.code == "duplicate_reference"));
    }
}
