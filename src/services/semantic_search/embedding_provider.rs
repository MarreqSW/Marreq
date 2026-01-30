//! Embedding provider trait and implementations.
//!
//! Provides a clean abstraction for generating text embeddings, with
//! implementations for:
//! - **Ollama**: Open-source, runs locally (recommended)
//! - **Mock**: Deterministic embeddings for testing

use super::config::SemanticSearchConfig;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Error type for embedding operations.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Provider not configured: {0}")]
    NotConfigured(String),
    #[error("API request failed: {0}")]
    ApiError(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Model not found: {0}. Run 'ollama pull {0}' to download it.")]
    ModelNotFound(String),
    #[error("Ollama server not reachable at {0}. Is Ollama running?")]
    ServerNotReachable(String),
}

/// Result type for embedding operations.
pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

/// Trait for embedding providers.
#[rocket::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for a batch of texts.
    async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<Vec<f32>>>;

    /// Generate embedding for a single text.
    async fn embed(&self, text: &str) -> EmbeddingResult<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::InvalidResponse("Empty batch result".into()))
    }

    /// Get the model name.
    fn model_name(&self) -> &str;

    /// Get the embedding dimension.
    fn dimension(&self) -> usize;
}

/// Mock embedding provider for testing.
///
/// Generates deterministic embeddings based on text hash for reproducible tests.
pub struct MockEmbeddingProvider {
    model: String,
    dimension: usize,
}

impl MockEmbeddingProvider {
    pub fn new(dimension: usize) -> Self {
        Self {
            model: "mock".into(),
            dimension,
        }
    }

    /// Generate a deterministic embedding from text.
    fn deterministic_embedding(&self, text: &str) -> Vec<f32> {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        // Generate pseudo-random normalized vector
        let mut embedding = Vec::with_capacity(self.dimension);
        let mut state = seed;
        let mut sum_sq = 0.0f32;

        for _ in 0..self.dimension {
            // Simple LCG PRNG
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let val = ((state >> 33) as f32 / u32::MAX as f32) * 2.0 - 1.0;
            embedding.push(val);
            sum_sq += val * val;
        }

        // Normalize to unit vector
        let norm = sum_sq.sqrt();
        if norm > 0.0 {
            for v in &mut embedding {
                *v /= norm;
            }
        }

        embedding
    }
}

