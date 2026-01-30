//! LLM provider trait and implementations for RAG answer generation.
//!
//! Provides a clean abstraction for generating answers from retrieved context,
//! with implementations for:
//! - **Ollama**: Open-source, runs locally (recommended)
//! - **Mock**: Deterministic responses for testing

use super::config::SemanticSearchConfig;
use crate::models::{RagCitation, SemanticSearchResult};
use serde::{Deserialize, Serialize};

/// Error type for LLM operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
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

/// Result type for LLM operations.
pub type LlmResult<T> = Result<T, LlmError>;

/// Message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

/// Trait for LLM providers.
pub trait LlmProvider: Send + Sync {
    /// Generate a chat completion.
    fn chat(&self, messages: &[ChatMessage], max_tokens: u32) -> LlmResult<String>;

    /// Get the model name.
    fn model_name(&self) -> &str;
}

/// Build the RAG system prompt.
pub fn build_rag_system_prompt() -> String {
    r#"You are a helpful assistant that answers questions about software requirements.

INSTRUCTIONS:
1. Use ONLY the provided requirements to answer the question
2. When citing a requirement, use its reference code in square brackets, e.g., [REQ-001]
3. If the provided requirements are insufficient to answer the question, say so clearly
4. Be concise and factual
5. Focus on what the requirements actually state, not assumptions
6. If multiple requirements are relevant, synthesize them into a coherent answer

FORMAT:
- Provide a direct answer to the question
- Include citations using reference codes [REQ-XXX] for each claim
- Keep the response focused and under 300 words unless more detail is requested"#
        .to_string()
}

/// Build the RAG user prompt with context.
pub fn build_rag_user_prompt(query: &str, results: &[SemanticSearchResult]) -> String {
    let mut context = String::new();
    context.push_str("RELEVANT REQUIREMENTS:\n\n");

    for (i, result) in results.iter().enumerate() {
        context.push_str(&format!(
            "{}. [{}] {}\n",
            i + 1,
            result.reference_code,
            result.title
        ));
        context.push_str(&format!("   Description: {}\n", result.description));
        context.push_str(&format!(
            "   Status: {} | Category: {} | Verification: {}\n\n",
            result.status, result.category, result.verification
        ));
    }

    format!(
        "{}\nQUESTION: {}\n\nProvide an answer based only on the requirements above.",
        context, query
    )
}

/// Extract citations from LLM response text.
pub fn extract_citations(response: &str, results: &[SemanticSearchResult]) -> Vec<RagCitation> {
    let mut citations = Vec::new();
    let mut seen_codes = std::collections::HashSet::new();

    // Match [REF-XXX] patterns in the response
    let re = regex::Regex::new(r"\[([A-Z]+-[A-Z0-9-]+)\]").unwrap();

    for cap in re.captures_iter(response) {
        let code = &cap[1];
        if seen_codes.contains(code) {
            continue;
        }

        // Find matching result
        if let Some(result) = results.iter().find(|r| r.reference_code == code) {
            citations.push(RagCitation {
                requirement_id: result.id,
                reference_code: result.reference_code.clone(),
                title: result.title.clone(),
            });
            seen_codes.insert(code.to_string());
        }
    }

    citations
}

/// Mock LLM provider for testing.
pub struct MockLlmProvider {
    model: String,
}

