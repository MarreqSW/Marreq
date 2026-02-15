//! Semantic search service with hybrid retrieval.
//!
//! Implements hybrid search combining:
//! - Lexical search using Postgres full-text search (tsvector)
//! - Dense vector search using pgvector embeddings
//! - Reciprocal Rank Fusion (RRF) for combining results

use super::config::SemanticSearchConfig;
use super::embedding_provider::{create_embedding_provider, EmbeddingError};
use super::llm_provider::{
    build_rag_system_prompt, build_rag_user_prompt, create_llm_provider, extract_citations,
    ChatMessage, LlmError,
};
use crate::app::{AppState, DieselCachedRepo};
use crate::models::{RagAnswerResponse, SemanticSearchResult};
use crate::repository::errors::RepoError;
use crate::schema::{requirement_version_verification_methods, requirement_versions, requirements};
use crate::services::DecoratedRequirementService;
use diesel::prelude::*;
use diesel::sql_types::{Float4, Integer, Text};
use std::collections::HashMap;

/// Search filters for semantic search.
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub status_id: Option<i32>,
    pub category_id: Option<i32>,
    pub applicability_id: Option<i32>,
    pub verification_id: Option<i32>,
}

/// Semantic search service.
pub struct SemanticSearchService<'a> {
    state: &'a AppState<DieselCachedRepo>,
    config: SemanticSearchConfig,
}

impl<'a> SemanticSearchService<'a> {
    /// Create a new search service.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self {
            state,
            config: SemanticSearchConfig::global().clone(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        state: &'a AppState<DieselCachedRepo>,
        config: SemanticSearchConfig,
    ) -> Self {
        Self { state, config }
    }

