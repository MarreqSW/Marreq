// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Integration tests for semantic search API endpoints.

use marreq::app::AppState;
use marreq::auth::session::SESSION_COOKIE;
use marreq::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};

type TestState = AppState<CacheRepository<DieselRepoMock>>;

const ADMIN_ID: i32 = 1;
const PROJECT_ID: i32 = 1;

fn state_from_repo(repo: DieselRepoMock) -> TestState {
    AppState {
        repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
    }
}

async fn client_with_repo(repo: DieselRepoMock) -> Client {
    let rocket = rocket::build()
        .manage(state_from_repo(repo.with_admin_user()))
        .manage(marreq::auth::rate_limiter::LoginRateLimiter::new())
        .mount("/api", marreq::api::routes());
    Client::tracked(rocket).await.unwrap()
}

fn auth_cookie() -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
    cookie.set_path("/");
    cookie
}

#[rocket::async_test]
async fn search_status_returns_config() {
    let client = client_with_repo(DieselRepoMock::default()).await;
    let response = client
        .get(format!(
            "/api/projects/{}/requirements/semantic_search/status",
            PROJECT_ID
        ))
        .private_cookie(auth_cookie())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let payload: Value = response.into_json().await.unwrap();

    // Default config should have embeddings disabled
    assert!(payload.get("embeddings_enabled").is_some());
    assert!(payload.get("rag_enabled").is_some());
    assert!(payload.get("embedding_provider").is_some());
    assert!(payload.get("embedding_model").is_some());
}

#[rocket::async_test]
async fn search_returns_disabled_message_when_not_configured() {
    let client = client_with_repo(DieselRepoMock::default()).await;
    let response = client
        .get(format!(
            "/api/projects/{}/requirements/semantic_search?q=test",
            PROJECT_ID
        ))
        .private_cookie(auth_cookie())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let payload: Value = response.into_json().await.unwrap();

    // Should indicate disabled state
    assert_eq!(payload.get("enabled"), Some(&Value::from(false)));
    assert!(payload.get("message").is_some());
}

