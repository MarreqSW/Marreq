// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Semantic search indexing fairing.
//!
//! This fairing manages automatic embedding generation for requirements:
//! - On startup: processes any pending items in the index queue
//! - In background: periodically checks and processes new queue items
//!
//! # Configuration
//!
//! The fairing respects the `EMBEDDINGS_ENABLED` environment variable.
//! When disabled, the fairing is a no-op.

use crate::app::{AppState, DieselCachedRepo};
use crate::services::semantic_search::{IndexingService, SemanticSearchConfig};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Orbit, Rocket};
use std::time::Duration;

/// Interval between background queue processing runs.
const QUEUE_PROCESS_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum items to process per queue run.
const QUEUE_BATCH_SIZE: i64 = 50;

/// Fairing that manages semantic search indexing.
///
/// This fairing:
/// 1. Logs the semantic search configuration status on liftoff
/// 2. Processes any pending index queue items on startup
/// 3. Starts a background task to periodically process new queue items
pub struct SemanticIndexFairing;

#[rocket::async_trait]
impl Fairing for SemanticIndexFairing {
    fn info(&self) -> Info {
        Info {
            name: "Semantic Index Manager",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let config = SemanticSearchConfig::global();

        // Log configuration status
        if config.embeddings_enabled {
            eprintln!(
                "🔍 Semantic search enabled: provider={}, model={}, dim={}",
                config.embedding_provider, config.embedding_model, config.embedding_dim
            );
            if config.rag_enabled {
                eprintln!(
                    "🤖 RAG enabled: model={}, max_tokens={}",
                    config.rag_model, config.rag_max_tokens
                );
            }
        } else {
            eprintln!("🔍 Semantic search disabled (set EMBEDDINGS_ENABLED=true to enable)");
            return;
        }

        // Get AppState from managed state
        let state: &'static AppState<DieselCachedRepo> =
            match rocket.state::<AppState<DieselCachedRepo>>() {
                Some(s) => {
                    // SAFETY: Rocket's managed state has 'static lifetime
                    unsafe {
                        std::mem::transmute::<
                            &AppState<DieselCachedRepo>,
                            &'static AppState<DieselCachedRepo>,
                        >(s)
                    }
                }
                None => {
                    eprintln!("⚠️  Could not access AppState for semantic indexing");
                    return;
                }
            };

        // Process any pending queue items on startup
        let startup_result = tokio::spawn(async move { process_queue_once(state).await }).await;

        match startup_result {
            Ok(Ok((processed, failed))) => {
                if processed > 0 || failed > 0 {
                    eprintln!(
                        "📊 Startup index queue: processed={}, failed={}",
                        processed, failed
                    );
                }
            }
            Ok(Err(e)) => {
                eprintln!("⚠️  Startup queue processing error: {}", e);
            }
            Err(e) => {
                eprintln!("⚠️  Startup queue task error: {}", e);
            }
        }

        // Start background processing task
        tokio::spawn(async move {
            background_queue_processor(state).await;
        });

        eprintln!("✅ Semantic index background processor started");
    }
}

/// Process the embedding index queue once.
async fn process_queue_once(state: &AppState<DieselCachedRepo>) -> Result<(usize, usize), String> {
    let config = SemanticSearchConfig::global();
    if !config.embeddings_enabled {
        return Ok((0, 0));
    }

    let service = IndexingService::new(state);
    service
        .process_queue(QUEUE_BATCH_SIZE)
        .await
        .map_err(|e| e.to_string())
}

/// Exposed for tests: run process_queue_once (covers disabled and enabled paths).
#[cfg(test)]
pub(super) async fn test_process_queue_once(
    state: &AppState<DieselCachedRepo>,
) -> Result<(usize, usize), String> {
    process_queue_once(state).await
}

/// Background task that periodically processes the index queue.
async fn background_queue_processor(state: &'static AppState<DieselCachedRepo>) {
    loop {
        tokio::time::sleep(QUEUE_PROCESS_INTERVAL).await;

        let config = SemanticSearchConfig::global();
        if !config.embeddings_enabled {
            continue;
        }

        match process_queue_once(state).await {
            Ok((processed, failed)) => {
                if processed > 0 || failed > 0 {
                    eprintln!(
                        "📊 Background index queue: processed={}, failed={}",
                        processed, failed
                    );
                }
            }
            Err(e) => {
                // Log but don't crash - the queue will be retried
                eprintln!("⚠️  Background queue processing error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-time assertions on constants
    const _: () = assert!(QUEUE_PROCESS_INTERVAL.as_secs() >= 10);
    const _: () = assert!(QUEUE_PROCESS_INTERVAL.as_secs() <= 300);
    const _: () = assert!(QUEUE_BATCH_SIZE >= 1);
    const _: () = assert!(QUEUE_BATCH_SIZE <= 1000);

    #[test]
    fn constants_match_expected_values() {
        assert_eq!(QUEUE_PROCESS_INTERVAL, Duration::from_secs(60));
        assert_eq!(QUEUE_BATCH_SIZE, 50);
    }

    #[test]
    fn fairing_info() {
        let fairing = SemanticIndexFairing;
        let info = fairing.info();
        assert_eq!(info.name, "Semantic Index Manager");
        assert!(info.kind.is(Kind::Liftoff));
    }

    #[test]
    fn fairing_kind_is_liftoff_only() {
        let fairing = SemanticIndexFairing;
        let info = fairing.info();
        // Should only be Liftoff, not Request or Response
        assert!(info.kind.is(Kind::Liftoff));
    }

    #[test]
    fn semantic_search_config_accessible() {
        // Verify global config is accessible
        let config = SemanticSearchConfig::global();
        // Just verify it doesn't panic
        let _ = config.embeddings_enabled;
        let _ = config.rag_enabled;
    }

    #[test]
    fn config_embeddings_disabled_by_default() {
        // In test environment without env vars, embeddings should be disabled
        let config = SemanticSearchConfig::default();
        assert!(!config.embeddings_enabled);
    }

    #[test]
    fn indexing_service_requires_config() {
        // Verify the service respects configuration
        let config = SemanticSearchConfig {
            embeddings_enabled: false,
            ..Default::default()
        };
        assert!(!config.embeddings_enabled);
        assert!(config.is_valid_for_embeddings().is_err());
    }

    #[test]
    fn indexing_service_enabled_with_mock() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_ok());
    }

    /// Runs the fairing's on_liftoff by launching a minimal Rocket with timeout.
    /// With EMBEDDINGS_ENABLED unset, the fairing hits the "disabled" path and returns early.
    ///
    /// Uses port `0` so the OS picks a free port — avoids `Address already in use` when the
    /// default `8000` is taken (dev server, parallel tests, or another process).
    #[tokio::test]
    async fn fairing_on_liftoff_runs_embeddings_disabled_path() {
        let figment = rocket::Config::figment().merge(("port", 0u16));
        let rocket = rocket::custom(figment).attach(SemanticIndexFairing);
        let ignited = rocket.ignite().await.expect("ignite");
        // Launch with short timeout; on_liftoff runs during launch and hits the "else" branch
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(400), ignited.launch()).await;
        // Timeout (Err) is expected since we don't shut down the server
        assert!(result.is_err() || result.unwrap().is_ok());
    }

    /// Covers process_queue_once when embeddings are disabled (returns Ok((0, 0))).
    #[tokio::test]
    async fn process_queue_once_when_disabled_returns_zero() {
        use crate::app::AppState;
        use crate::repository::diesel_repo_mock::DieselRepoMock;
        use crate::repository::CacheRepository;
        use std::sync::{Arc, RwLock};

        let inner = DieselRepoMock::default();
        let cached = CacheRepository::new(inner, 60);
        let state = AppState {
            repo: Arc::new(RwLock::new(cached)),
        };
        let result = test_process_queue_once(&state).await;
        assert!(result.is_ok());
        let (processed, failed) = result.unwrap();
        assert_eq!(processed, 0);
        assert_eq!(failed, 0);
    }
}
