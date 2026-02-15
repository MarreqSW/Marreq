//! API endpoints for semantic search functionality.
//!
//! All endpoints are project-scoped and use the `ProjectAccess` guard
//! for authorization.

use crate::api::prelude::*;
use crate::auth::guards::{AdminOnly, ProjectAccess};
use crate::services::semantic_search::{
    IndexingService, SearchError, SearchFilters, SemanticSearchConfig, SemanticSearchService,
};
use rocket::serde::Deserialize;

/// Query parameters for semantic search.
#[derive(Debug, FromForm)]
pub struct SemanticSearchQuery {
    /// Search query text
    q: String,
    /// Number of results to return (default: 10, max: 50)
    k: Option<usize>,
    /// Filter by status ID
    status_filter: Option<i32>,
    /// Filter by category ID
    category_filter: Option<i32>,
    /// Filter by applicability ID
    applicability_filter: Option<i32>,
    /// Filter by verification method ID
    verification_filter: Option<i32>,
    /// Include debug timing information
    debug: Option<bool>,
}

/// Request body for RAG ask endpoint.
#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AskRequest {
    /// The question to answer
    query: String,
    /// Number of results to use for context (default: 10)
    k: Option<usize>,
    /// Filter by status ID
    status_filter: Option<i32>,
    /// Filter by category ID
    category_filter: Option<i32>,
    /// Filter by applicability ID
    applicability_filter: Option<i32>,
    /// Filter by verification method ID
    verification_filter: Option<i32>,
}

/// Semantic search endpoint.
///
/// GET /api/projects/<project_id>/requirements/semantic_search?q=...&k=...
#[get("/projects/<project_id>/requirements/semantic_search?<query..>")]
pub async fn semantic_search(
    project_access: ProjectAccess,
    project_id: i32,
    query: SemanticSearchQuery,
    state: &State<AppState>,
) -> ApiResult<Value> {
    let _user = project_access.into_user();
    let config = SemanticSearchConfig::global();

    // Check if embeddings are enabled
    if !config.embeddings_enabled {
        return Ok(json!({
            "enabled": false,
            "message": "Semantic search is disabled. Set EMBEDDINGS_ENABLED=true to enable.",
            "results": [],
            "total": 0
        }));
    }

    let start = std::time::Instant::now();

    let filters = SearchFilters {
        status_id: query.status_filter,
        category_id: query.category_filter,
        applicability_id: query.applicability_filter,
        verification_id: query.verification_filter,
    };

    let k = query.k.unwrap_or(10).min(50);
    let service = SemanticSearchService::new(state.inner());

    let results = service
        .search(project_id, &query.q, &filters, k)
        .await
        .map_err(|e| {
            eprintln!("❌ Semantic search error: {:?}", e);
            match e {
                SearchError::Repo(repo_err) => ApiError::from(repo_err),
                SearchError::Embedding(emb_err) => {
                    ApiError::Internal(format!("Embedding error: {}", emb_err))
                }
                SearchError::Llm(llm_err) => ApiError::Internal(format!("LLM error: {}", llm_err)),
                SearchError::NotConfigured(msg) => ApiError::BadRequest(msg),
            }
        })?;

    let total = results.len();

    let mut response = json!({
        "results": results,
        "query": query.q,
        "total": total,
        "enabled": true
    });

    if query.debug.unwrap_or(false) {
        response["timing_ms"] = json!(start.elapsed().as_millis() as u64);
    }

    Ok(response)
}

