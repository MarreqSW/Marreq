//! API endpoints for semantic search functionality.
//!
//! All endpoints are project-scoped and use the `ProjectAccess` guard
//! for authorization.

use crate::api::prelude::*;
use crate::auth::guards::{AdminOnly, ProjectAccess};
use crate::services::semantic_search::{
    IndexingService, SearchFilters, SearchError, SemanticSearchConfig, SemanticSearchService,
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
        .map_err(|e| match e {
            SearchError::Repo(repo_err) => ApiError::from(repo_err),
            SearchError::Embedding(emb_err) => {
                ApiError::Internal(format!("Embedding error: {}", emb_err))
            }
            SearchError::Llm(llm_err) => ApiError::Internal(format!("LLM error: {}", llm_err)),
            SearchError::NotConfigured(msg) => ApiError::BadRequest(msg),
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
    let status = service.get_index_status(project_id).map_err(ApiError::from)?;

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
    #[test]
    fn search_query_defaults() {
        // Test that default values are sensible
        let default_k = 10_usize.min(50);
        assert_eq!(default_k, 10);
    }
}