#[rocket::async_test]
async fn search_requires_authentication() {
    let client = client_with_repo(DieselRepoMock::default()).await;
    let response = client
        .get(format!(
            "/api/projects/{}/requirements/semantic_search?q=test",
            PROJECT_ID
        ))
        // No auth cookie
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn ask_returns_error_when_rag_disabled() {
    let client = client_with_repo(DieselRepoMock::default()).await;
    let response = client
        .post(format!("/api/projects/{}/requirements/ask", PROJECT_ID))
        .header(ContentType::JSON)
        .private_cookie(auth_cookie())
        .body(json!({ "query": "What are the safety requirements?" }).to_string())
        .dispatch()
        .await;

    // Should return bad request when RAG is disabled
    assert_eq!(response.status(), Status::BadRequest);
    let payload: Value = response.into_json().await.unwrap();
    assert!(payload
        .get("message")
        .and_then(|m| m.as_str())
        .map(|s| s.contains("disabled"))
        .unwrap_or(false));
}

#[rocket::async_test]
async fn reindex_returns_error_when_embeddings_disabled() {
    let client = client_with_repo(DieselRepoMock::default()).await;
    let response = client
        .post(format!("/api/projects/{}/requirements/reindex", PROJECT_ID))
        .private_cookie(auth_cookie())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
    let payload: Value = response.into_json().await.unwrap();
    assert!(payload
        .get("message")
        .and_then(|m| m.as_str())
        .map(|s| s.contains("disabled"))
        .unwrap_or(false));
}

#[rocket::async_test]
async fn search_with_empty_query_returns_empty_results() {
    let client = client_with_repo(DieselRepoMock::default()).await;
    let response = client
        .get(format!(
            "/api/projects/{}/requirements/semantic_search?q=",
            PROJECT_ID
        ))
        .private_cookie(auth_cookie())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let payload: Value = response.into_json().await.unwrap();

    // Empty query should return empty results or disabled message
    let results = payload.get("results").and_then(|r| r.as_array());
    if let Some(results) = results {
        assert!(results.is_empty());
    }
}

// Note: Full integration tests with actual embeddings require database setup
// and enabling EMBEDDINGS_ENABLED=true. These tests verify the API contract
// when features are disabled.

#[cfg(test)]
mod document_builder_tests {
    use marreq::models::DecoratedRequirement;
    use marreq::services::semantic_search::{build_embedding_document, compute_content_hash};

    fn sample_requirement() -> DecoratedRequirement {
        DecoratedRequirement {
            id: 1,
            current_version_id: None,
            title: "System shall process inputs".into(),
            description: "The system shall process all valid inputs within 100ms".into(),
            verification_method_id: "Analysis".into(),
            req_verification_ids: vec![1],
            status_id: "Draft".into(),
            req_current_status_id: 1,
            status_tag_color: None,
            author_id: "John Doe".into(),
            req_author_id: 1,
            reviewer_id: "Jane Doe".into(),
            req_reviewer_id: 2,
            reference_code: "REQ-SYS-001".into(),
            category_id: "Functional".into(),
            req_category_id: 1,
            applicability_id: "All Variants".into(),
            req_applicability_id: 1,
            req_parent_id: Some(0),
            req_parent_title: "System Requirements".into(),
            req_parents: vec![],
            req_parent_reference_code: "".into(),
            req_parent_description: "".into(),
            req_parent_status_id: "".into(),
            req_parent_status_tag_color: None,
            req_parent_category_id: "".into(),
            creation_date: "2024-01-01".into(),
            update_date: "2024-01-15".into(),
            deadline_date: "".into(),
            justification: Some("Required for real-time operation".into()),
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    #[test]
    fn document_builder_deterministic() {
        let req = sample_requirement();
        let doc1 = build_embedding_document(&req);
        let doc2 = build_embedding_document(&req);
        assert_eq!(
            doc1, doc2,
            "Same requirement should produce identical document"
        );
    }

    #[test]
    fn content_hash_stable() {
        let req = sample_requirement();
        let doc = build_embedding_document(&req);
        let model = "text-embedding-3-small";

        let hash1 = compute_content_hash(&doc, model);
        let hash2 = compute_content_hash(&doc, model);

        assert_eq!(hash1, hash2, "Same input should produce identical hash");
        assert_eq!(hash1.len(), 64, "SHA-256 should produce 64 hex chars");
    }

    #[test]
    fn content_hash_changes_with_model() {
        let req = sample_requirement();
        let doc = build_embedding_document(&req);

        let hash1 = compute_content_hash(&doc, "model-a");
        let hash2 = compute_content_hash(&doc, "model-b");

        assert_ne!(
            hash1, hash2,
            "Different models should produce different hashes"
        );
    }

    #[test]
    fn document_includes_key_fields() {
        let req = sample_requirement();
        let doc = build_embedding_document(&req);

        assert!(doc.contains("[REF] REQ-SYS-001"));
        assert!(doc.contains("[TITLE]"));
        assert!(doc.contains("[DESC]"));
        assert!(doc.contains("[RATIONALE]"));
        assert!(doc.contains("[CATEGORY]"));
        assert!(doc.contains("[STATUS]"));
    }
}

#[cfg(test)]
mod embedding_provider_tests {
    use marreq::services::semantic_search::{EmbeddingProvider, MockEmbeddingProvider};

    #[tokio::test]
    async fn mock_provider_produces_deterministic_embeddings() {
        let provider = MockEmbeddingProvider::new(1536);
        let text = "Test requirement description";

        let emb1 = provider.embed(text).await.unwrap();
        let emb2 = provider.embed(text).await.unwrap();

        assert_eq!(emb1.len(), 1536);
        assert_eq!(emb1, emb2, "Same text should produce same embedding");
    }

    #[tokio::test]
    async fn mock_provider_embeddings_are_normalized() {
        let provider = MockEmbeddingProvider::new(1536);
        let emb = provider.embed("Test").await.unwrap();

        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.001,
            "Embedding should be normalized to unit length"
        );
    }

    #[tokio::test]
    async fn mock_provider_batch_works() {
        let provider = MockEmbeddingProvider::new(1536);
        let texts = vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ];

        let results = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(results.len(), 3);
        for emb in &results {
            assert_eq!(emb.len(), 1536);
        }
    }
}
