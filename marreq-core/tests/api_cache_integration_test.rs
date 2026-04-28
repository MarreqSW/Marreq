// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Cache API endpoints.
//!
//! These tests verify the complete behavior of `/api/cache` endpoints including:
//! - Statistics retrieval
//! - Cache clearing
//! - Performance metrics
//! - Health checks
//! - Recommendations

use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;

mod test_support {
    use super::*;
    use marreq_core::app::AppState;
    use marreq_core::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use std::sync::{Arc, RwLock};

    pub type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    pub fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    pub async fn test_client() -> Client {
        marreq_core::deployment::install_test_server_mode();
        let rocket = rocket::build()
            .manage(managed_state(DieselRepoMock::default()))
            .manage(marreq_core::auth::rate_limiter::LoginRateLimiter::new())
            .mount("/api", marreq_core::api::routes());

        Client::tracked(rocket).await.expect("rocket instance")
    }
}

use test_support::*;

// ============================================================================
// GET /api/cache/stats
// ============================================================================

#[rocket::async_test]
async fn get_stats_returns_cache_statistics() {
    let client = test_client().await;

    let response = client.get("/api/cache/stats").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let stats: Value = response.into_json().await.expect("json");

    assert!(stats["hits"].is_number());
    assert!(stats["misses"].is_number());
    assert!(stats["cache_size_bytes"].is_number());
    assert!(stats["total_entries"].is_number());
}

// ============================================================================
// POST /api/cache/clear
// ============================================================================

#[rocket::async_test]
async fn post_clear_clears_cache() {
    let client = test_client().await;

    let response = client
        .post("/api/cache/clear")
        .header(ContentType::JSON)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["message"], "Cache cleared successfully");
    assert!(result["timestamp"].is_string());
}

// ============================================================================
// POST /api/cache/cleanup
// ============================================================================

#[rocket::async_test]
async fn post_cleanup_removes_expired_entries() {
    let client = test_client().await;

    let response = client
        .post("/api/cache/cleanup")
        .header(ContentType::JSON)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert!(result["cleaned_entries"].is_number());
}

// ============================================================================
// GET /api/cache/performance
// ============================================================================

#[rocket::async_test]
async fn get_performance_returns_metrics() {
    let client = test_client().await;

    let response = client.get("/api/cache/performance").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let perf: Value = response.into_json().await.expect("json");
    assert!(perf.is_object());
}

// ============================================================================
// GET /api/cache/health
// ============================================================================

#[rocket::async_test]
async fn get_health_returns_status() {
    let client = test_client().await;

    let response = client.get("/api/cache/health").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let health: Value = response.into_json().await.expect("json");
    assert!(health.is_object());
}

// ============================================================================
// GET /api/cache/recommendations
// ============================================================================

#[rocket::async_test]
async fn get_recommendations_returns_suggestions() {
    let client = test_client().await;

    let response = client.get("/api/cache/recommendations").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let recs: Value = response.into_json().await.expect("json");
    assert!(recs["recommendations"].is_array());
}

// ============================================================================
// POST /api/cache/reset-counters
// ============================================================================

#[rocket::async_test]
async fn post_reset_counters_resets_metrics() {
    let client = test_client().await;

    let response = client
        .post("/api/cache/reset-counters")
        .header(ContentType::JSON)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(
        result["message"],
        "Cache performance counters reset successfully"
    );
}
