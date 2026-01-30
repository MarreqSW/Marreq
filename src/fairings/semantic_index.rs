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
                    unsafe { std::mem::transmute(s) }
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
