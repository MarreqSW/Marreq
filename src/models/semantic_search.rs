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

    #[test]
    fn embedding_index_status_empty_returns_none() {
        assert_eq!(EmbeddingIndexStatus::from_str(""), None);
    }

    #[test]
    fn embedding_index_status_case_sensitive() {
        // Statuses are case-sensitive
        assert_eq!(EmbeddingIndexStatus::from_str("PENDING"), None);
        assert_eq!(EmbeddingIndexStatus::from_str("Pending"), None);
        assert_eq!(
            EmbeddingIndexStatus::from_str("pending"),
            Some(EmbeddingIndexStatus::Pending)
        );
    }

    #[test]
    fn embedding_index_status_as_str_values() {
        assert_eq!(EmbeddingIndexStatus::Pending.as_str(), "pending");
        assert_eq!(EmbeddingIndexStatus::Processing.as_str(), "processing");
        assert_eq!(EmbeddingIndexStatus::Completed.as_str(), "completed");
        assert_eq!(EmbeddingIndexStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn embedding_index_status_clone() {
        let status = EmbeddingIndexStatus::Processing;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn embedding_index_status_copy() {
        let status = EmbeddingIndexStatus::Completed;
        let copied: EmbeddingIndexStatus = status; // Copy, not move
        assert_eq!(status, copied);
    }

    #[test]
    fn embedding_index_status_debug() {
        let debug = format!("{:?}", EmbeddingIndexStatus::Failed);
        assert!(debug.contains("Failed"));
    }

    #[test]
    fn semantic_search_result_creation() {
        let result = SemanticSearchResult {
            id: 1,
            reference_code: "REQ-001".into(),
            title: "Test Requirement".into(),
            description: "Full description here".into(),
            snippet: "Full description...".into(),
            score: 0.95,
            rank: 1,
            lexical_rank: Some(2),
            vector_rank: Some(1),
            status: "Active".into(),
            category: "Functional".into(),
            applicability: "All Variants".into(),
            verification: "Test".into(),
        };

        assert_eq!(result.id, 1);
        assert_eq!(result.reference_code, "REQ-001");
        assert_eq!(result.title, "Test Requirement");
        assert!((result.score - 0.95).abs() < f32::EPSILON);
        assert_eq!(result.rank, 1);
        assert_eq!(result.lexical_rank, Some(2));
        assert_eq!(result.vector_rank, Some(1));
    }

    #[test]
    fn semantic_search_result_no_ranks() {
        let result = SemanticSearchResult {
            id: 1,
            reference_code: "REQ-001".into(),
            title: "Test".into(),
            description: "Desc".into(),
            snippet: "Snip".into(),
            score: 0.5,
            rank: 1,
            lexical_rank: None,
            vector_rank: None,
            status: "Draft".into(),
            category: "Cat".into(),
            applicability: "App".into(),
            verification: "Ver".into(),
        };

        assert!(result.lexical_rank.is_none());
        assert!(result.vector_rank.is_none());
    }

    #[test]
    fn semantic_search_result_clone() {
        let result = SemanticSearchResult {
            id: 42,
            reference_code: "REQ-042".into(),
            title: "Clone Test".into(),
            description: "Description".into(),
            snippet: "Snippet".into(),
            score: 0.75,
            rank: 3,
            lexical_rank: Some(5),
            vector_rank: Some(2),
            status: "Status".into(),
            category: "Category".into(),
            applicability: "Applicability".into(),
            verification: "Verification".into(),
        };

        let cloned = result.clone();
        assert_eq!(result.id, cloned.id);
        assert_eq!(result.reference_code, cloned.reference_code);
        assert_eq!(result.score, cloned.score);
    }

    #[test]
    fn project_index_status_creation() {
        let status = ProjectIndexStatus {
            project_id: 5,
            total_requirements: 100,
            indexed_count: 95,
            pending_count: 3,
            failed_count: 2,
            last_indexed_at: None,
            embeddings_enabled: true,
            embedding_model: "nomic-embed-text".into(),
        };

        assert_eq!(status.project_id, 5);
        assert_eq!(status.total_requirements, 100);
        assert_eq!(status.indexed_count, 95);
        assert_eq!(status.pending_count, 3);
        assert_eq!(status.failed_count, 2);
        assert!(status.embeddings_enabled);
    }

    #[test]
    fn project_index_status_with_timestamp() {
        use chrono::{NaiveDate, Timelike};
        let ts = NaiveDate::from_ymd_opt(2025, 1, 30)
            .unwrap()
            .and_hms_opt(10, 30, 0)
            .unwrap();

        let status = ProjectIndexStatus {
            project_id: 1,
            total_requirements: 50,
            indexed_count: 50,
            pending_count: 0,
            failed_count: 0,
            last_indexed_at: Some(ts),
            embeddings_enabled: true,
            embedding_model: "model".into(),
        };

        assert!(status.last_indexed_at.is_some());
        assert_eq!(status.last_indexed_at.unwrap().hour(), 10);
    }

    #[test]
    fn rag_answer_response_creation() {
        let response = RagAnswerResponse {
            answer: "This is the answer based on requirements.".into(),
            citations: vec![RagCitation {
                requirement_id: 1,
                reference_code: "REQ-001".into(),
                title: "First".into(),
            }],
            results: vec![],
        };

        assert!(!response.answer.is_empty());
        assert_eq!(response.citations.len(), 1);
        assert_eq!(response.results.len(), 0);
    }

    #[test]
    fn rag_answer_response_empty() {
        let response = RagAnswerResponse {
            answer: "No results found.".into(),
            citations: vec![],
            results: vec![],
        };

        assert!(response.citations.is_empty());
        assert!(response.results.is_empty());
    }

    #[test]
    fn rag_answer_response_with_results() {
        let result = SemanticSearchResult {
            id: 1,
            reference_code: "REQ-001".into(),
            title: "Test".into(),
            description: "Desc".into(),
            snippet: "Snip".into(),
            score: 0.9,
            rank: 1,
            lexical_rank: None,
            vector_rank: None,
            status: "Active".into(),
            category: "Cat".into(),
            applicability: "App".into(),
            verification: "Ver".into(),
        };

        let response = RagAnswerResponse {
            answer: "Answer with context".into(),
            citations: vec![RagCitation {
                requirement_id: 1,
                reference_code: "REQ-001".into(),
                title: "Test".into(),
            }],
            results: vec![result],
        };

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.citations.len(), 1);
        assert_eq!(response.results[0].id, response.citations[0].requirement_id);
    }

    #[test]
    fn rag_citation_creation() {
        let citation = RagCitation {
            requirement_id: 123,
            reference_code: "SYS-PERF-001".into(),
            title: "Performance Requirement".into(),
        };

        assert_eq!(citation.requirement_id, 123);
        assert_eq!(citation.reference_code, "SYS-PERF-001");
        assert_eq!(citation.title, "Performance Requirement");
    }

    #[test]
    fn rag_citation_clone() {
        let citation = RagCitation {
            requirement_id: 1,
            reference_code: "REQ-001".into(),
            title: "Title".into(),
        };

        let cloned = citation.clone();
        assert_eq!(citation.requirement_id, cloned.requirement_id);
        assert_eq!(citation.reference_code, cloned.reference_code);
        assert_eq!(citation.title, cloned.title);
    }

    #[test]
    fn new_embedding_index_queue_entry() {
        let entry = NewEmbeddingIndexQueueEntry {
            requirement_id: 42,
            project_id: 1,
            status: "pending".into(),
        };

        assert_eq!(entry.requirement_id, 42);
        assert_eq!(entry.project_id, 1);
        assert_eq!(entry.status, "pending");
    }

    #[test]
    fn new_requirement_embedding_creation() {
        use chrono::Utc;

        let now = Utc::now().naive_utc();
        let embedding = NewRequirementEmbedding {
            requirement_id: 1,
            project_id: 1,
            embedding: None,
            embedding_model: "nomic-embed-text".into(),
            content_hash: "abc123".into(),
            updated_at: now,
        };

        assert_eq!(embedding.requirement_id, 1);
        assert_eq!(embedding.project_id, 1);
        assert!(embedding.embedding.is_none());
        assert_eq!(embedding.embedding_model, "nomic-embed-text");
        assert_eq!(embedding.content_hash, "abc123");
    }
}
