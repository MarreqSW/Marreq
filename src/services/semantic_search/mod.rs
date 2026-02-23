// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Semantic search services for RAG-powered requirement search.
//!
//! This module provides the infrastructure for:
//! - Embedding generation and storage using Ollama
//! - Hybrid search (lexical + vector) with RRF fusion
//! - RAG-based answer generation with citations
//!
//! # Configuration
//!
//! Set the following environment variables:
//! - `EMBEDDINGS_ENABLED=true` - Enable embedding generation
//! - `EMBEDDING_PROVIDER=ollama` - Provider (ollama or mock)
//! - `EMBEDDING_MODEL=nomic-embed-text` - Ollama embedding model
//! - `OLLAMA_URL=http://localhost:11434` - Ollama server URL
//! - `RAG_ENABLED=true` - Enable RAG answer generation
//! - `RAG_MODEL=llama3.2` - Ollama LLM model for answers
//!
//! See doc/OLLAMA_SETUP.md for installation and setup instructions.

pub mod config;
pub mod document_builder;
pub mod embedding_provider;
pub mod indexing_service;
pub mod llm_provider;
pub mod search_service;

pub use config::SemanticSearchConfig;
pub use document_builder::{build_embedding_document, compute_content_hash};
pub use embedding_provider::{
    create_embedding_provider, EmbeddingError, EmbeddingProvider, EmbeddingResult,
    MockEmbeddingProvider, OllamaEmbeddingProvider,
};
pub use indexing_service::IndexingService;
pub use llm_provider::{
    build_rag_system_prompt, build_rag_user_prompt, create_llm_provider, extract_citations,
    ChatMessage, LlmError, LlmProvider, LlmResult, MockLlmProvider, OllamaLlmProvider,
};
pub use search_service::{SearchError, SearchFilters, SemanticSearchService};