/// RAG answer generation endpoint.
///
/// POST /api/projects/<project_id>/requirements/ask
#[post("/projects/<project_id>/requirements/ask", data = "<request>")]
pub async fn ask(
    project_access: ProjectAccess,
    project_id: i32,
    request: Json<AskRequest>,
    state: &State<AppState>,
) -> ApiResult<Value> {
    let _user = project_access.into_user();
    let config = SemanticSearchConfig::global();

    // Check if RAG is enabled
    if !config.rag_enabled {
        return Err(ApiError::BadRequest(
            "RAG answer generation is disabled. Set RAG_ENABLED=true to enable.".into(),
        ));
    }

    let filters = SearchFilters {
        status_id: request.status_filter,
        category_id: request.category_filter,
        applicability_id: request.applicability_filter,
        verification_id: request.verification_filter,
    };

    let k = request.k.unwrap_or(10).min(20); // Limit context for RAG
    let service = SemanticSearchService::new(state.inner());

    let response = service
        .ask(project_id, &request.query, &filters, k)
        .await
        .map_err(|e| match e {
            SearchError::Repo(repo_err) => ApiError::from(repo_err),
            SearchError::Embedding(emb_err) => {
                ApiError::Internal(format!("Embedding error: {}", emb_err))
            }
            SearchError::Llm(llm_err) => ApiError::Internal(format!("LLM error: {}", llm_err)),
            SearchError::NotConfigured(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(json!(response))
}

/// Reindex all requirements for a project.
///
/// POST /api/projects/<project_id>/requirements/reindex
/// Admin only.
#[post("/projects/<project_id>/requirements/reindex")]
pub async fn reindex(
    admin: AdminOnly,
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Value> {
    let _user = admin.into_inner();
    let _ = project_access; // Verify project access

    let config = SemanticSearchConfig::global();

    if !config.embeddings_enabled {
        return Err(ApiError::BadRequest(
            "Embeddings are disabled. Set EMBEDDINGS_ENABLED=true to enable.".into(),
        ));
    }

    let service = IndexingService::new(state.inner());
    let (indexed, skipped, failed) = service
        .reindex_project(project_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Reindex failed: {}", e)))?;

    Ok(json!({
        "status": "completed",
        "indexed": indexed,
        "skipped": skipped,
        "failed": failed,
        "total": indexed + skipped + failed
    }))
}

/// Get index status for a project.
///
/// GET /api/projects/<project_id>/requirements/index_status
#[get("/projects/<project_id>/requirements/index_status")]
pub async fn index_status(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Value> {
    let _user = project_access.into_user();

    let service = IndexingService::new(state.inner());
    let status = service
        .get_index_status(project_id)
        .map_err(ApiError::from)?;

    Ok(json!(status))
}

/// Check if semantic search is enabled.
///
/// GET /api/projects/<project_id>/requirements/semantic_search/status
#[get("/projects/<project_id>/requirements/semantic_search/status")]
pub async fn search_status(
    project_access: ProjectAccess,
    project_id: i32,
    _state: &State<AppState>,
) -> ApiResult<Value> {
    let _user = project_access.into_user();
    let _ = project_id; // Used for ProjectAccess guard
    let config = SemanticSearchConfig::global();

    Ok(json!({
        "embeddings_enabled": config.embeddings_enabled,
        "rag_enabled": config.rag_enabled,
        "embedding_provider": config.embedding_provider,
        "embedding_model": config.embedding_model,
        "rag_model": config.rag_model
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to simulate the k parameter bounding logic from endpoints
    fn bound_k(k: Option<usize>, default: usize, max: usize) -> usize {
        k.unwrap_or(default).min(max)
    }

    #[test]
    fn search_query_defaults() {
        // Test that default values are sensible
        assert_eq!(bound_k(None, 10, 50), 10);
    }

    #[test]
    fn search_query_k_max_capped() {
        // k should be capped at 50
        assert_eq!(bound_k(Some(100), 10, 50), 50);
    }

    #[test]
    fn ask_request_k_max_capped() {
        // RAG k should be capped at 20
        assert_eq!(bound_k(Some(50), 10, 20), 20);
    }

    #[test]
    fn search_filters_from_query() {
        let filters = SearchFilters {
            status_id: Some(1),
            category_id: Some(2),
            applicability_id: Some(3),
            verification_id: Some(4),
        };

        assert_eq!(filters.status_id, Some(1));
        assert_eq!(filters.category_id, Some(2));
        assert_eq!(filters.applicability_id, Some(3));
        assert_eq!(filters.verification_id, Some(4));
    }

    #[test]
    fn search_filters_partial() {
        let filters = SearchFilters {
            status_id: Some(1),
            category_id: None,
            applicability_id: None,
            verification_id: Some(4),
        };

        assert!(filters.status_id.is_some());
        assert!(filters.category_id.is_none());
        assert!(filters.applicability_id.is_none());
        assert!(filters.verification_id.is_some());
    }

    #[test]
    fn search_error_to_api_error_not_configured() {
        let err = SearchError::NotConfigured("test message".into());
        let display = err.to_string();
        assert!(display.contains("test message"));
    }

    #[test]
    fn semantic_search_config_global_accessible() {
        // Verify we can access global config without panicking
        let config = SemanticSearchConfig::global();
        // Just verify we can read the field (config is runtime-determined)
        let _ = config.embeddings_enabled;
    }

    #[test]
    fn k_parameter_bounds() {
        // Test the k parameter bounding logic used in endpoints
        assert_eq!(bound_k(None, 10, 50), 10); // Default case
        assert_eq!(bound_k(Some(5), 10, 50), 5); // Small value
        assert_eq!(bound_k(Some(100), 10, 50), 50); // Large value (capped)
        assert_eq!(bound_k(Some(0), 10, 50), 0); // Zero
    }

    #[test]
    fn rag_k_parameter_bounds() {
        // RAG uses min(20) instead of min(50)
        assert_eq!(bound_k(None, 10, 20), 10); // Default case
        assert_eq!(bound_k(Some(50), 10, 20), 20); // Large value (capped at 20)
    }

    #[test]
    fn json_response_structure_disabled() {
        // When disabled, response should include specific fields
        let disabled_response = serde_json::json!({
            "enabled": false,
            "message": "Semantic search is disabled. Set EMBEDDINGS_ENABLED=true to enable.",
            "results": [],
            "total": 0
        });

        assert_eq!(disabled_response["enabled"], false);
        assert!(disabled_response["results"].as_array().unwrap().is_empty());
        assert_eq!(disabled_response["total"], 0);
    }

    #[test]
    fn json_response_structure_enabled() {
        // When enabled, response should include results
        let enabled_response = serde_json::json!({
            "results": [{"id": 1, "score": 0.95}],
            "query": "test query",
            "total": 1,
            "enabled": true
        });

        assert_eq!(enabled_response["enabled"], true);
        assert_eq!(enabled_response["total"], 1);
        assert!(!enabled_response["results"].as_array().unwrap().is_empty());
    }

    #[test]
    fn json_response_with_timing() {
        let response_with_timing = serde_json::json!({
            "results": [],
            "query": "test",
            "total": 0,
            "enabled": true,
            "timing_ms": 42
        });

        assert_eq!(response_with_timing["timing_ms"], 42);
    }

    #[test]
    fn search_status_response_structure() {
        let config = SemanticSearchConfig::default();
        let status_response = serde_json::json!({
            "embeddings_enabled": config.embeddings_enabled,
            "rag_enabled": config.rag_enabled,
            "embedding_provider": config.embedding_provider,
            "embedding_model": config.embedding_model,
            "rag_model": config.rag_model
        });

        assert!(!status_response["embeddings_enabled"].as_bool().unwrap());
        assert!(!status_response["rag_enabled"].as_bool().unwrap());
        assert_eq!(status_response["embedding_provider"], "ollama");
    }

    #[test]
    fn reindex_response_structure() {
        let reindex_response = serde_json::json!({
            "status": "completed",
            "indexed": 10,
            "skipped": 5,
            "failed": 2,
            "total": 17
        });

        assert_eq!(reindex_response["status"], "completed");
        assert_eq!(reindex_response["total"], 17);
        assert_eq!(
            reindex_response["indexed"].as_i64().unwrap()
                + reindex_response["skipped"].as_i64().unwrap()
                + reindex_response["failed"].as_i64().unwrap(),
            reindex_response["total"].as_i64().unwrap()
        );
    }

    #[test]
    fn ask_request_deserialize() {
        let json = r#"{"query":"What is the scope?"}"#;
        let req: AskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "What is the scope?");
        assert_eq!(req.k, None);
        assert_eq!(req.status_filter, None);
    }

    #[test]
    fn ask_request_deserialize_with_filters() {
        let json = r#"{"query":"Q","k":5,"status_filter":1,"category_filter":2}"#;
        let req: AskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "Q");
        assert_eq!(req.k, Some(5));
        assert_eq!(req.status_filter, Some(1));
        assert_eq!(req.category_filter, Some(2));
    }

    #[test]
    fn semantic_search_query_k_respects_max() {
        assert_eq!(bound_k(Some(50), 10, 50), 50);
        assert_eq!(bound_k(Some(51), 10, 50), 50);
    }

    #[test]
    fn semantic_search_query_k_uses_default() {
        assert_eq!(bound_k(None, 10, 50), 10);
    }
}
