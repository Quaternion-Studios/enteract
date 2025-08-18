use serde::{Deserialize, Serialize};

use super::embeddings::{EmbeddingConfig};
use super::search::{SearchConfig};
use super::chunking::{ChunkingConfig};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnhancedDocument {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    pub file_size: i64,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub access_count: i32,
    pub last_accessed: Option<String>,
    pub is_cached: bool,
    pub embedding_status: String, // "pending", "processing", "completed", "failed"
    pub chunk_count: i32,
    pub metadata: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnhancedDocumentChunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub start_char: i32,
    pub end_char: i32,
    pub token_count: i32,
    pub embedding: Option<Vec<f32>>,
    pub similarity_score: Option<f32>,
    pub bm25_score: Option<f32>,
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnhancedRagSettings {
    pub max_document_size_mb: f64,
    pub max_collection_size_gb: f64,
    pub max_cached_documents: usize,
    pub auto_embedding: bool,
    pub background_processing: bool,
    pub reranking_enabled: bool,
    pub chunking_config: ChunkingConfig,
    pub embedding_config: EmbeddingConfig,
    pub search_config: SearchConfig,
}

impl Default for EnhancedRagSettings {
    fn default() -> Self {
        Self {
            max_document_size_mb: 50.0,
            max_collection_size_gb: 2.0,
            max_cached_documents: 10,
            auto_embedding: true,
            background_processing: true,
            reranking_enabled: false, // Disabled by default for performance
            chunking_config: ChunkingConfig::default(),
            embedding_config: EmbeddingConfig::default(),
            search_config: SearchConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocumentValidationResult {
    pub ready_documents: Vec<String>,
    pub pending_documents: Vec<String>,
    pub processing_documents: Vec<String>,
    pub failed_documents: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_documents: usize,
    pub total_chunks: usize,
    pub database_size_mb: f64,
    pub storage_size_mb: f64,
    pub index_size_mb: f64,
    pub cache_size_mb: f64,
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingStatus {
    pub total_documents: usize,
    pub processed_documents: usize,
    pub pending_documents: usize,
    pub failed_documents: usize,
    pub processing_documents: usize,
    pub percentage_complete: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub chunks: Vec<EnhancedDocumentChunk>,
    pub total_results: usize,
    pub search_time_ms: u64,
    pub query: String,
}