    /// Check if semantic search is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.embeddings_enabled && self.config.is_valid_for_embeddings().is_ok()
    }

    /// Check if RAG is enabled.
    pub fn is_rag_enabled(&self) -> bool {
        self.config.rag_enabled && self.config.is_valid_for_rag().is_ok()
    }

    /// Perform hybrid semantic search.
    ///
    /// Combines lexical (full-text) and dense (vector) search using RRF.
    pub async fn search(
        &self,
        project_id: i32,
        query: &str,
        filters: &SearchFilters,
        k: usize,
    ) -> Result<Vec<SemanticSearchResult>, SearchError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(vec![]);
        }

        // Check for exact reference code match first
        if let Some(exact_match) = self.exact_reference_match(project_id, query, filters)? {
            return Ok(vec![exact_match]);
        }

        // Perform lexical search
        let lexical_results = self.lexical_search(project_id, query, filters, k * 2)?;

        // Perform vector search if embeddings are enabled
        let vector_results = if self.is_enabled() {
            self.vector_search(project_id, query, filters, k * 2)
                .await?
        } else {
            vec![]
        };

        // Combine using RRF
        let fused = self.reciprocal_rank_fusion(&lexical_results, &vector_results, k);

        // Enrich with decorated data
        self.enrich_results(fused)
    }

    /// Check for exact reference code match.
    fn exact_reference_match(
        &self,
        project_id: i32,
        query: &str,
        filters: &SearchFilters,
    ) -> Result<Option<SemanticSearchResult>, SearchError> {
        // Check if query looks like a reference code (e.g., REQ-001, SYS-PERF-002)
        let query_upper = query.to_uppercase();
        if !query_upper.contains('-')
            || query_upper.len() < 3
            || !query_upper.chars().any(|c| c.is_ascii_digit())
        {
            return Ok(None);
        }

        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn().map_err(SearchError::Repo)?;

        // Build query: join requirements with current version; optional verification via subquery
        let mut query_builder = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::project_id.eq(project_id))
            .filter(requirements::stable_code.ilike(&query_upper))
            .into_boxed();
        if let Some(verification_id) = filters.verification_id {
            let subquery = requirement_version_verification_methods::table
                .filter(
                    requirement_version_verification_methods::verification_method_id
                        .eq(verification_id),
                )
                .select(requirement_version_verification_methods::requirement_version_id);
            query_builder = query_builder.filter(requirement_versions::id.eq_any(subquery));
        }

        if let Some(status_id) = filters.status_id {
            query_builder = query_builder.filter(requirement_versions::status_id.eq(status_id));
        }
        if let Some(category_id) = filters.category_id {
            query_builder = query_builder.filter(requirement_versions::category_id.eq(category_id));
        }
        if let Some(applicability_id) = filters.applicability_id {
            query_builder =
                query_builder.filter(requirement_versions::applicability_id.eq(applicability_id));
        }

        let result: Option<(i32, String, String, String)> = query_builder
            .select((
                requirements::id,
                requirements::stable_code,
                requirement_versions::title,
                requirement_versions::description,
            ))
            .first(conn.as_mut())
            .optional()
            .map_err(|e| SearchError::Repo(RepoError::Db(e)))?;

        if let Some((id, reference_code, title, description)) = result {
            // Get decorated version for full metadata
            let decorated = DecoratedRequirementService::new(self.state)
                .get_by_id(id)
                .map_err(SearchError::Repo)?;

            return Ok(Some(SemanticSearchResult {
                id,
                reference_code,
                title,
                description: description.clone(),
                snippet: truncate_snippet(&description, 200),
                score: 1.0,
                rank: 1,
                lexical_rank: Some(1),
                vector_rank: None,
                status: decorated.status_id,
                category: decorated.category_id,
                applicability: decorated.applicability_id,
                verification: decorated.verification_method_id,
            }));
        }

        Ok(None)
    }

    /// Perform lexical full-text search.
    fn lexical_search(
        &self,
        project_id: i32,
        query: &str,
        filters: &SearchFilters,
        limit: usize,
    ) -> Result<Vec<(i32, f32)>, SearchError> {
        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn().map_err(SearchError::Repo)?;

        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        // Build OR-based tsquery for better recall with natural language questions
        // Extract meaningful words (skip common stop words and short terms)
        let stop_words = [
            "does", "is", "are", "the", "a", "an", "and", "or", "but", "in", "on", "at", "to",
            "for",
        ];
        let ts_query: String = query
            .split_whitespace()
            .filter(|w| w.len() > 2) // Skip very short words
            .filter(|w| !stop_words.contains(&w.to_lowercase().as_str()))
            .map(|w| {
                // Remove all non-alphanumeric characters from both ends
                let cleaned = w.trim_matches(|c: char| !c.is_alphanumeric());
                if cleaned.is_empty() {
                    String::new()
                } else {
                    // Escape single quotes and add prefix matching
                    format!("{}:*", cleaned.replace("'", "''").to_lowercase())
                }
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" | "); // Use OR (|) instead of AND (&) for better recall

        if ts_query.is_empty() {
            return Ok(vec![]);
        }

        // Execute with parameters using to_tsquery with OR logic.
        // Search uses requirement_versions.search_vector (current version content); return requirement id.
        let results: Vec<(i32, f32)> = diesel::sql_query(
            r#"
            SELECT
                r.id,
                ts_rank_cd(rv.search_vector, to_tsquery('english', $1)) AS rank
            FROM requirements r
            INNER JOIN requirement_versions rv ON r.current_version_id = rv.id
            WHERE r.project_id = $2
                AND rv.search_vector @@ to_tsquery('english', $1)
            ORDER BY rank DESC
            LIMIT $3
            "#,
        )
        .bind::<Text, _>(&ts_query)
        .bind::<Integer, _>(project_id)
        .bind::<Integer, _>(limit as i32)
        .load::<LexicalResult>(conn.as_mut())
        .map_err(|e| SearchError::Repo(RepoError::Db(e)))?
        .into_iter()
        .map(|r| (r.id, r.rank))
        .collect();

        // Apply filters in Rust since the dynamic SQL binding is complex
        if filters.status_id.is_some()
            || filters.category_id.is_some()
            || filters.applicability_id.is_some()
            || filters.verification_id.is_some()
        {
            return self.filter_results(results, filters);
        }

        Ok(results)
    }

    /// Filter results by additional criteria.
    fn filter_results(
        &self,
        results: Vec<(i32, f32)>,
        filters: &SearchFilters,
    ) -> Result<Vec<(i32, f32)>, SearchError> {
        if results.is_empty() {
            return Ok(vec![]);
        }

        let ids: Vec<i32> = results.iter().map(|(id, _)| *id).collect();
        let scores: HashMap<i32, f32> = results.into_iter().collect();

        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn().map_err(SearchError::Repo)?;

        let mut query = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::id.eq_any(&ids))
            .into_boxed();
        if let Some(verification_id) = filters.verification_id {
            let subquery = requirement_version_verification_methods::table
                .filter(
                    requirement_version_verification_methods::verification_method_id
                        .eq(verification_id),
                )
                .select(requirement_version_verification_methods::requirement_version_id);
            query = query.filter(requirement_versions::id.eq_any(subquery));
        }

        if let Some(status_id) = filters.status_id {
            query = query.filter(requirement_versions::status_id.eq(status_id));
        }
        if let Some(category_id) = filters.category_id {
            query = query.filter(requirement_versions::category_id.eq(category_id));
        }
        if let Some(applicability_id) = filters.applicability_id {
            query = query.filter(requirement_versions::applicability_id.eq(applicability_id));
        }

        let filtered_ids: Vec<i32> = query
            .select(requirements::id)
            .load(conn.as_mut())
            .map_err(|e| SearchError::Repo(RepoError::Db(e)))?;

        Ok(filtered_ids
            .into_iter()
            .filter_map(|id| scores.get(&id).map(|s| (id, *s)))
            .collect())
    }

    /// Perform vector similarity search.
    async fn vector_search(
        &self,
        project_id: i32,
        query: &str,
        filters: &SearchFilters,
        limit: usize,
    ) -> Result<Vec<(i32, f32)>, SearchError> {
        // Generate query embedding
        let provider = create_embedding_provider(&self.config).map_err(SearchError::Embedding)?;
        let query_embedding = provider
            .embed(query)
            .await
            .map_err(SearchError::Embedding)?;

        let repo = self.state.repo_read();
        let mut conn = repo.inner_repo().get_conn().map_err(SearchError::Repo)?;

        // Convert to pgvector format for query
        let embedding_str = format!(
            "[{}]",
            query_embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // Vector search using cosine distance
        let results: Vec<(i32, f32)> = diesel::sql_query(
            r#"
            SELECT 
                re.requirement_id as id,
                (1 - (re.embedding <=> $1::vector)) as similarity
            FROM requirement_embeddings re
            JOIN requirements r ON r.id = re.requirement_id
            WHERE re.project_id = $2
                AND re.embedding IS NOT NULL
            ORDER BY re.embedding <=> $1::vector
            LIMIT $3
            "#,
        )
        .bind::<Text, _>(&embedding_str)
        .bind::<Integer, _>(project_id)
        .bind::<Integer, _>(limit as i32)
        .load::<VectorResult>(conn.as_mut())
        .map_err(|e| SearchError::Repo(RepoError::Db(e)))?
        .into_iter()
        .map(|r| (r.id, r.similarity as f32))
        .collect();

        // Apply filters
        if filters.status_id.is_some()
            || filters.category_id.is_some()
            || filters.applicability_id.is_some()
            || filters.verification_id.is_some()
        {
            return self.filter_results(results, filters);
        }

        Ok(results)
    }

    /// Combine results using Reciprocal Rank Fusion (RRF).
    ///
    /// RRF score = sum(1 / (k + rank)) across all result lists
    /// where k is a constant (typically 60) that dampens the contribution of high ranks.
    fn reciprocal_rank_fusion(
        &self,
        lexical: &[(i32, f32)],
        vector: &[(i32, f32)],
        k: usize,
    ) -> Vec<(i32, f32, Option<i32>, Option<i32>)> {
        // Delegate to standalone function for testability
        combine_results_rrf(lexical, vector, k)
    }

    /// Enrich results with full requirement data.
    fn enrich_results(
        &self,
        fused: Vec<(i32, f32, Option<i32>, Option<i32>)>,
    ) -> Result<Vec<SemanticSearchResult>, SearchError> {
        if fused.is_empty() {
            return Ok(vec![]);
        }

        let decorated_service = DecoratedRequirementService::new(self.state);
        let mut results = Vec::with_capacity(fused.len());

        for (rank, (id, score, lexical_rank, vector_rank)) in fused.into_iter().enumerate() {
            let req = match decorated_service.get_by_id(id) {
                Ok(r) => r,
                Err(_) => continue, // Skip if requirement not found
            };

            results.push(SemanticSearchResult {
                id: req.id,
                reference_code: req.reference_code,
                title: req.title,
                description: req.description.clone(),
                snippet: truncate_snippet(&req.description, 200),
                score,
                rank: (rank + 1) as i32,
                lexical_rank,
                vector_rank,
                status: req.status_id,
                category: req.category_id,
                applicability: req.applicability_id,
                verification: req.verification_method_id,
            });
        }

        Ok(results)
    }

    /// Generate a RAG answer from search results.
    pub async fn ask(
        &self,
        project_id: i32,
        query: &str,
        filters: &SearchFilters,
        k: usize,
    ) -> Result<RagAnswerResponse, SearchError> {
        if !self.is_rag_enabled() {
            return Err(SearchError::NotConfigured(
                "RAG is disabled. Set RAG_ENABLED=true".into(),
            ));
        }

        // Get search results
        let results = self.search(project_id, query, filters, k).await?;

        if results.is_empty() {
            return Ok(RagAnswerResponse {
                answer: "No relevant requirements found to answer this question.".into(),
                citations: vec![],
                results: vec![],
            });
        }

        // Build prompts
        let system_prompt = build_rag_system_prompt();
        let user_prompt = build_rag_user_prompt(query, &results);

        // Generate answer
        let llm = create_llm_provider(&self.config).map_err(SearchError::Llm)?;
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];

        let answer = llm
            .chat(&messages, self.config.rag_max_tokens)
            .await
            .map_err(SearchError::Llm)?;

        // Extract citations
        let citations = extract_citations(&answer, &results);

        Ok(RagAnswerResponse {
            answer,
            citations,
            results,
        })
    }
}

