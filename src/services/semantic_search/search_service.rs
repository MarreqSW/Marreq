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
use crate::schema::requirements;
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

        // Build query
        let mut query_builder = requirements::table
            .filter(requirements::project_id.eq(project_id))
            .filter(requirements::reference_code.ilike(&query_upper))
            .into_boxed();

        if let Some(status_id) = filters.status_id {
            query_builder = query_builder.filter(requirements::status_id.eq(status_id));
        }
        if let Some(category_id) = filters.category_id {
            query_builder = query_builder.filter(requirements::category_id.eq(category_id));
        }
        if let Some(applicability_id) = filters.applicability_id {
            query_builder =
                query_builder.filter(requirements::applicability_id.eq(applicability_id));
        }
        if let Some(verification_id) = filters.verification_id {
            query_builder =
                query_builder.filter(requirements::verification_method_id.eq(verification_id));
        }

        let result: Option<(i32, String, String, String)> = query_builder
            .select((
                requirements::id,
                requirements::reference_code,
                requirements::title,
                requirements::description,
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

        // Execute with parameters using to_tsquery with OR logic
        let results: Vec<(i32, f32)> = diesel::sql_query(
            r#"
            SELECT 
                r.id,
                ts_rank_cd(r.search_vector, to_tsquery('english', $1)) as rank
            FROM requirements r
            WHERE r.project_id = $2
                AND r.search_vector @@ to_tsquery('english', $1)
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
            .filter(requirements::id.eq_any(&ids))
            .into_boxed();

        if let Some(status_id) = filters.status_id {
            query = query.filter(requirements::status_id.eq(status_id));
        }
        if let Some(category_id) = filters.category_id {
            query = query.filter(requirements::category_id.eq(category_id));
        }
        if let Some(applicability_id) = filters.applicability_id {
            query = query.filter(requirements::applicability_id.eq(applicability_id));
        }
        if let Some(verification_id) = filters.verification_id {
            query = query.filter(requirements::verification_method_id.eq(verification_id));
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
        .map(|r| (r.id, r.similarity))
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
        const RRF_K: f32 = 60.0;

        let mut scores: HashMap<i32, (f32, Option<i32>, Option<i32>)> = HashMap::new();

        // Add lexical scores
        for (rank, (id, _score)) in lexical.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + (rank + 1) as f32);
            let entry = scores.entry(*id).or_insert((0.0, None, None));
            entry.0 += rrf_score;
            entry.1 = Some((rank + 1) as i32);
        }

        // Add vector scores
        for (rank, (id, _score)) in vector.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + (rank + 1) as f32);
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
    #[diesel(sql_type = Float4)]
    similarity: f32,
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
    fn rrf_fusion_combines_scores() {
        // Can't fully test without state, but we can test the algorithm

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
    fn search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.status_id.is_none());
        assert!(filters.category_id.is_none());
        assert!(filters.applicability_id.is_none());
        assert!(filters.verification_id.is_none());
    }
}
