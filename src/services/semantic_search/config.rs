//! Configuration for semantic search services.
//!
//! Reads configuration from environment variables with sensible defaults.
//! Uses Ollama for local, open-source AI inference.

use std::env;
use std::sync::OnceLock;

/// Global configuration instance
static CONFIG: OnceLock<SemanticSearchConfig> = OnceLock::new();

/// Configuration for semantic search features.
#[derive(Debug, Clone)]
pub struct SemanticSearchConfig {
    /// Whether embeddings are enabled
    pub embeddings_enabled: bool,
    /// Embedding provider name: "ollama" or "mock" (for testing)
    pub embedding_provider: String,
    /// Embedding model name (Ollama model)
    pub embedding_model: String,
    /// Embedding dimension (auto-detected for known models)
    pub embedding_dim: usize,
    /// Ollama server URL
    pub ollama_url: String,
    /// Whether RAG answer generation is enabled
    pub rag_enabled: bool,
    /// LLM model for RAG answers (Ollama model)
    pub rag_model: String,
    /// Maximum tokens for RAG response
    pub rag_max_tokens: u32,
    /// Number of top results to use for RAG context
    pub rag_top_k: usize,
}

impl Default for SemanticSearchConfig {
    fn default() -> Self {
        Self {
            embeddings_enabled: false,
            embedding_provider: "ollama".into(),
            embedding_model: "nomic-embed-text".into(),
            embedding_dim: 768,
            ollama_url: "http://localhost:11434".into(),
            rag_enabled: false,
            rag_model: "llama3.2".into(),
            rag_max_tokens: 1024,
            rag_top_k: 10,
        }
    }
}

impl SemanticSearchConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let embeddings_enabled = env::var("EMBEDDINGS_ENABLED")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        let embedding_provider =
            env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ollama".into());

        let embedding_model =
            env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "nomic-embed-text".into());

        let embedding_dim = env::var("EMBEDDING_DIM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| Self::default_dim_for_model(&embedding_model));

        let ollama_url =
            env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

        let rag_enabled = env::var("RAG_ENABLED")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);

        let rag_model = env::var("RAG_MODEL").unwrap_or_else(|_| "llama3.2".into());

        let rag_max_tokens = env::var("RAG_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1024);

        let rag_top_k = env::var("RAG_TOP_K")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Self {
            embeddings_enabled,
            embedding_provider,
            embedding_model,
            embedding_dim,
            ollama_url,
            rag_enabled,
            rag_model,
            rag_max_tokens,
            rag_top_k,
        }
    }

    /// Get the global configuration instance.
    pub fn global() -> &'static Self {
        CONFIG.get_or_init(Self::from_env)
    }

    /// Get default embedding dimension for known Ollama models.
    pub fn default_dim_for_model(model: &str) -> usize {
        match model {
            "nomic-embed-text" => 768,
            "mxbai-embed-large" => 1024,
            "all-minilm" => 384,
            "snowflake-arctic-embed" => 1024,
            "bge-m3" => 1024,
            "bge-large" => 1024,
            _ => 768, // Default fallback
        }
    }

    /// Check if the configuration is valid for embedding operations.
    pub fn is_valid_for_embeddings(&self) -> Result<(), String> {
        if !self.embeddings_enabled {
            return Err("Embeddings are disabled. Set EMBEDDINGS_ENABLED=true".into());
        }

        match self.embedding_provider.as_str() {
            "ollama" | "mock" => Ok(()),
            other => Err(format!("Unknown embedding provider: {}. Use 'ollama' or 'mock'.", other)),
        }
    }

    /// Check if the configuration is valid for RAG operations.
    pub fn is_valid_for_rag(&self) -> Result<(), String> {
        self.is_valid_for_embeddings()?;

        if !self.rag_enabled {
            return Err("RAG is disabled. Set RAG_ENABLED=true".into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = SemanticSearchConfig::default();
        assert!(!config.embeddings_enabled);
        assert_eq!(config.embedding_provider, "ollama");
        assert_eq!(config.embedding_model, "nomic-embed-text");
        assert_eq!(config.embedding_dim, 768);
    }

    #[test]
    fn model_dimensions() {
        assert_eq!(
            SemanticSearchConfig::default_dim_for_model("nomic-embed-text"),
            768
        );
        assert_eq!(
            SemanticSearchConfig::default_dim_for_model("mxbai-embed-large"),
            1024
        );
        assert_eq!(
            SemanticSearchConfig::default_dim_for_model("all-minilm"),
            384
        );
        assert_eq!(
            SemanticSearchConfig::default_dim_for_model("unknown"),
            768
        );
    }

    #[test]
    fn validation_disabled_embeddings() {
        let config = SemanticSearchConfig::default();
        assert!(config.is_valid_for_embeddings().is_err());
    }

    #[test]
    fn validation_valid_ollama_config() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "ollama".into(),
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_ok());
    }

    #[test]
    fn validation_valid_mock_config() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_ok());
    }

    #[test]
    fn validation_unknown_provider_fails() {
        let config = SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "unknown".into(),
            ..Default::default()
        };
        assert!(config.is_valid_for_embeddings().is_err());
    }

    #[test]
    fn validation_rag_requires_embeddings() {
        let config = SemanticSearchConfig {
            embeddings_enabled: false,
            rag_enabled: true,
            ..Default::default()
        };
        assert!(config.is_valid_for_rag().is_err());
    }
}