impl MockLlmProvider {
    pub fn new() -> Self {
        Self {
            model: "mock-llm".into(),
        }
    }
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProvider for MockLlmProvider {
    fn chat(&self, messages: &[ChatMessage], _max_tokens: u32) -> LlmResult<String> {
        // Extract the user message to find requirements context
        let empty = String::new();
        let user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| &m.content)
            .unwrap_or(&empty);

        // Extract reference codes from context
        let re = regex::Regex::new(r"\[([A-Z]+-[A-Z0-9-]+)\]").unwrap();
        let codes: Vec<&str> = re
            .captures_iter(user_msg)
            .filter_map(|c| c.get(1).map(|m| m.as_str()))
            .take(3)
            .collect();

        if codes.is_empty() {
            return Ok(
                "Based on the provided requirements, I cannot find sufficient information to answer this question."
                    .to_string(),
            );
        }

        // Generate a mock response citing the found requirements
        let citations = codes
            .iter()
            .map(|c| format!("[{}]", c))
            .collect::<Vec<_>>()
            .join(", ");

        Ok(format!(
            "Based on the requirements {}, the system addresses this concern through the documented specifications. \
            These requirements establish the necessary criteria for implementation and verification.",
            citations
        ))
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// Ollama LLM provider.
///
/// Connects to a local or remote Ollama server for chat completions.
pub struct OllamaLlmProvider {
    client: reqwest::blocking::Client,
    base_url: String,
    model: String,
}

impl OllamaLlmProvider {
    pub fn new(config: &SemanticSearchConfig) -> LlmResult<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // LLM can be slow
            .build()
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        Ok(Self {
            client,
            base_url: config.ollama_url.clone(),
            model: config.rag_model.clone(),
        })
    }
}

impl LlmProvider for OllamaLlmProvider {
    fn chat(&self, messages: &[ChatMessage], max_tokens: u32) -> LlmResult<String> {
        let url = format!("{}/api/chat", self.base_url);

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {
                "num_predict": max_tokens,
                "temperature": 0.3,  // Lower temperature for more factual responses
            }
        });

        let response = self.client.post(&url).json(&payload).send().map_err(|e| {
            if e.is_connect() {
                LlmError::ServerNotReachable(self.base_url.clone())
            } else {
                LlmError::ApiError(e.to_string())
            }
        })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(LlmError::ModelNotFound(self.model.clone()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();

            if text.contains("not found") || text.contains("pull") {
                return Err(LlmError::ModelNotFound(self.model.clone()));
            }

            return Err(LlmError::ApiError(format!("HTTP {}: {}", status, text)));
        }

        let body: serde_json::Value = response
            .json()
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let content = body
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| LlmError::InvalidResponse("Missing content in response".into()))?;

        Ok(content.to_string())
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// Create an LLM provider based on configuration.
pub fn create_llm_provider(config: &SemanticSearchConfig) -> LlmResult<Box<dyn LlmProvider>> {
    match config.embedding_provider.as_str() {
        "ollama" => Ok(Box::new(OllamaLlmProvider::new(config)?)),
        "mock" => Ok(Box::new(MockLlmProvider::new())),
        _ => Ok(Box::new(OllamaLlmProvider::new(config)?)), // Default to Ollama
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_generates_response() {
        let provider = MockLlmProvider::new();
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("RELEVANT REQUIREMENTS:\n1. [REQ-001] Test requirement\n\nQUESTION: What does this do?"),
        ];

        let response = provider.chat(&messages, 100).unwrap();
        assert!(response.contains("[REQ-001]"));
    }

    #[test]
    fn mock_provider_handles_no_context() {
        let provider = MockLlmProvider::new();
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("What is the meaning of life?"),
        ];

        let response = provider.chat(&messages, 100).unwrap();
        assert!(response.contains("cannot find sufficient information"));
    }

    #[test]
    fn mock_provider_multiple_citations() {
        let provider = MockLlmProvider::new();
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("RELEVANT REQUIREMENTS:\n1. [REQ-001] First\n2. [REQ-002] Second\n3. [REQ-003] Third\n\nQUESTION: Compare them"),
        ];

