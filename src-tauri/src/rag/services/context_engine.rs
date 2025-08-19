use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

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
    pub confidence: f32,
    pub relevant_chunks: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDocument {
    pub id: String,
    pub file_path: String,
    pub filename: String,
    pub relevance_score: f32,
    pub access_count: u32,
    pub last_accessed: DateTime<Utc>,
    pub content_preview: Option<String>,
    pub embedding_status: EmbeddingStatus,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingStatus {
    Pending,
    Processing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSession {
    pub id: String,
    pub chat_id: String,
    pub active_documents: Vec<String>,
    pub suggested_documents: Vec<String>,
    pub context_mode: ContextMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextMode {
    Auto,
    Manual,
    Search,
    All,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAnalysis {
    pub topics: Vec<String>,
    pub entities: Vec<String>,
    pub intent: String,
    pub suggested_documents: Vec<ContextSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
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
    sessions: Arc<RwLock<HashMap<String, ContextSession>>>,
    document_cache: Arc<RwLock<HashMap<String, ContextDocument>>>,
    access_patterns: Arc<RwLock<HashMap<String, Vec<AccessPattern>>>>,
}

#[derive(Debug, Clone)]
struct AccessPattern {
    document_id: String,
    timestamp: DateTime<Utc>,
    context: String,
    relevance: f32,
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
            sessions: Arc::new(RwLock::new(HashMap::new())),
            document_cache: Arc::new(RwLock::new(HashMap::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn initialize_context_session(&self, chat_id: String) -> Result<ContextSession> {
        let session = ContextSession {
            id: Uuid::new_v4().to_string(),
            chat_id: chat_id.clone(),
            active_documents: Vec::new(),
            suggested_documents: Vec::new(),
            context_mode: ContextMode::Auto,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(chat_id, session.clone());

        Ok(session)
    }
    
    pub async fn analyze_conversation_context(&self, messages: Vec<ConversationMessage>) -> Result<ContextAnalysis> {
        // Extract topics and entities from conversation
        let context = self.analyze_conversation(&messages.iter().map(|m| m.content.clone()).collect::<Vec<_>>()).await?;
        
        // Generate document suggestions based on analysis
        let suggested_documents = self.generate_advanced_suggestions(&context).await?;
        
        Ok(ContextAnalysis {
            topics: context.topics,
            entities: context.entities,
            intent: context.intent,
            suggested_documents,
        })
    }
    
    async fn generate_advanced_suggestions(&self, context: &ConversationContext) -> Result<Vec<ContextSuggestion>> {
        let mut suggestions = Vec::new();
        let mut seen_docs = HashSet::new();
        
        // Search based on topics
        for topic in &context.topics {
            let search_results = self.search_service.search_bm25(topic, 5)?;
            for result in search_results {
                if seen_docs.insert(result.document_id.clone()) {
                    suggestions.push(ContextSuggestion {
                        document_id: result.document_id.clone(),
                        document_name: result.title.unwrap_or_else(|| "Unknown".to_string()),
                        relevance_score: result.score,
                        reason: format!("Related to topic: {}", topic),
                        preview: result.content.chars().take(200).collect(),
                        confidence: result.score * 0.8,
                        relevant_chunks: vec![result.content.chars().take(500).collect()],
                    });
                }
            }
        }
        
        // Search based on entities
        for entity in &context.entities {
            let search_results = self.search_service.search_bm25(entity, 3)?;
            for result in search_results {
                if seen_docs.insert(result.document_id.clone()) {
                    suggestions.push(ContextSuggestion {
                        document_id: result.document_id.clone(),
                        document_name: result.title.unwrap_or_else(|| "Unknown".to_string()),
                        relevance_score: result.score,
                        reason: format!("References: {}", entity),
                        preview: result.content.chars().take(200).collect(),
                        confidence: result.score * 0.7,
                        relevant_chunks: vec![result.content.chars().take(500).collect()],
                    });
                }
            }
        }
        
        // Check access patterns for frequently used documents
        let _patterns = self.access_patterns.read().await;
        let usage_stats = self.usage_stats.lock().await;
        
        for (doc_id, count) in usage_stats.iter() {
            if *count > 2 && seen_docs.insert(doc_id.clone()) {
                suggestions.push(ContextSuggestion {
                    document_id: doc_id.clone(),
                    document_name: format!("Document {}", doc_id),
                    relevance_score: 0.5 + (*count as f32 * 0.1).min(0.5),
                    reason: format!("Frequently accessed ({} times)", count),
                    preview: String::new(),
                    confidence: 0.6,
                    relevant_chunks: Vec::new(),
                });
            }
        }
        
        // Sort by confidence and take top suggestions
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(suggestions.into_iter().take(10).collect())
    }
    
    pub async fn get_cached_context_documents(&self) -> Result<Vec<ContextDocument>> {
        let cache = self.document_cache.read().await;
        let mut documents: Vec<ContextDocument> = cache.values().cloned().collect();
        
        // Sort by access count and recency
        documents.sort_by(|a, b| {
            let score_a = a.access_count as f32 + (1.0 / (Utc::now() - a.last_accessed).num_hours().max(1) as f32);
            let score_b = b.access_count as f32 + (1.0 / (Utc::now() - b.last_accessed).num_hours().max(1) as f32);
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        // Return top 10
        Ok(documents.into_iter().take(10).collect())
    }
    
    pub async fn update_document_access(
        &self,
        document_id: String,
        access_count: u32,
        last_accessed: String,
    ) -> Result<()> {
        let mut cache = self.document_cache.write().await;
        let last_accessed_dt = DateTime::parse_from_rfc3339(&last_accessed)?.with_timezone(&Utc);
        
        if let Some(doc) = cache.get_mut(&document_id) {
            doc.access_count = access_count;
            doc.last_accessed = last_accessed_dt;
        } else {
            // Create new cache entry
            let context_doc = ContextDocument {
                id: document_id.clone(),
                file_path: String::new(),
                filename: format!("Document_{}", document_id),
                relevance_score: 0.0,
                access_count,
                last_accessed: last_accessed_dt,
                content_preview: None,
                embedding_status: EmbeddingStatus::Ready,
                metadata: HashMap::new(),
            };
            cache.insert(document_id.clone(), context_doc);
        }
        
        // Record access pattern
        let mut patterns = self.access_patterns.write().await;
        let pattern = AccessPattern {
            document_id: document_id.clone(),
            timestamp: last_accessed_dt,
            context: String::new(),
            relevance: 1.0,
        };
        
        patterns
            .entry(document_id.clone())
            .or_insert_with(Vec::new)
            .push(pattern);
        
        // Also update usage stats
        let mut usage_stats = self.usage_stats.lock().await;
        *usage_stats.entry(document_id).or_insert(0) = access_count;
        
        Ok(())
    }
    
    pub async fn search_context_documents(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let results = self.search_service.search_bm25(query, limit)?;
        Ok(results.into_iter().map(|r| r.document_id).collect())
    }
    
    pub async fn get_context_for_message(
        &self,
        message: &str,
        document_ids: Vec<String>,
        max_chunks: usize,
    ) -> Result<Vec<String>> {
        let mut all_chunks = Vec::new();
        
        for doc_id in document_ids {
            let results = self.search_service.search_bm25(&format!("{} in:{}", message, doc_id), max_chunks)?;
            all_chunks.extend(results.into_iter().map(|r| r.content));
        }
        
        // Take top chunks
        all_chunks.truncate(max_chunks);
        
        Ok(all_chunks)
    }
    
    pub async fn process_document_embeddings(&self, document_id: &str, priority: &str) -> Result<()> {
        let mut cache = self.document_cache.write().await;
        
        if let Some(doc) = cache.get_mut(document_id) {
            doc.embedding_status = if priority == "high" {
                EmbeddingStatus::Processing
            } else {
                EmbeddingStatus::Pending
            };
        }
        
        // In production, trigger actual embedding processing
        // For now, just mark as ready after a delay
        tokio::spawn({
            let cache = self.document_cache.clone();
            let doc_id = document_id.to_string();
            async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                let mut cache = cache.write().await;
                if let Some(doc) = cache.get_mut(&doc_id) {
                    doc.embedding_status = EmbeddingStatus::Ready;
                }
            }
        });
        
        Ok(())
    }
    
    pub async fn update_context_session(
        &self,
        session_id: &str,
        mode: ContextMode,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        for session in sessions.values_mut() {
            if session.id == session_id {
                session.context_mode = mode;
                session.updated_at = Utc::now();
                break;
            }
        }
        
        Ok(())
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
                    confidence: result.score,
                    relevant_chunks: vec![result.content.chars().take(500).collect()],
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