use anyhow::{Result, anyhow};
use rusqlite::{Connection, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::fs;
use chrono::Utc;
use uuid::Uuid;
use tauri::Manager;
use sha2::{Sha256, Digest};

use crate::simple_embedding_service::{SimpleEmbeddingService as EmbeddingService, EmbeddingConfig};
use crate::search_service::{SearchService, SearchConfig, SearchResult};
use crate::chunking_service::{ChunkingService, ChunkingConfig, TextChunk, extract_text_from_pdf, clean_text};

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

#[derive(Clone)]
pub struct EnhancedRagSystem {
    db_path: PathBuf,
    storage_path: PathBuf,
    index_path: PathBuf,
    cache_path: PathBuf,
    settings: Arc<Mutex<EnhancedRagSettings>>,
    embedding_service: Arc<EmbeddingService>,
    search_service: Arc<SearchService>,
    chunking_service: Arc<Mutex<ChunkingService>>,
}

impl EnhancedRagSystem {
    pub async fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
        let app_dir = app_handle.path().app_data_dir()?;
        let db_path = app_dir.join("enhanced_rag_documents.db");
        let storage_path = app_dir.join("document_storage");
        let index_path = app_dir.join("tantivy_index");
        let cache_path = app_dir.join("model_cache");
        
        // Create directories
        fs::create_dir_all(&storage_path)?;
        fs::create_dir_all(&index_path)?;
        fs::create_dir_all(&cache_path)?;
        
        let settings = Arc::new(Mutex::new(EnhancedRagSettings::default()));
        
        // Initialize services
        let embedding_service = Arc::new(EmbeddingService::new(
            cache_path.clone(), 
            Some(settings.lock().unwrap().embedding_config.clone())
        ));
        
        let search_service = Arc::new(SearchService::new(
            index_path.clone(),
            Some(settings.lock().unwrap().search_config.clone())
        )?);
        
        let chunking_service = Arc::new(Mutex::new(ChunkingService::new(
            Some(settings.lock().unwrap().chunking_config.clone())
        )?));
        
        let system = Self {
            db_path,
            storage_path,
            index_path,
            cache_path,
            settings,
            embedding_service,
            search_service,
            chunking_service,
        };
        
        // Initialize database and services
        system.initialize_database()?;
        system.search_service.initialize_writer()?;
        
        // Initialize embedding service in background
        let embedding_service_clone = system.embedding_service.clone();
        tokio::spawn(async move {
            if let Err(e) = embedding_service_clone.initialize().await {
                eprintln!("Failed to initialize embedding service: {}", e);
            } else {
                println!("Embedding service initialized successfully");
            }
        });
        
        Ok(system)
    }
    
    fn initialize_database(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        // Create enhanced documents table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS enhanced_documents (
                id TEXT PRIMARY KEY,
                file_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                access_count INTEGER DEFAULT 0,
                last_accessed TEXT,
                is_cached INTEGER DEFAULT 0,
                embedding_status TEXT DEFAULT 'pending',
                chunk_count INTEGER DEFAULT 0,
                metadata TEXT,
                content_hash TEXT
            )",
            [],
        )?;
        
        // Add content_hash column if it doesn't exist (for existing databases)
        let _ = conn.execute(
            "ALTER TABLE enhanced_documents ADD COLUMN content_hash TEXT",
            [],
        );
        
        // Create enhanced document_chunks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS enhanced_document_chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                start_char INTEGER NOT NULL,
                end_char INTEGER NOT NULL,
                token_count INTEGER NOT NULL,
                embedding BLOB,
                metadata TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (document_id) REFERENCES enhanced_documents(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Create processing_queue table for background tasks
        conn.execute(
            "CREATE TABLE IF NOT EXISTS processing_queue (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                task_type TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                error_message TEXT,
                FOREIGN KEY (document_id) REFERENCES enhanced_documents(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Create user_settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS enhanced_user_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;
        
        // Create indexes for better performance
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_enhanced_document_chunks_document_id ON enhanced_document_chunks(document_id)",
            "CREATE INDEX IF NOT EXISTS idx_enhanced_documents_last_accessed ON enhanced_documents(last_accessed)",
            "CREATE INDEX IF NOT EXISTS idx_enhanced_documents_embedding_status ON enhanced_documents(embedding_status)",
            "CREATE INDEX IF NOT EXISTS idx_processing_queue_status ON processing_queue(status)",
            "CREATE INDEX IF NOT EXISTS idx_processing_queue_document_id ON processing_queue(document_id)",
        ];
        
        for index_sql in indexes {
            conn.execute(index_sql, [])?;
        }
        
        // Load settings from database
        self.load_settings_from_db()?;
        
        Ok(())
    }
    
    pub fn check_duplicate_public(&self, content_hash: &str) -> Result<Option<EnhancedDocument>> {
        self.check_duplicate(content_hash)
    }
    
    fn check_duplicate(&self, content_hash: &str) -> Result<Option<EnhancedDocument>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, file_name, file_path, file_type, file_size, content,
                    created_at, updated_at, access_count, last_accessed, is_cached,
                    embedding_status, chunk_count, metadata, content_hash
             FROM enhanced_documents
             WHERE content_hash = ?1"
        )?;
        
        let document = stmt.query_row(params![content_hash], |row| {
            Ok(EnhancedDocument {
                id: row.get(0)?,
                file_name: row.get(1)?,
                file_path: row.get(2)?,
                file_type: row.get(3)?,
                file_size: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                access_count: row.get(8)?,
                last_accessed: row.get(9)?,
                is_cached: row.get::<_, i32>(10)? != 0,
                embedding_status: row.get(11)?,
                chunk_count: row.get(12)?,
                metadata: row.get(13)?,
                content_hash: row.get(14)?,
            })
        }).optional()?;
        
        Ok(document)
    }
    
    pub async fn upload_document(
        &self,
        file_name: String,
        file_content: Vec<u8>,
        file_type: String,
    ) -> Result<EnhancedDocument> {
        // Calculate content hash for duplicate detection
        let mut hasher = Sha256::new();
        hasher.update(&file_content);
        hasher.update(file_name.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());
        
        // Check for duplicates
        let existing_doc = self.check_duplicate(&content_hash)?;
        if let Some(doc) = existing_doc {
            return Ok(doc);
        }
        
        // Validate file size
        let (max_size_mb, auto_embedding) = {
            let settings = self.settings.lock().unwrap();
            (settings.max_document_size_mb, settings.auto_embedding)
        };
        
        let file_size_mb = file_content.len() as f64 / (1024.0 * 1024.0);
        if file_size_mb > max_size_mb {
            return Err(anyhow!(
                "File size {:.2}MB exceeds limit of {:.2}MB",
                file_size_mb, max_size_mb
            ));
        }
        
        // Generate unique ID
        let doc_id = Uuid::new_v4().to_string();
        
        // Save file to storage
        let file_path = self.storage_path.join(&doc_id).join(&file_name);
        fs::create_dir_all(file_path.parent().unwrap())?;
        fs::write(&file_path, &file_content)?;
        
        // Extract and clean text content
        let raw_text = self.extract_text_content(&file_content, &file_type)?;
        let clean_content = clean_text(&raw_text);
        
        // Create document chunks
        let chunks = self.create_document_chunks(&doc_id, &clean_content).await?;
        
        // Create document record
        let now = Utc::now().to_rfc3339();
        let document = EnhancedDocument {
            id: doc_id.clone(),
            file_name: file_name.clone(),
            file_path: file_path.to_string_lossy().to_string(),
            file_type,
            file_size: file_content.len() as i64,
            content: clean_content.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            access_count: 0,
            last_accessed: None,
            is_cached: false,
            embedding_status: "pending".to_string(),
            chunk_count: chunks.len() as i32,
            metadata: None,
            content_hash: Some(content_hash),
        };
        
        // Save to database
        self.save_document_to_db(&document)?;
        self.save_chunks_to_db(&doc_id, &chunks)?;
        
        // Queue for embedding generation if enabled
        if auto_embedding {
            self.queue_embedding_generation(&doc_id).await?;
        }
        
        println!("Document uploaded: {} with {} chunks", file_name, chunks.len());
        
        Ok(document)
    }
    
    fn extract_text_content(&self, file_content: &[u8], file_type: &str) -> Result<String> {
        match file_type {
            t if t.contains("text") || t.contains("plain") => {
                Ok(String::from_utf8_lossy(file_content).to_string())
            }
            t if t.contains("pdf") => {
                extract_text_from_pdf(file_content)
            }
            t if t.contains("image") => {
                // TODO: Implement OCR
                Ok("Image OCR not yet implemented".to_string())
            }
            _ => {
                // Try to parse as text
                Ok(String::from_utf8_lossy(file_content).to_string())
            }
        }
    }
    
    async fn create_document_chunks(&self, document_id: &str, content: &str) -> Result<Vec<TextChunk>> {
        let chunking_service = self.chunking_service.lock().unwrap();
        let chunks = chunking_service.chunk_text(content)?;
        
        // Convert to our TextChunk format with document_id
        Ok(chunks)
    }
    
    fn save_document_to_db(&self, document: &EnhancedDocument) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO enhanced_documents (
                id, file_name, file_path, file_type, file_size, content,
                created_at, updated_at, access_count, last_accessed, is_cached,
                embedding_status, chunk_count, metadata, content_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                document.id,
                document.file_name,
                document.file_path,
                document.file_type,
                document.file_size,
                document.content,
                document.created_at,
                document.updated_at,
                document.access_count,
                document.last_accessed,
                document.is_cached as i32,
                document.embedding_status,
                document.chunk_count,
                document.metadata,
                document.content_hash,
            ],
        )?;
        Ok(())
    }
    
    fn save_chunks_to_db(&self, document_id: &str, chunks: &[TextChunk]) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let now = Utc::now().to_rfc3339();
        
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO enhanced_document_chunks (
                    id, document_id, chunk_index, content, start_char, end_char,
                    token_count, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    chunk_id,
                    document_id,
                    i as i32,
                    chunk.content,
                    chunk.start_char as i32,
                    chunk.end_char as i32,
                    chunk.token_count as i32,
                    now,
                ],
            )?;
        }
        Ok(())
    }
    
    async fn queue_embedding_generation(&self, document_id: &str) -> Result<()> {
        // Add to processing queue
        let queue_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO processing_queue (id, document_id, task_type, status, created_at)
             VALUES (?1, ?2, 'embedding_generation', 'pending', ?3)",
            params![queue_id, document_id, now],
        )?;
        
        // Process in background
        let system_clone = self.clone();
        let document_id_clone = document_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = system_clone.process_embeddings(&document_id_clone).await {
                eprintln!("Failed to process embeddings for document {}: {}", document_id_clone, e);
            }
        });
        
        Ok(())
    }
    
    async fn process_embeddings(&self, document_id: &str) -> Result<()> {
        // Wait for embedding service to be ready
        while !self.embedding_service.is_initialized() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // Update document status
        self.update_embedding_status(document_id, "processing")?;
        
        // Get document chunks
        let chunks = self.get_document_chunks(document_id)?;
        if chunks.is_empty() {
            return Err(anyhow!("No chunks found for document {}", document_id));
        }
        
        // Generate embeddings for chunks
        let chunk_texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        
        match self.embedding_service.embed_documents(chunk_texts) {
            Ok(embeddings) => {
                // Save embeddings to database and search index
                self.save_embeddings_to_db(document_id, &chunks, &embeddings)?;
                self.index_chunks_for_search(document_id, &chunks, &embeddings).await?;
                
                // Update document status
                self.update_embedding_status(document_id, "completed")?;
                self.update_document_cached_status(document_id, true)?;
                
                println!("Successfully processed embeddings for document {}", document_id);
            }
            Err(e) => {
                self.update_embedding_status(document_id, "failed")?;
                return Err(anyhow!("Failed to generate embeddings: {}", e));
            }
        }
        
        Ok(())
    }
    
    fn get_document_chunks(&self, document_id: &str) -> Result<Vec<EnhancedDocumentChunk>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, document_id, chunk_index, content, start_char, end_char, token_count, embedding, metadata
             FROM enhanced_document_chunks
             WHERE document_id = ?1
             ORDER BY chunk_index"
        )?;
        
        let chunks = stmt.query_map([document_id], |row| {
            Ok(EnhancedDocumentChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                start_char: row.get(4)?,
                end_char: row.get(5)?,
                token_count: row.get(6)?,
                embedding: None, // Will be loaded separately if needed
                similarity_score: None,
                bm25_score: None,
                metadata: row.get(8)?,
            })
        })?;
        
        Ok(chunks.collect::<Result<Vec<_>, _>>()?)
    }
    
    fn save_embeddings_to_db(&self, document_id: &str, chunks: &[EnhancedDocumentChunk], embeddings: &[Vec<f32>]) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            // Serialize embedding as bytes
            let embedding_bytes = embedding.iter()
                .flat_map(|&f| f.to_le_bytes().to_vec())
                .collect::<Vec<u8>>();
            
            conn.execute(
                "UPDATE enhanced_document_chunks SET embedding = ?1 WHERE id = ?2",
                params![embedding_bytes, chunk.id],
            )?;
        }
        
        Ok(())
    }
    
    async fn index_chunks_for_search(&self, document_id: &str, chunks: &[EnhancedDocumentChunk], embeddings: &[Vec<f32>]) -> Result<()> {
        let search_chunks: Vec<crate::search_service::DocumentChunk> = chunks.iter()
            .zip(embeddings.iter())
            .map(|(chunk, embedding)| crate::search_service::DocumentChunk {
                id: chunk.id.clone(),
                document_id: chunk.document_id.clone(),
                content: chunk.content.clone(),
                embedding: Some(embedding.clone()),
                metadata: chunk.metadata.clone(),
            })
            .collect();
        
        self.search_service.add_documents(search_chunks)?;
        self.search_service.commit()?;
        
        Ok(())
    }
    
    fn update_embedding_status(&self, document_id: &str, status: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            "UPDATE enhanced_documents SET embedding_status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, document_id],
        )?;
        
        Ok(())
    }
    
    fn update_document_cached_status(&self, document_id: &str, is_cached: bool) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            "UPDATE enhanced_documents SET is_cached = ?1, updated_at = ?2 WHERE id = ?3",
            params![is_cached as i32, now, document_id],
        )?;
        
        Ok(())
    }
    
    pub async fn search_documents(&self, query: &str, context_document_ids: Vec<String>) -> Result<Vec<EnhancedDocumentChunk>> {
        // Update access count for queried documents
        self.update_document_access(&context_document_ids)?;
        
        // Generate query embedding
        let query_embedding = if self.embedding_service.is_initialized() {
            match self.embedding_service.embed_query(query) {
                Ok(emb) => Some(emb),
                Err(e) => {
                    eprintln!("Failed to generate query embedding: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        // Perform search
        let search_results = if let Some(embedding) = query_embedding {
            // Use hybrid search (BM25 + vector)
            self.search_service.hybrid_search(query, &embedding, 20)?
        } else {
            // Fall back to BM25 only
            self.search_service.search_bm25(query, 20)?
        };
        
        // Filter by context documents if specified
        let filtered_results = if context_document_ids.is_empty() {
            search_results
        } else {
            search_results.into_iter()
                .filter(|result| context_document_ids.contains(&result.document_id))
                .collect()
        };
        
        // Convert search results to enhanced document chunks
        let enhanced_chunks = self.convert_search_results_to_chunks(filtered_results)?;
        
        Ok(enhanced_chunks)
    }
    
    fn convert_search_results_to_chunks(&self, search_results: Vec<SearchResult>) -> Result<Vec<EnhancedDocumentChunk>> {
        let conn = Connection::open(&self.db_path)?;
        let mut chunks = Vec::new();
        
        for result in search_results {
            let mut stmt = conn.prepare(
                "SELECT id, document_id, chunk_index, content, start_char, end_char, token_count, metadata
                 FROM enhanced_document_chunks WHERE id = ?1"
            )?;
            
            let chunk_result = stmt.query_row([&result.chunk_id], |row| {
                Ok(EnhancedDocumentChunk {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_index: row.get(2)?,
                    content: row.get(3)?,
                    start_char: row.get(4)?,
                    end_char: row.get(5)?,
                    token_count: row.get(6)?,
                    embedding: None,
                    similarity_score: Some(result.score),
                    bm25_score: Some(result.bm25_score),
                    metadata: row.get(7)?,
                })
            });
            
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }
        
        Ok(chunks)
    }
    
    fn update_document_access(&self, document_ids: &[String]) -> Result<()> {
        if document_ids.is_empty() {
            return Ok(());
        }
        
        let conn = Connection::open(&self.db_path)?;
        let now = Utc::now().to_rfc3339();
        
        for doc_id in document_ids {
            conn.execute(
                "UPDATE enhanced_documents 
                 SET access_count = access_count + 1, last_accessed = ?1 
                 WHERE id = ?2",
                params![now, doc_id],
            )?;
        }
        
        Ok(())
    }
    
    pub fn get_all_documents(&self) -> Result<Vec<EnhancedDocument>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, file_name, file_path, file_type, file_size, content,
                    created_at, updated_at, access_count, last_accessed, is_cached,
                    embedding_status, chunk_count, metadata, content_hash
             FROM enhanced_documents
             ORDER BY created_at DESC"
        )?;
        
        let documents = stmt.query_map([], |row| {
            Ok(EnhancedDocument {
                id: row.get(0)?,
                file_name: row.get(1)?,
                file_path: row.get(2)?,
                file_type: row.get(3)?,
                file_size: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                access_count: row.get(8)?,
                last_accessed: row.get(9)?,
                is_cached: row.get::<_, i32>(10)? != 0,
                embedding_status: row.get(11)?,
                chunk_count: row.get(12)?,
                metadata: row.get(13)?,
                content_hash: row.get(14)?,
            })
        })?;
        
        Ok(documents.collect::<Result<Vec<_>, _>>()?)
    }
    
    pub async fn delete_document(&self, document_id: &str) -> Result<()> {
        // Delete from search index
        self.search_service.delete_document(document_id)?;
        self.search_service.commit()?;
        
        // Delete from database (cascades to chunks)
        let conn = Connection::open(&self.db_path)?;
        conn.execute("DELETE FROM enhanced_documents WHERE id = ?1", params![document_id])?;
        
        // Delete files from storage
        let doc_path = self.storage_path.join(document_id);
        if doc_path.exists() {
            fs::remove_dir_all(doc_path)?;
        }
        
        Ok(())
    }
    
    pub async fn generate_embeddings(&self, document_id: &str) -> Result<String> {
        if !self.embedding_service.is_initialized() {
            return Err(anyhow!("Embedding service not initialized"));
        }
        
        self.queue_embedding_generation(document_id).await?;
        Ok(format!("Embeddings queued for generation for document {}", document_id))
    }
    
    pub async fn clear_embedding_cache(&self) -> Result<String> {
        // Clear search index
        self.search_service.clear_index()?;
        
        // Clear embeddings from database
        let conn = Connection::open(&self.db_path)?;
        conn.execute("UPDATE enhanced_document_chunks SET embedding = NULL", [])?;
        conn.execute("UPDATE enhanced_documents SET is_cached = 0, embedding_status = 'pending'", [])?;
        
        Ok("Embedding cache cleared successfully".to_string())
    }
    
    pub fn get_settings(&self) -> EnhancedRagSettings {
        self.settings.lock().unwrap().clone()
    }
    
    pub fn update_settings(&self, new_settings: EnhancedRagSettings) -> Result<()> {
        // Update in-memory settings
        let mut settings = self.settings.lock().unwrap();
        *settings = new_settings.clone();
        drop(settings);
        
        // Save to database
        let conn = Connection::open(&self.db_path)?;
        let settings_json = serde_json::to_string(&new_settings)?;
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT OR REPLACE INTO enhanced_user_settings (key, value, updated_at) 
             VALUES ('enhanced_rag_settings', ?1, ?2)",
            params![settings_json, now],
        )?;
        
        Ok(())
    }
    
    fn load_settings_from_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let result = conn.query_row(
            "SELECT value FROM enhanced_user_settings WHERE key = 'enhanced_rag_settings'",
            [],
            |row| {
                let settings_json: String = row.get(0)?;
                Ok(settings_json)
            },
        );
        
        if let Ok(settings_json) = result {
            if let Ok(stored_settings) = serde_json::from_str::<EnhancedRagSettings>(&settings_json) {
                let mut settings = self.settings.lock().unwrap();
                *settings = stored_settings;
            }
        }
        
        Ok(())
    }
    
    pub fn get_storage_stats(&self) -> Result<HashMap<String, serde_json::Value>> {
        let conn = Connection::open(&self.db_path)?;
        
        let doc_count: i64 = conn.query_row("SELECT COUNT(*) FROM enhanced_documents", [], |row| row.get(0))?;
        let total_size: i64 = conn.query_row("SELECT COALESCE(SUM(file_size), 0) FROM enhanced_documents", [], |row| row.get(0))?;
        let cached_count: i64 = conn.query_row("SELECT COUNT(*) FROM enhanced_documents WHERE is_cached = 1", [], |row| row.get(0))?;
        let total_chunks: i64 = conn.query_row("SELECT COUNT(*) FROM enhanced_document_chunks", [], |row| row.get(0))?;
        let embedded_chunks: i64 = conn.query_row("SELECT COUNT(*) FROM enhanced_document_chunks WHERE embedding IS NOT NULL", [], |row| row.get(0))?;
        
        let mut stats = HashMap::new();
        stats.insert("total_documents".to_string(), serde_json::json!(doc_count));
        stats.insert("total_size_bytes".to_string(), serde_json::json!(total_size));
        stats.insert("total_size_mb".to_string(), serde_json::json!(total_size as f64 / (1024.0 * 1024.0)));
        stats.insert("cached_documents".to_string(), serde_json::json!(cached_count));
        stats.insert("total_chunks".to_string(), serde_json::json!(total_chunks));
        stats.insert("embedded_chunks".to_string(), serde_json::json!(embedded_chunks));
        stats.insert("embedding_coverage".to_string(), serde_json::json!(
            if total_chunks > 0 { embedded_chunks as f64 / total_chunks as f64 } else { 0.0 }
        ));
        
        let settings = self.settings.lock().unwrap();
        stats.insert("max_cached_documents".to_string(), serde_json::json!(settings.max_cached_documents));
        stats.insert("max_document_size_mb".to_string(), serde_json::json!(settings.max_document_size_mb));
        stats.insert("embedding_model".to_string(), serde_json::json!(settings.embedding_config.model_name));
        stats.insert("reranking_enabled".to_string(), serde_json::json!(settings.reranking_enabled));
        
        Ok(stats)
    }
}