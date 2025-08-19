use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;

use super::embedding::SimpleEmbeddingService;
use super::search::{SearchService, SearchResult};
use crate::rag::enhanced::system::{EnhancedDocument, EnhancedDocumentChunk};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextSuggestion {
    pub document_id: String,
    pub document_name: String,
    pub relevance_score: f32,
    pub reason: String,
    pub preview: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RelatedDocument {
    pub document_id: String,
    pub document_name: String,
    pub relationship_type: String, // "similar", "referenced", "sequel", etc.
    pub similarity_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConversationContext {
    pub topics: Vec<String>,
    pub entities: Vec<String>,
    pub keywords: Vec<String>,
    pub intent: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RankedDocument {
    pub document: EnhancedDocument,
    pub relevance_score: f32,
    pub rank_factors: HashMap<String, f32>, // what contributed to the score
}

pub struct ContextEngine {
    embedding_service: Arc<SimpleEmbeddingService>,
    search_service: Arc<SearchService>,
    usage_stats: Arc<tokio::sync::Mutex<HashMap<String, u32>>>, // document_id -> usage_count
}

impl ContextEngine {
    pub fn new(
        embedding_service: Arc<SimpleEmbeddingService>,
        search_service: Arc<SearchService>,
    ) -> Self {
        Self {
            embedding_service,
            search_service,
            usage_stats: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Suggest relevant documents based on conversation history
    pub async fn suggest_context(&self, conversation_history: &[String]) -> Result<Vec<ContextSuggestion>> {
        if conversation_history.is_empty() {
            return Ok(Vec::new());
        }

        // Analyze conversation to extract context
        let context = self.analyze_conversation(conversation_history).await?;
        
        // Search for relevant documents
        let mut suggestions = Vec::new();
        
        // Search based on extracted topics and keywords
        for topic in &context.topics {
            let search_results = self.search_service.search_bm25(topic, 5)?;
            for result in search_results {
                suggestions.push(ContextSuggestion {
                    document_id: result.document_id.clone(),
                    document_name: result.title.unwrap_or_else(|| "Unknown".to_string()),
                    relevance_score: result.score,
                    reason: format!("Related to topic: {}", topic),
                    preview: result.content.chars().take(200).collect(),
                });
            }
        }

        // Sort by relevance and remove duplicates
        suggestions.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        suggestions.dedup_by(|a, b| a.document_id == b.document_id);
        
        Ok(suggestions.into_iter().take(10).collect())
    }

    /// Search all documents with advanced ranking
    pub async fn search_all_documents(&self, query: &str) -> Result<Vec<RankedDocument>> {
        // Perform hybrid search
        let search_results = self.search_service.search_bm25(query, 50)?;
        
        let mut ranked_docs = Vec::new();
        let usage_stats = self.usage_stats.lock().await;
        
        for result in search_results {
            let mut rank_factors = HashMap::new();
            
            // Base relevance score from search
            let base_score = result.score;
            rank_factors.insert("semantic_relevance".to_string(), base_score);
            
            // Usage frequency bonus
            let usage_count = usage_stats.get(&result.document_id).unwrap_or(&0);
            let usage_bonus = (*usage_count as f32).ln() / 10.0; // log scale
            rank_factors.insert("usage_frequency".to_string(), usage_bonus);
            
            // Recency bonus (could be implemented if we track document modification times)
            let recency_bonus = 0.0; // placeholder
            rank_factors.insert("recency".to_string(), recency_bonus);
            
            // Final score calculation
            let final_score = base_score + usage_bonus + recency_bonus;
            
            // Create mock document (in real implementation, fetch from database)
            let document = EnhancedDocument {
                id: result.document_id.clone(),
                file_name: result.title.unwrap_or_else(|| "Unknown".to_string()),
                file_path: String::new(),
                file_type: String::new(),
                file_size: 0,
                content: result.content,
                created_at: Utc::now().to_rfc3339(),
                updated_at: Utc::now().to_rfc3339(),
                access_count: *usage_count as i32,
                last_accessed: Some(Utc::now().to_rfc3339()),
                is_cached: true,
                embedding_status: "completed".to_string(),
                content_hash: Some(String::new()),
                chunk_count: 0,
                metadata: None,
            };
            
            ranked_docs.push(RankedDocument {
                document,
                relevance_score: final_score,
                rank_factors,
            });
        }
        
        // Sort by final score
        ranked_docs.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        Ok(ranked_docs)
    }

    /// Find documents related to the given document IDs
    pub async fn get_related_documents(&self, doc_ids: &[String]) -> Result<Vec<RelatedDocument>> {
        let mut related = Vec::new();
        
        for doc_id in doc_ids {
            // This would use embeddings to find similar documents
            // For now, return placeholder data
            let similar_results = self.search_service.search_bm25(&format!("similar to {}", doc_id), 5)?;
            
            for result in similar_results {
                if !doc_ids.contains(&result.document_id) {
                    related.push(RelatedDocument {
                        document_id: result.document_id,
                        document_name: result.title.unwrap_or_else(|| "Unknown".to_string()),
                        relationship_type: "similar".to_string(),
                        similarity_score: result.score,
                    });
                }
            }
        }
        
        // Remove duplicates and sort by similarity
        related.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        related.dedup_by(|a, b| a.document_id == b.document_id);
        
        Ok(related.into_iter().take(10).collect())
    }

    /// Record document usage for analytics
    pub async fn record_document_usage(&self, document_id: &str) {
        let mut usage_stats = self.usage_stats.lock().await;
        *usage_stats.entry(document_id.to_string()).or_insert(0) += 1;
    }

    /// Analyze conversation to extract topics, entities, and intent
    async fn analyze_conversation(&self, messages: &[String]) -> Result<ConversationContext> {
        // Simple keyword extraction (in production, use NLP libraries)
        let combined_text = messages.join(" ");
        let words: Vec<&str> = combined_text.split_whitespace().collect();
        
        // Extract potential topics (simple heuristics)
        let mut topics = Vec::new();
        let mut keywords = Vec::new();
        
        for word in &words {
            let clean_word = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            if clean_word.len() > 3 && !is_stop_word(&clean_word) {
                keywords.push(clean_word.clone());
                if clean_word.len() > 5 {
                    topics.push(clean_word);
                }
            }
        }
        
        // Remove duplicates
        topics.sort();
        topics.dedup();
        keywords.sort();
        keywords.dedup();
        
        // Simple intent detection
        let intent = if combined_text.contains("how") || combined_text.contains("what") {
            "question".to_string()
        } else if combined_text.contains("implement") || combined_text.contains("create") {
            "implementation".to_string()
        } else if combined_text.contains("error") || combined_text.contains("problem") {
            "troubleshooting".to_string()
        } else {
            "general".to_string()
        };
        
        Ok(ConversationContext {
            topics: topics.into_iter().take(5).collect(),
            entities: Vec::new(), // placeholder for entity extraction
            keywords: keywords.into_iter().take(10).collect(),
            intent,
        })
    }

    /// Get usage analytics for documents
    pub async fn get_usage_analytics(&self) -> Result<HashMap<String, u32>> {
        let usage_stats = self.usage_stats.lock().await;
        Ok(usage_stats.clone())
    }
}

/// Simple stop word filter
fn is_stop_word(word: &str) -> bool {
    matches!(word, 
        "the" | "and" | "or" | "but" | "in" | "on" | "at" | "to" | "for" | "of" | "with" | 
        "by" | "is" | "are" | "was" | "were" | "be" | "been" | "have" | "has" | "had" | 
        "do" | "does" | "did" | "will" | "would" | "could" | "should" | "can" | "may" | 
        "must" | "shall" | "this" | "that" | "these" | "those" | "a" | "an"
    )
}