/// Search error type.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
    #[error("Not configured: {0}")]
    NotConfigured(String),
}

/// Result row for lexical search.
#[derive(QueryableByName)]
struct LexicalResult {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Float4)]
    rank: f32,
}

/// Result row for vector search.
#[derive(QueryableByName)]
struct VectorResult {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = diesel::sql_types::Double)]
    similarity: f64,
}

/// Truncate text to create a snippet.
fn truncate_snippet(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    // Find a good breaking point
    let truncated = &text[..max_len];
    if let Some(last_space) = truncated.rfind(|c: char| c.is_whitespace()) {
        format!("{}...", &text[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

/// RRF constant for damping rank contributions.
const RRF_K: f32 = 60.0;

/// Calculate RRF score for a given rank.
/// Uses the formula: 1 / (k + rank) where k=60
pub fn calculate_rrf_score(rank: usize) -> f32 {
    1.0 / (RRF_K + rank as f32)
}

/// Combine search results using Reciprocal Rank Fusion (RRF).
///
/// RRF score = sum(1 / (k + rank)) across all result lists
/// where k is a constant (60) that dampens the contribution of high ranks.
///
/// Returns (id, combined_score, lexical_rank, vector_rank) sorted by score descending.
pub fn combine_results_rrf(
    lexical: &[(i32, f32)],
    vector: &[(i32, f32)],
    k: usize,
) -> Vec<(i32, f32, Option<i32>, Option<i32>)> {
    let mut scores: HashMap<i32, (f32, Option<i32>, Option<i32>)> = HashMap::new();

    // Add lexical scores
    for (rank, (id, _score)) in lexical.iter().enumerate() {
        let rrf_score = calculate_rrf_score(rank + 1);
        let entry = scores.entry(*id).or_insert((0.0, None, None));
        entry.0 += rrf_score;
        entry.1 = Some((rank + 1) as i32);
    }

    // Add vector scores
    for (rank, (id, _score)) in vector.iter().enumerate() {
        let rrf_score = calculate_rrf_score(rank + 1);
        let entry = scores.entry(*id).or_insert((0.0, None, None));
        entry.0 += rrf_score;
        entry.2 = Some((rank + 1) as i32);
    }

    // Sort by combined RRF score
    let mut results: Vec<_> = scores
        .into_iter()
        .map(|(id, (score, lex_rank, vec_rank))| (id, score, lex_rank, vec_rank))
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top k
    results.truncate(k);
    results
}

/// Check if a query string looks like a reference code.
/// Reference codes typically contain a hyphen, are at least 3 chars,
/// and contain at least one digit.
pub fn looks_like_reference_code(query: &str) -> bool {
    let query_upper = query.to_uppercase();
    query_upper.contains('-')
        && query_upper.len() >= 3
        && query_upper.chars().any(|c| c.is_ascii_digit())
}

/// Build OR-based tsquery from natural language query.
/// Extracts meaningful words, skips stop words and short terms.
pub fn build_tsquery(query: &str) -> String {
    const STOP_WORDS: [&str; 14] = [
        "does", "is", "are", "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
    ];

    query
        .split_whitespace()
        .filter(|w| w.len() > 2) // Skip very short words
        .filter(|w| !STOP_WORDS.contains(&w.to_lowercase().as_str()))
        .map(|w| {
            // Remove all non-alphanumeric characters from both ends
            let cleaned = w.trim_matches(|c: char| !c.is_alphanumeric());
            if cleaned.is_empty() {
                String::new()
            } else {
                // Escape single quotes and add prefix matching
                format!("{}:*", cleaned.replace('\'', "''").to_lowercase())
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_snippet_short_text() {
        let text = "Short text";
        assert_eq!(truncate_snippet(text, 100), "Short text");
    }

    #[test]
    fn truncate_snippet_long_text() {
        let text = "This is a longer text that should be truncated at a word boundary";
        let result = truncate_snippet(text, 30);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 33); // 30 + "..."
    }

    #[test]
    fn truncate_snippet_no_word_boundary() {
        // Text without spaces - should truncate at exact position
        let text = "abcdefghijklmnopqrstuvwxyz";
        let result = truncate_snippet(text, 10);
        assert_eq!(result, "abcdefghij...");
    }

    #[test]
    fn truncate_snippet_exact_length() {
        let text = "Exactly ten";
        let result = truncate_snippet(text, 11);
        assert_eq!(result, "Exactly ten");
    }

    #[test]
    fn truncate_snippet_empty_text() {
        let text = "";
        let result = truncate_snippet(text, 100);
        assert_eq!(result, "");
    }

    #[test]
    fn truncate_snippet_single_word_longer_than_max() {
        let text = "superlongwordwithoutanyspaces and more";
        let result = truncate_snippet(text, 15);
        // Should truncate at word boundary after "superlongwordwithoutanyspaces"
        // but since first word is longer than max, falls back to exact truncation
        assert!(result.ends_with("..."));
    }

    #[test]
    fn rrf_fusion_combines_scores() {
        // RRF formula: 1/(k + rank) where k=60
        // rank 1: 1/61 ≈ 0.0164
        // rank 2: 1/62 ≈ 0.0161
        let rrf_1: f32 = 1.0 / 61.0;
        let _rrf_2: f32 = 1.0 / 62.0;

        // If item appears in both lists at rank 1, score = 2 * 1/61
        let combined = rrf_1 + rrf_1;
        assert!((combined - 0.0328_f32).abs() < 0.001);
    }

    #[test]
    fn rrf_formula_decreases_with_rank() {
        // Higher ranks should contribute less to the score
        let rrf_1: f32 = 1.0 / 61.0;
        let rrf_10: f32 = 1.0 / 70.0;
        let rrf_100: f32 = 1.0 / 160.0;

        assert!(rrf_1 > rrf_10);
        assert!(rrf_10 > rrf_100);
    }

    #[test]
    fn rrf_k_constant_dampens_rank_differences() {
        // With k=60, the difference between rank 1 and 2 is small
        let rrf_1: f32 = 1.0 / 61.0;
        let rrf_2: f32 = 1.0 / 62.0;
        let ratio = rrf_1 / rrf_2;

        // Should be close to 1 (about 1.016)
        assert!((ratio - 1.016).abs() < 0.01);
    }

    #[test]
    fn search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.status_id.is_none());
        assert!(filters.category_id.is_none());
        assert!(filters.applicability_id.is_none());
        assert!(filters.verification_id.is_none());
    }

    #[test]
    fn search_filters_with_values() {
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
            applicability_id: Some(3),
            verification_id: None,
        };
        assert!(filters.status_id.is_some());
        assert!(filters.category_id.is_none());
        assert!(filters.applicability_id.is_some());
        assert!(filters.verification_id.is_none());
    }

    #[test]
    fn search_error_display() {
        let repo_err = SearchError::NotConfigured("test error".into());
        assert!(repo_err.to_string().contains("test error"));
    }

    #[test]
    fn search_error_embedding_variant() {
        let emb_err = EmbeddingError::NotConfigured("not configured".into());
        let search_err = SearchError::Embedding(emb_err);
        assert!(search_err.to_string().contains("Embedding error"));
    }

    #[test]
    fn search_error_llm_variant() {
        let llm_err = LlmError::NotConfigured("not configured".into());
        let search_err = SearchError::Llm(llm_err);
        assert!(search_err.to_string().contains("LLM error"));
    }

    #[test]
    fn lexical_result_queryable() {
        // This test ensures the LexicalResult struct is properly defined
        // It's used for SQL queries and must have the correct types
        let result = LexicalResult { id: 1, rank: 0.5 };
        assert_eq!(result.id, 1);
        assert!((result.rank - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn vector_result_queryable() {
        // This test ensures the VectorResult struct is properly defined
        let result = VectorResult {
            id: 42,
            similarity: 0.95,
        };
        assert_eq!(result.id, 42);
        assert!((result.similarity - 0.95).abs() < f64::EPSILON);
    }

    // Tests for calculate_rrf_score
    #[test]
    fn calculate_rrf_score_rank_1() {
        let score = calculate_rrf_score(1);
        let expected = 1.0 / 61.0;
        assert!((score - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn calculate_rrf_score_rank_10() {
        let score = calculate_rrf_score(10);
        let expected = 1.0 / 70.0;
        assert!((score - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn calculate_rrf_score_decreases_with_rank() {
        let score_1 = calculate_rrf_score(1);
        let score_5 = calculate_rrf_score(5);
        let score_10 = calculate_rrf_score(10);
        let score_100 = calculate_rrf_score(100);

        assert!(score_1 > score_5);
        assert!(score_5 > score_10);
        assert!(score_10 > score_100);
    }

    // Tests for combine_results_rrf
    #[test]
    fn combine_results_rrf_empty_inputs() {
        let result = combine_results_rrf(&[], &[], 10);
        assert!(result.is_empty());
    }

    #[test]
    fn combine_results_rrf_only_lexical() {
        let lexical = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let result = combine_results_rrf(&lexical, &[], 10);

        assert_eq!(result.len(), 3);
        // First item should be id=1 with lexical_rank=1, vector_rank=None
        assert_eq!(result[0].0, 1); // id
        assert_eq!(result[0].2, Some(1)); // lexical_rank
        assert!(result[0].3.is_none()); // vector_rank
    }

    #[test]
    fn combine_results_rrf_only_vector() {
        let vector = vec![(4, 0.95), (5, 0.85)];
        let result = combine_results_rrf(&[], &vector, 10);

        assert_eq!(result.len(), 2);
        // First item should be id=4 with vector_rank=1, lexical_rank=None
        assert_eq!(result[0].0, 4);
        assert!(result[0].2.is_none()); // lexical_rank
        assert_eq!(result[0].3, Some(1)); // vector_rank
    }

    #[test]
    fn combine_results_rrf_overlapping_results() {
        // Same item appears in both lists - should have higher combined score
        let lexical = vec![(1, 0.9), (2, 0.8)];
        let vector = vec![(1, 0.95), (3, 0.75)];
        let result = combine_results_rrf(&lexical, &vector, 10);

        assert_eq!(result.len(), 3);
        // id=1 should be first because it appears in both lists
        assert_eq!(result[0].0, 1);
        assert_eq!(result[0].2, Some(1)); // lexical_rank
        assert_eq!(result[0].3, Some(1)); // vector_rank

        // Score should be sum of both RRF contributions
        let expected_score = calculate_rrf_score(1) + calculate_rrf_score(1);
        assert!((result[0].1 - expected_score).abs() < 0.001);
    }

    #[test]
    fn combine_results_rrf_respects_k_limit() {
        let lexical = vec![(1, 0.9), (2, 0.8), (3, 0.7), (4, 0.6), (5, 0.5)];
        let result = combine_results_rrf(&lexical, &[], 3);

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn combine_results_rrf_sorted_by_score() {
        let lexical = vec![(3, 0.7)];
        let vector = vec![(1, 0.95), (2, 0.85)];
        let result = combine_results_rrf(&lexical, &vector, 10);

        // Check that results are sorted by score descending
        for i in 1..result.len() {
            assert!(result[i - 1].1 >= result[i].1);
        }
    }

    #[test]
    fn combine_results_rrf_complex_overlap() {
        let lexical = vec![(1, 0.9), (2, 0.8), (3, 0.7)];
        let vector = vec![(2, 0.95), (4, 0.85), (1, 0.75)];
        let result = combine_results_rrf(&lexical, &vector, 10);

        assert_eq!(result.len(), 4);

        // id=1: lexical rank 1, vector rank 3
        // id=2: lexical rank 2, vector rank 1
        // Both should have high scores due to overlap

        // Find id=2 in results - should be first because (rank 2, rank 1) beats (rank 1, rank 3)
        let id2_result = result.iter().find(|r| r.0 == 2).unwrap();
        assert_eq!(id2_result.2, Some(2)); // lexical rank
        assert_eq!(id2_result.3, Some(1)); // vector rank
    }

    // Tests for looks_like_reference_code
    #[test]
    fn looks_like_reference_code_valid() {
        assert!(looks_like_reference_code("REQ-001"));
        assert!(looks_like_reference_code("SYS-PERF-002"));
        assert!(looks_like_reference_code("req-1")); // lowercase also works
        assert!(looks_like_reference_code("A-1"));
    }

    #[test]
    fn looks_like_reference_code_invalid() {
        assert!(!looks_like_reference_code("REQ")); // No hyphen
        assert!(!looks_like_reference_code("REQ-")); // No digit
        assert!(!looks_like_reference_code("123")); // No hyphen
        assert!(!looks_like_reference_code("AB")); // Too short
        assert!(!looks_like_reference_code("")); // Empty
    }

    #[test]
    fn looks_like_reference_code_edge_cases() {
        assert!(looks_like_reference_code("X-1")); // Minimum valid
        assert!(!looks_like_reference_code("-1")); // No letter before hyphen
        assert!(looks_like_reference_code("REQ-ABC-123")); // Multiple hyphens
    }

    // Tests for build_tsquery
    #[test]
    fn build_tsquery_simple() {
        let result = build_tsquery("system performance");
        assert!(result.contains("system:*"));
        assert!(result.contains("performance:*"));
        assert!(result.contains(" | "));
    }

    #[test]
    fn build_tsquery_filters_stop_words() {
        let result = build_tsquery("the system is fast");
        assert!(!result.contains("the:*"));
        assert!(!result.contains("is:*"));
        assert!(result.contains("system:*"));
        assert!(result.contains("fast:*"));
    }

    #[test]
    fn build_tsquery_filters_short_words() {
        let result = build_tsquery("a to be");
        // All words are 2 chars or less, so should be filtered
        assert!(result.is_empty());
    }

    #[test]
    fn build_tsquery_handles_punctuation() {
        let result = build_tsquery("system's performance, good!");
        // Apostrophe inside word is escaped (doubled for SQL), punctuation at word boundaries is trimmed
        // "system's" becomes "system''s" (apostrophe escaped)
        assert!(result.contains("system''s:*"));
        assert!(result.contains("performance:*"));
        assert!(result.contains("good:*"));
    }

    #[test]
    fn build_tsquery_empty() {
        let result = build_tsquery("");
        assert!(result.is_empty());
    }

    #[test]
    fn build_tsquery_only_stop_words() {
        let result = build_tsquery("the a an and or");
        assert!(result.is_empty());
    }

    #[test]
    fn build_tsquery_preserves_case_insensitive() {
        let result = build_tsquery("SYSTEM Performance");
        assert!(result.contains("system:*"));
        assert!(result.contains("performance:*"));
    }

    #[test]
    fn build_tsquery_handles_special_chars() {
        let result = build_tsquery("(system) [performance]");
        // Brackets should be stripped
        assert!(result.contains("system:*"));
        assert!(result.contains("performance:*"));
    }

    #[test]
    fn semantic_search_service_with_config() {
        use crate::app::AppState;
        use crate::repository::diesel_repo_mock::DieselRepoMock;
        use crate::repository::CacheRepository;
        use std::sync::{Arc, RwLock};

        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                0,
            ))),
        };
        let config = crate::services::semantic_search::config::SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            ..Default::default()
        };
        let service = SemanticSearchService::with_config(&state, config);
        assert!(service.is_enabled());
        assert!(!service.is_rag_enabled());
    }

    #[test]
    fn semantic_search_service_rag_enabled_with_config() {
        use crate::app::AppState;
        use crate::repository::diesel_repo_mock::DieselRepoMock;
        use crate::repository::CacheRepository;
        use std::sync::{Arc, RwLock};

        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                0,
            ))),
        };
        let config = crate::services::semantic_search::config::SemanticSearchConfig {
            embeddings_enabled: true,
            embedding_provider: "mock".into(),
            rag_enabled: true,
            ..Default::default()
        };
        let service = SemanticSearchService::with_config(&state, config);
        assert!(service.is_enabled());
        assert!(service.is_rag_enabled());
    }
}