        let response = provider.chat(&messages, 100).unwrap();
        assert!(response.contains("[REQ-001]"));
        assert!(response.contains("[REQ-002]"));
        assert!(response.contains("[REQ-003]"));
    }

    #[test]
    fn mock_provider_limits_to_three_citations() {
        let provider = MockLlmProvider::new();
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("RELEVANT REQUIREMENTS:\n1. [REQ-001] First\n2. [REQ-002] Second\n3. [REQ-003] Third\n4. [REQ-004] Fourth\n5. [REQ-005] Fifth\n\nQUESTION: List them"),
        ];

        let response = provider.chat(&messages, 100).unwrap();
        // Should contain at most 3 citations
        let citation_count = response.matches("[REQ-").count();
        assert!(citation_count <= 3);
    }

    #[test]
    fn mock_provider_default() {
        let provider = MockLlmProvider::default();
        assert_eq!(provider.model_name(), "mock-llm");
    }

    #[test]
    fn mock_provider_model_name() {
        let provider = MockLlmProvider::new();
        assert_eq!(provider.model_name(), "mock-llm");
    }

    #[test]
    fn mock_provider_empty_messages() {
        let provider = MockLlmProvider::new();
        let messages: Vec<ChatMessage> = vec![];

        let response = provider.chat(&messages, 100).unwrap();
        assert!(response.contains("cannot find sufficient information"));
    }

    #[test]
    fn mock_provider_only_system_message() {
        let provider = MockLlmProvider::new();
        let messages = vec![
            ChatMessage::system("You are a helpful assistant."),
        ];

        let response = provider.chat(&messages, 100).unwrap();
        assert!(response.contains("cannot find sufficient information"));
    }

    #[test]
    fn extract_citations_finds_refs() {
        let response = "The system [REQ-001] handles this, and [REQ-002] provides backup.";
        let results = vec![
            SemanticSearchResult {
                id: 1,
                reference_code: "REQ-001".into(),
                title: "First".into(),
                description: "Desc".into(),
                snippet: "".into(),
                score: 1.0,
                rank: 1,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Cat".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
            SemanticSearchResult {
                id: 2,
                reference_code: "REQ-002".into(),
                title: "Second".into(),
                description: "Desc".into(),
                snippet: "".into(),
                score: 0.9,
                rank: 2,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Cat".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
        ];

        let citations = extract_citations(response, &results);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].reference_code, "REQ-001");
        assert_eq!(citations[1].reference_code, "REQ-002");
    }

    #[test]
    fn extract_citations_deduplicates() {
        let response = "See [REQ-001], also [REQ-001] again.";
        let results = vec![SemanticSearchResult {
            id: 1,
            reference_code: "REQ-001".into(),
            title: "First".into(),
            description: "Desc".into(),
            snippet: "".into(),
            score: 1.0,
            rank: 1,
            lexical_rank: None,
            vector_rank: None,
            status: "Draft".into(),
            category: "Cat".into(),
            applicability: "All".into(),
            verification: "Test".into(),
        }];

        let citations = extract_citations(response, &results);
        assert_eq!(citations.len(), 1);
    }

    #[test]
    fn extract_citations_empty_response() {
        let response = "";
        let results = vec![SemanticSearchResult {
            id: 1,
            reference_code: "REQ-001".into(),
            title: "First".into(),
            description: "Desc".into(),
            snippet: "".into(),
            score: 1.0,
            rank: 1,
            lexical_rank: None,
            vector_rank: None,
            status: "Draft".into(),
            category: "Cat".into(),
            applicability: "All".into(),
            verification: "Test".into(),
        }];

        let citations = extract_citations(response, &results);
        assert_eq!(citations.len(), 0);
    }

    #[test]
    fn extract_citations_no_matching_results() {
        let response = "See [REQ-999] for details.";
        let results = vec![SemanticSearchResult {
            id: 1,
            reference_code: "REQ-001".into(),
            title: "First".into(),
            description: "Desc".into(),
            snippet: "".into(),
            score: 1.0,
            rank: 1,
            lexical_rank: None,
            vector_rank: None,
            status: "Draft".into(),
            category: "Cat".into(),
            applicability: "All".into(),
            verification: "Test".into(),
        }];

        let citations = extract_citations(response, &results);
        assert_eq!(citations.len(), 0);
    }

    #[test]
    fn extract_citations_complex_codes() {
        let response = "See [SYS-PERF-001] and [REQ-SEC-002-A] for details.";
        let results = vec![
            SemanticSearchResult {
                id: 1,
                reference_code: "SYS-PERF-001".into(),
                title: "Performance".into(),
                description: "Desc".into(),
                snippet: "".into(),
                score: 1.0,
                rank: 1,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Cat".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
            SemanticSearchResult {
                id: 2,
                reference_code: "REQ-SEC-002-A".into(),
                title: "Security".into(),
                description: "Desc".into(),
                snippet: "".into(),
                score: 0.9,
                rank: 2,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Cat".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
        ];

        let citations = extract_citations(response, &results);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].reference_code, "SYS-PERF-001");
        assert_eq!(citations[1].reference_code, "REQ-SEC-002-A");
    }

    #[test]
    fn extract_citations_preserves_order() {
        let response = "First [REQ-003], then [REQ-001], finally [REQ-002].";
        let results = vec![
            SemanticSearchResult {
                id: 1,
                reference_code: "REQ-001".into(),
                title: "First".into(),
                description: "Desc".into(),
                snippet: "".into(),
                score: 1.0,
                rank: 1,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Cat".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
            SemanticSearchResult {
                id: 2,
                reference_code: "REQ-002".into(),
                title: "Second".into(),
                description: "Desc".into(),
                snippet: "".into(),
                score: 0.9,
                rank: 2,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Cat".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
            SemanticSearchResult {
                id: 3,
                reference_code: "REQ-003".into(),
                title: "Third".into(),
                description: "Desc".into(),
                snippet: "".into(),
                score: 0.8,
                rank: 3,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Cat".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
        ];

        let citations = extract_citations(response, &results);
        assert_eq!(citations.len(), 3);
        // Order should match appearance in response
        assert_eq!(citations[0].reference_code, "REQ-003");
        assert_eq!(citations[1].reference_code, "REQ-001");
        assert_eq!(citations[2].reference_code, "REQ-002");
    }

    #[test]
    fn build_rag_prompts() {
        let system = build_rag_system_prompt();
        assert!(system.contains("reference code"));
        assert!(system.contains("[REQ-001]"));

        let results = vec![SemanticSearchResult {
            id: 1,
            reference_code: "REQ-TEST".into(),
            title: "Test Title".into(),
            description: "Test description".into(),
            snippet: "".into(),
            score: 1.0,
            rank: 1,
            lexical_rank: None,
            vector_rank: None,
            status: "Draft".into(),
            category: "Safety".into(),
            applicability: "All".into(),
            verification: "Analysis".into(),
        }];

        let user = build_rag_user_prompt("What is the test about?", &results);
        assert!(user.contains("[REQ-TEST]"));
        assert!(user.contains("Test Title"));
        assert!(user.contains("Test description"));
    }

    #[test]
    fn build_rag_system_prompt_content() {
        let prompt = build_rag_system_prompt();
        
        // Check key instructions are present
        assert!(prompt.contains("ONLY the provided requirements"));
        assert!(prompt.contains("square brackets"));
        assert!(prompt.contains("concise"));
        assert!(prompt.contains("300 words"));
    }

    #[test]
    fn build_rag_user_prompt_empty_results() {
        let user = build_rag_user_prompt("What about nothing?", &[]);
        assert!(user.contains("RELEVANT REQUIREMENTS:"));
        assert!(user.contains("What about nothing?"));
    }

    #[test]
    fn build_rag_user_prompt_multiple_results() {
        let results = vec![
            SemanticSearchResult {
                id: 1,
                reference_code: "REQ-001".into(),
                title: "First Requirement".into(),
                description: "First description".into(),
                snippet: "".into(),
                score: 1.0,
                rank: 1,
                lexical_rank: None,
                vector_rank: None,
                status: "Active".into(),
                category: "Functional".into(),
                applicability: "All".into(),
                verification: "Test".into(),
            },
            SemanticSearchResult {
                id: 2,
                reference_code: "REQ-002".into(),
                title: "Second Requirement".into(),
                description: "Second description".into(),
                snippet: "".into(),
                score: 0.9,
                rank: 2,
                lexical_rank: None,
                vector_rank: None,
                status: "Draft".into(),
                category: "Performance".into(),
                applicability: "Variant A".into(),
                verification: "Analysis".into(),
            },
        ];

        let user = build_rag_user_prompt("Compare these requirements", &results);
        
        // Check numbering
        assert!(user.contains("1. [REQ-001]"));
        assert!(user.contains("2. [REQ-002]"));
        
        // Check content
        assert!(user.contains("First Requirement"));
        assert!(user.contains("Second Requirement"));
        assert!(user.contains("First description"));
        assert!(user.contains("Second description"));
        
        // Check metadata
        assert!(user.contains("Active"));
        assert!(user.contains("Functional"));
        assert!(user.contains("Compare these requirements"));
    }

    #[test]
    fn chat_message_constructors() {
        let system = ChatMessage::system("System message");
        assert_eq!(system.role, "system");
        assert_eq!(system.content, "System message");

        let user = ChatMessage::user("User message");
        assert_eq!(user.role, "user");
        assert_eq!(user.content, "User message");

        let assistant = ChatMessage::assistant("Assistant message");
        assert_eq!(assistant.role, "assistant");
        assert_eq!(assistant.content, "Assistant message");
    }

    #[test]
    fn chat_message_into_string() {
        // Test that Into<String> works
        let msg = ChatMessage::system(String::from("Owned string"));
        assert_eq!(msg.content, "Owned string");

        let msg = ChatMessage::user("String literal");
        assert_eq!(msg.content, "String literal");
    }

    #[test]
    fn llm_error_not_configured_display() {
        let err = LlmError::NotConfigured("test".into());
        assert!(err.to_string().contains("not configured"));
    }

    #[test]
    fn llm_error_api_error_display() {
        let err = LlmError::ApiError("connection failed".into());
        assert!(err.to_string().contains("API request failed"));
    }

    #[test]
    fn llm_error_invalid_response_display() {
        let err = LlmError::InvalidResponse("missing field".into());
        assert!(err.to_string().contains("Invalid response"));
    }

    #[test]
    fn llm_error_model_not_found_display() {
        let err = LlmError::ModelNotFound("llama3.2".into());
        let display = err.to_string();
        assert!(display.contains("Model not found"));
        assert!(display.contains("llama3.2"));
        assert!(display.contains("ollama pull"));
    }

    #[test]
    fn llm_error_server_not_reachable_display() {
        let err = LlmError::ServerNotReachable("http://localhost:11434".into());
        let display = err.to_string();
        assert!(display.contains("not reachable"));
        assert!(display.contains("http://localhost:11434"));
    }

    #[test]
    fn create_mock_llm_provider() {
        let config = SemanticSearchConfig {
            embedding_provider: "mock".into(),
            ..Default::default()
        };

        let provider = create_llm_provider(&config).unwrap();
        assert_eq!(provider.model_name(), "mock-llm");
    }

    #[test]
    fn create_ollama_llm_provider() {
        let config = SemanticSearchConfig {
            embedding_provider: "ollama".into(),
            rag_model: "llama3.2".into(),
            ollama_url: "http://localhost:11434".into(),
            ..Default::default()
        };

        let provider = create_llm_provider(&config).unwrap();
        assert_eq!(provider.model_name(), "llama3.2");
    }

    #[test]
    fn create_default_llm_provider() {
        // Default (unknown provider) should fall back to Ollama
        let config = SemanticSearchConfig {
            embedding_provider: "something_else".into(),
            rag_model: "custom-model".into(),
            ..Default::default()
        };

        let provider = create_llm_provider(&config).unwrap();
        assert_eq!(provider.model_name(), "custom-model");
    }

    #[test]
    fn rag_citation_fields() {
        let citation = RagCitation {
            requirement_id: 42,
            reference_code: "REQ-042".into(),
            title: "Test Requirement".into(),
        };

        assert_eq!(citation.requirement_id, 42);
        assert_eq!(citation.reference_code, "REQ-042");
        assert_eq!(citation.title, "Test Requirement");
    }
}