#[rocket::async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|t| self.deterministic_embedding(t))
            .collect())
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Ollama embedding provider.
///
/// Connects to a local or remote Ollama server for embedding generation.
/// See https://ollama.ai for installation instructions.
pub struct OllamaEmbeddingProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    dimension: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(config: &SemanticSearchConfig) -> EmbeddingResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        Ok(Self {
            client,
            base_url: config.ollama_url.clone(),
            model: config.embedding_model.clone(),
            dimension: config.embedding_dim,
        })
    }

    pub fn from_env() -> EmbeddingResult<Self> {
        Self::new(SemanticSearchConfig::global())
    }

    /// Check if the Ollama server is reachable.
    pub async fn health_check(&self) -> EmbeddingResult<()> {
        let url = format!("{}/api/tags", self.base_url);
        self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|_| EmbeddingError::ServerNotReachable(self.base_url.clone()))?;
        Ok(())
    }
}
#[rocket::async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed_batch(&self, texts: &[String]) -> EmbeddingResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let url = format!("{}/api/embed", self.base_url);
        let mut embeddings = Vec::with_capacity(texts.len());

        // Ollama's embed endpoint can handle multiple prompts
        let payload = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    EmbeddingError::ServerNotReachable(self.base_url.clone())
                } else {
                    EmbeddingError::ApiError(e.to_string())
                }
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(EmbeddingError::ModelNotFound(self.model.clone()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();

            // Check for model not found in error message
            if text.contains("not found") || text.contains("pull") {
                return Err(EmbeddingError::ModelNotFound(self.model.clone()));
            }

            return Err(EmbeddingError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

        // Ollama returns embeddings array directly
        let emb_array = body
            .get("embeddings")
            .and_then(|e| e.as_array())
            .ok_or_else(|| EmbeddingError::InvalidResponse("Missing 'embeddings' array".into()))?;

        for emb_value in emb_array {
            let embedding: Vec<f32> = emb_value
                .as_array()
                .ok_or_else(|| EmbeddingError::InvalidResponse("Invalid embedding format".into()))?
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();

            if embedding.is_empty() {
                return Err(EmbeddingError::InvalidResponse(
                    "Empty embedding returned".into(),
                ));
            }

            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Create an embedding provider based on configuration.
pub fn create_embedding_provider(
    config: &SemanticSearchConfig,
) -> EmbeddingResult<Box<dyn EmbeddingProvider>> {
    match config.embedding_provider.as_str() {
        "ollama" => Ok(Box::new(OllamaEmbeddingProvider::new(config)?)),
        "mock" => Ok(Box::new(MockEmbeddingProvider::new(config.embedding_dim))),
        other => Err(EmbeddingError::NotConfigured(format!(
            "Unknown provider: {}. Use 'ollama' or 'mock'.",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_deterministic() {
        let provider = MockEmbeddingProvider::new(768);
        let text = "Test requirement description";

        let emb1 = provider.embed(text).await.unwrap();
        let emb2 = provider.embed(text).await.unwrap();

        assert_eq!(emb1.len(), 768);
        assert_eq!(emb1, emb2, "Same text should produce same embedding");
    }

    #[tokio::test]
    async fn mock_provider_different_texts() {
        let provider = MockEmbeddingProvider::new(768);

        let emb1 = provider.embed("First text").await.unwrap();
        let emb2 = provider.embed("Second text").await.unwrap();

        assert_ne!(
            emb1, emb2,
            "Different texts should produce different embeddings"
        );
    }

    #[tokio::test]
    async fn mock_provider_normalized() {
        let provider = MockEmbeddingProvider::new(768);
        let emb = provider.embed("Test").await.unwrap();

        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001, "Embedding should be normalized");
    }

    #[tokio::test]
    async fn mock_provider_batch() {
        let provider = MockEmbeddingProvider::new(768);
        let texts = vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ];

        let results = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn mock_provider_empty_batch() {
        let provider = MockEmbeddingProvider::new(768);
        let texts: Vec<String> = vec![];

        let results = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn mock_provider_single_batch() {
        let provider = MockEmbeddingProvider::new(768);
        let texts = vec!["Single text".to_string()];

        let results = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].len(), 768);
    }

    #[tokio::test]
    async fn mock_provider_different_dimensions() {
        let provider_384 = MockEmbeddingProvider::new(384);
        let provider_1024 = MockEmbeddingProvider::new(1024);

        let emb_384 = provider_384.embed("Test").await.unwrap();
        let emb_1024 = provider_1024.embed("Test").await.unwrap();

        assert_eq!(emb_384.len(), 384);
        assert_eq!(emb_1024.len(), 1024);
        assert_eq!(provider_384.dimension(), 384);
        assert_eq!(provider_1024.dimension(), 1024);
    }

    #[tokio::test]
    async fn mock_provider_model_name() {
        let provider = MockEmbeddingProvider::new(768);
        assert_eq!(provider.model_name(), "mock");
    }

    #[tokio::test]
    async fn mock_provider_consistent_across_batches() {
        let provider = MockEmbeddingProvider::new(768);
        
        // Single embed
        let single = provider.embed("Test text").await.unwrap();
        
        // Batch embed with same text
        let batch = provider.embed_batch(&["Test text".to_string()]).await.unwrap();
        
        assert_eq!(single, batch[0], "Single and batch should produce same embedding");
    }

    #[tokio::test]
    async fn mock_provider_unicode_text() {
        let provider = MockEmbeddingProvider::new(768);
        let emb = provider.embed("日本語テスト 🚀 émojis").await.unwrap();
        
        assert_eq!(emb.len(), 768);
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn mock_provider_empty_text() {
        let provider = MockEmbeddingProvider::new(768);
        let emb = provider.embed("").await.unwrap();
        
        assert_eq!(emb.len(), 768);
    }

    #[tokio::test]
    async fn mock_provider_very_long_text() {
        let provider = MockEmbeddingProvider::new(768);
        let long_text = "a".repeat(100_000);
        let emb = provider.embed(&long_text).await.unwrap();
        
        assert_eq!(emb.len(), 768);
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn create_mock_provider() {
        let config = SemanticSearchConfig {
            embedding_provider: "mock".into(),
            embedding_dim: 768,
            ..Default::default()
        };

        let provider = create_embedding_provider(&config).unwrap();
        assert_eq!(provider.model_name(), "mock");
        assert_eq!(provider.dimension(), 768);
    }

    #[test]
    fn create_mock_provider_custom_dimension() {
        let config = SemanticSearchConfig {
            embedding_provider: "mock".into(),
            embedding_dim: 384,
            ..Default::default()
        };

        let provider = create_embedding_provider(&config).unwrap();
        assert_eq!(provider.dimension(), 384);
    }

    #[test]
    fn create_unknown_provider_fails() {
        let config = SemanticSearchConfig {
            embedding_provider: "unknown".into(),
            ..Default::default()
        };

        let result = create_embedding_provider(&config);
        assert!(result.is_err());
        
        match result {
            Err(err) => {
                assert!(matches!(err, EmbeddingError::NotConfigured(_)));
                assert!(err.to_string().contains("unknown"));
            }
            Ok(_) => panic!("Expected error but got Ok"),
        }
    }

    #[test]
    fn create_ollama_provider() {
        let config = SemanticSearchConfig {
            embedding_provider: "ollama".into(),
            embedding_model: "nomic-embed-text".into(),
            embedding_dim: 768,
            ollama_url: "http://localhost:11434".into(),
            ..Default::default()
        };

        // This should succeed even if Ollama isn't running (lazy connection)
        let provider = create_embedding_provider(&config).unwrap();
        assert_eq!(provider.model_name(), "nomic-embed-text");
        assert_eq!(provider.dimension(), 768);
    }

    #[test]
    fn embedding_error_not_configured_display() {
        let err = EmbeddingError::NotConfigured("test message".into());
        let display = err.to_string();
        assert!(display.contains("not configured"));
        assert!(display.contains("test message"));
    }

    #[test]
    fn embedding_error_api_error_display() {
        let err = EmbeddingError::ApiError("connection failed".into());
        let display = err.to_string();
        assert!(display.contains("API request failed"));
        assert!(display.contains("connection failed"));
    }

    #[test]
    fn embedding_error_invalid_response_display() {
        let err = EmbeddingError::InvalidResponse("missing field".into());
        let display = err.to_string();
        assert!(display.contains("Invalid response"));
        assert!(display.contains("missing field"));
    }

    #[test]
    fn embedding_error_model_not_found_display() {
        let err = EmbeddingError::ModelNotFound("nomic-embed-text".into());
        let display = err.to_string();
        assert!(display.contains("Model not found"));
        assert!(display.contains("nomic-embed-text"));
        assert!(display.contains("ollama pull"));
    }

    #[test]
    fn embedding_error_server_not_reachable_display() {
        let err = EmbeddingError::ServerNotReachable("http://localhost:11434".into());
        let display = err.to_string();
        assert!(display.contains("not reachable"));
        assert!(display.contains("http://localhost:11434"));
        assert!(display.contains("Ollama running"));
    }

    #[test]
    fn ollama_provider_new_custom_config() {
        let config = SemanticSearchConfig {
            embedding_provider: "ollama".into(),
            embedding_model: "custom-model".into(),
            embedding_dim: 512,
            ollama_url: "http://custom:8080".into(),
            ..Default::default()
        };

        let provider = OllamaEmbeddingProvider::new(&config).unwrap();
        assert_eq!(provider.model_name(), "custom-model");
        assert_eq!(provider.dimension(), 512);
    }

    #[test]
    fn mock_provider_dimension_zero() {
        // Edge case: what happens with 0 dimensions?
        let provider = MockEmbeddingProvider::new(0);
        assert_eq!(provider.dimension(), 0);
    }

    #[tokio::test]
    async fn mock_provider_single_embed() {
        // Test the single-text embed method (default implementation)
        let provider = MockEmbeddingProvider::new(64);
        let embedding = provider.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), 64);
    }

    #[tokio::test]
    async fn mock_provider_single_embed_deterministic() {
        // Verify single embed produces same result as batch
        let provider = MockEmbeddingProvider::new(64);
        
        let single = provider.embed("hello world").await.unwrap();
        let batch = provider.embed_batch(&["hello world".to_string()]).await.unwrap();
        
        assert_eq!(single, batch[0]);
    }

    #[tokio::test]
    async fn mock_provider_embed_whitespace() {
        let provider = MockEmbeddingProvider::new(32);
        let embedding = provider.embed("  ").await.unwrap();
        assert_eq!(embedding.len(), 32);
    }

    #[tokio::test]
    async fn mock_provider_embed_special_chars() {
        let provider = MockEmbeddingProvider::new(32);
        let embedding = provider.embed("@#$%^&*()!").await.unwrap();
        assert_eq!(embedding.len(), 32);
    }

    #[tokio::test]
    async fn mock_provider_embed_newlines() {
        let provider = MockEmbeddingProvider::new(32);
        let embedding = provider.embed("line1\nline2\nline3").await.unwrap();
        assert_eq!(embedding.len(), 32);
    }
}
