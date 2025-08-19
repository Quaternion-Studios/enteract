use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::search::{SearchService, SearchResult};
use super::embedding::SimpleEmbeddingService;
use crate::rag::enhanced::system::{EnhancedDocument, EnhancedDocumentChunk};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdvancedSearchQuery {
    pub text: String,
    pub filters: SearchFilters,
    pub ranking_options: RankingOptions,
    pub search_modes: Vec<SearchMode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchFilters {
    pub file_types: Vec<String>,
    pub date_range: Option<DateRange>,
    pub tags: Vec<String>,
    pub min_size: Option<i64>,
    pub max_size: Option<i64>,
    pub has_embeddings: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRange {
    pub from: String, // ISO 8601 format
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RankingOptions {
    pub semantic_weight: f32,      // 0.0 - 1.0
    pub keyword_weight: f32,       // 0.0 - 1.0
    pub recency_weight: f32,       // 0.0 - 1.0
    pub usage_weight: f32,         // 0.0 - 1.0
    pub boost_exact_matches: bool,
    pub penalize_duplicates: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SearchMode {
    Semantic,    // Vector-based semantic search
    Keyword,     // BM25/keyword search
    Fuzzy,       // Fuzzy string matching
    Metadata,    // Search in metadata fields
    Hybrid,      // Combination of semantic + keyword
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdvancedSearchResult {
    pub document: EnhancedDocument,
    pub chunks: Vec<EnhancedDocumentChunk>,
    pub relevance_score: f32,
    pub rank_breakdown: RankBreakdown,
    pub highlights: Vec<SearchHighlight>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RankBreakdown {
    pub semantic_score: f32,
    pub keyword_score: f32,
    pub recency_score: f32,
    pub usage_score: f32,
    pub total_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchHighlight {
    pub field: String,           // "content", "title", "metadata"
    pub text: String,           // highlighted text
    pub start_offset: usize,
    pub end_offset: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchSuggestion {
    pub query: String,
    pub type_: String,  // "correction", "expansion", "related"
    pub confidence: f32,
}

pub struct AdvancedSearchService {
    base_search: Arc<SearchService>,
    embedding_service: Arc<SimpleEmbeddingService>,
    query_cache: Arc<tokio::sync::Mutex<HashMap<String, Vec<AdvancedSearchResult>>>>,
}

impl AdvancedSearchService {
    pub fn new(
        search_service: Arc<SearchService>,
        embedding_service: Arc<SimpleEmbeddingService>,
    ) -> Self {
        Self {
            base_search: search_service,
            embedding_service,
            query_cache: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Perform advanced search with multiple modes and ranking
    pub async fn search_advanced(&self, query: AdvancedSearchQuery) -> Result<Vec<AdvancedSearchResult>> {
        // Check cache first
        let cache_key = self.generate_cache_key(&query);
        {
            let cache = self.query_cache.lock().await;
            if let Some(cached_results) = cache.get(&cache_key) {
                return Ok(cached_results.clone());
            }
        }

        let mut all_results = Vec::new();

        // Execute different search modes
        for mode in &query.search_modes {
            let mode_results = match mode {
                SearchMode::Semantic => self.semantic_search(&query).await?,
                SearchMode::Keyword => self.keyword_search(&query).await?,
                SearchMode::Fuzzy => self.fuzzy_search(&query).await?,
                SearchMode::Metadata => self.metadata_search(&query).await?,
                SearchMode::Hybrid => self.hybrid_search(&query).await?,
            };
            all_results.extend(mode_results);
        }

        // Combine and rank results
        let final_results = self.combine_and_rank_results(all_results, &query.ranking_options).await?;

        // Apply filters
        let filtered_results = self.apply_filters(final_results, &query.filters).await?;

        // Generate highlights
        let highlighted_results = self.add_highlights(filtered_results, &query.text).await?;

        // Cache results
        {
            let mut cache = self.query_cache.lock().await;
            cache.insert(cache_key, highlighted_results.clone());
            
            // Limit cache size
            if cache.len() > 100 {
                let oldest_key = cache.keys().next().unwrap().clone();
                cache.remove(&oldest_key);
            }
        }

        Ok(highlighted_results)
    }

    /// Semantic search using embeddings
    async fn semantic_search(&self, query: &AdvancedSearchQuery) -> Result<Vec<AdvancedSearchResult>> {
        // Generate query embedding
        let query_embedding = vec![self.embedding_service.embed_query(&query.text)?];
        if query_embedding.is_empty() {
            return Ok(Vec::new());
        }

        // Use base search service for now (would need to implement vector search)
        let search_results = self.base_search.search_bm25(&query.text, 50)?;
        
        let mut results = Vec::new();
        for result in search_results {
            // Mock document creation (in real implementation, fetch from database)
            let document = self.create_mock_document_from_result(&result);
            
            results.push(AdvancedSearchResult {
                document,
                chunks: Vec::new(), // Would be populated from actual chunks
                relevance_score: result.score,
                rank_breakdown: RankBreakdown {
                    semantic_score: result.score,
                    keyword_score: 0.0,
                    recency_score: 0.0,
                    usage_score: 0.0,
                    total_score: result.score,
                },
                highlights: Vec::new(),
            });
        }

        Ok(results)
    }

    /// Keyword-based search
    async fn keyword_search(&self, query: &AdvancedSearchQuery) -> Result<Vec<AdvancedSearchResult>> {
        let search_results = self.base_search.search_bm25(&query.text, 50)?;
        
        let mut results = Vec::new();
        for result in search_results {
            let document = self.create_mock_document_from_result(&result);
            
            // Calculate keyword score based on term frequency
            let keyword_score = self.calculate_keyword_score(&query.text, &result.content);
            
            results.push(AdvancedSearchResult {
                document,
                chunks: Vec::new(),
                relevance_score: keyword_score,
                rank_breakdown: RankBreakdown {
                    semantic_score: 0.0,
                    keyword_score,
                    recency_score: 0.0,
                    usage_score: 0.0,
                    total_score: keyword_score,
                },
                highlights: Vec::new(),
            });
        }

        Ok(results)
    }

    /// Fuzzy search for typo tolerance
    async fn fuzzy_search(&self, query: &AdvancedSearchQuery) -> Result<Vec<AdvancedSearchResult>> {
        // Simple fuzzy matching implementation
        let search_results = self.base_search.search_bm25(&query.text, 50)?;
        
        let mut results = Vec::new();
        for result in search_results {
            let document = self.create_mock_document_from_result(&result);
            
            // Calculate fuzzy score
            let fuzzy_score = self.calculate_fuzzy_score(&query.text, &result.content);
            
            results.push(AdvancedSearchResult {
                document,
                chunks: Vec::new(),
                relevance_score: fuzzy_score,
                rank_breakdown: RankBreakdown {
                    semantic_score: 0.0,
                    keyword_score: fuzzy_score,
                    recency_score: 0.0,
                    usage_score: 0.0,
                    total_score: fuzzy_score,
                },
                highlights: Vec::new(),
            });
        }

        Ok(results)
    }

    /// Search in metadata fields
    async fn metadata_search(&self, query: &AdvancedSearchQuery) -> Result<Vec<AdvancedSearchResult>> {
        // Mock implementation - would search in document metadata
        Ok(Vec::new())
    }

    /// Hybrid search combining semantic and keyword
    async fn hybrid_search(&self, query: &AdvancedSearchQuery) -> Result<Vec<AdvancedSearchResult>> {
        let semantic_results = self.semantic_search(query).await?;
        let keyword_results = self.keyword_search(query).await?;
        
        // Combine results with weighted scores
        let mut combined = semantic_results;
        combined.extend(keyword_results);
        
        Ok(combined)
    }

    /// Combine and rank results from different search modes
    async fn combine_and_rank_results(
        &self,
        mut results: Vec<AdvancedSearchResult>,
        ranking: &RankingOptions,
    ) -> Result<Vec<AdvancedSearchResult>> {
        // Remove duplicates
        if ranking.penalize_duplicates {
            results.sort_by(|a, b| a.document.id.cmp(&b.document.id));
            results.dedup_by(|a, b| a.document.id == b.document.id);
        }

        // Recalculate scores with weights
        for result in &mut results {
            let breakdown = &mut result.rank_breakdown;
            
            // Calculate recency score
            breakdown.recency_score = self.calculate_recency_score(&result.document);
            
            // Calculate usage score
            breakdown.usage_score = self.calculate_usage_score(&result.document);
            
            // Calculate weighted total
            breakdown.total_score = 
                breakdown.semantic_score * ranking.semantic_weight +
                breakdown.keyword_score * ranking.keyword_weight +
                breakdown.recency_score * ranking.recency_weight +
                breakdown.usage_score * ranking.usage_weight;
            
            result.relevance_score = breakdown.total_score;
        }

        // Sort by final score
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        Ok(results)
    }

    /// Apply search filters
    async fn apply_filters(
        &self,
        results: Vec<AdvancedSearchResult>,
        filters: &SearchFilters,
    ) -> Result<Vec<AdvancedSearchResult>> {
        let filtered: Vec<_> = results.into_iter().filter(|result| {
            // File type filter
            if !filters.file_types.is_empty() && 
               !filters.file_types.contains(&result.document.file_type) {
                return false;
            }

            // Size filters
            if let Some(min_size) = filters.min_size {
                if result.document.file_size < min_size {
                    return false;
                }
            }
            if let Some(max_size) = filters.max_size {
                if result.document.file_size > max_size {
                    return false;
                }
            }

            // Embedding filter
            if let Some(has_embeddings) = filters.has_embeddings {
                let has_emb = result.document.embedding_status == "completed";
                if has_embeddings != has_emb {
                    return false;
                }
            }

            true
        }).collect();

        Ok(filtered)
    }

    /// Add search highlights to results
    async fn add_highlights(
        &self,
        mut results: Vec<AdvancedSearchResult>,
        query: &str,
    ) -> Result<Vec<AdvancedSearchResult>> {
        let query_terms: Vec<&str> = query.split_whitespace().collect();
        
        for result in &mut results {
            let content = &result.document.content;
            let content_lower = content.to_lowercase();
            
            for term in &query_terms {
                let term_lower = term.to_lowercase();
                if let Some(pos) = content_lower.find(&term_lower) {
                    result.highlights.push(SearchHighlight {
                        field: "content".to_string(),
                        text: content[pos..pos + term.len()].to_string(),
                        start_offset: pos,
                        end_offset: pos + term.len(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Generate query suggestions for autocomplete
    pub async fn get_query_suggestions(&self, partial_query: &str) -> Result<Vec<SearchSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Simple word completion
        if partial_query.len() >= 2 {
            let common_terms = vec![
                "authentication", "database", "configuration", "implementation",
                "documentation", "testing", "deployment", "security", "performance"
            ];
            
            for term in &common_terms {
                if term.starts_with(&partial_query.to_lowercase()) {
                    suggestions.push(SearchSuggestion {
                        query: term.to_string(),
                        type_: "completion".to_string(),
                        confidence: 0.8,
                    });
                }
            }
        }

        Ok(suggestions.into_iter().take(5).collect())
    }

    // Helper methods
    
    fn generate_cache_key(&self, query: &AdvancedSearchQuery) -> String {
        format!("{:x}", md5::compute(serde_json::to_string(query).unwrap_or_default()))
    }

    fn create_mock_document_from_result(&self, result: &SearchResult) -> EnhancedDocument {
        EnhancedDocument {
            id: result.document_id.clone(),
            file_name: result.title.clone().unwrap_or_else(|| "Unknown".to_string()),
            file_path: String::new(),
            file_type: "text/plain".to_string(),
            file_size: result.content.len() as i64,
            content: result.content.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            access_count: 0,
            last_accessed: None,
            is_cached: true,
            embedding_status: "completed".to_string(),
            content_hash: Some(String::new()),
            chunk_count: 1,
            metadata: None,
        }
    }

    fn calculate_keyword_score(&self, query: &str, content: &str) -> f32 {
        let query_terms: Vec<&str> = query.split_whitespace().collect();
        let content_lower = content.to_lowercase();
        
        let mut score = 0.0;
        for term in query_terms {
            let term_lower = term.to_lowercase();
            let count = content_lower.matches(&term_lower).count();
            score += count as f32 * 0.1;
        }
        
        score.min(1.0)
    }

    fn calculate_fuzzy_score(&self, query: &str, content: &str) -> f32 {
        // Simple Levenshtein-based fuzzy scoring
        let content_words: Vec<&str> = content.split_whitespace().collect();
        let mut best_score: f32 = 0.0;
        
        for word in content_words {
            let distance = self.levenshtein_distance(query, word);
            let similarity = 1.0 - (distance as f32 / query.len().max(word.len()) as f32);
            best_score = best_score.max(similarity);
        }
        
        best_score
    }

    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for (i, c1) in s1.chars().enumerate() {
            for (j, c2) in s2.chars().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                    .min(matrix[i + 1][j] + 1)
                    .min(matrix[i][j] + cost);
            }
        }

        matrix[len1][len2]
    }

    fn calculate_recency_score(&self, document: &EnhancedDocument) -> f32 {
        // Calculate score based on how recent the document is
        if let Ok(updated_time) = chrono::DateTime::parse_from_rfc3339(&document.updated_at) {
            let now = chrono::Utc::now();
            let age = now.signed_duration_since(updated_time.with_timezone(&chrono::Utc));
            let days_old = age.num_days();
            
            // Score decreases with age (max 1.0 for documents updated today)
            (1.0 - (days_old as f32 / 365.0)).max(0.0)
        } else {
            0.0
        }
    }

    fn calculate_usage_score(&self, document: &EnhancedDocument) -> f32 {
        // Score based on access count (log scale)
        if document.access_count > 0 {
            (document.access_count as f32).ln() / 10.0
        } else {
            0.0
        }
    }
}