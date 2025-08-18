use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Document {
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
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub start_char: i32,
    pub end_char: i32,
    pub embedding: Option<Vec<f32>>,
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmbeddingCache {
    pub document_id: String,
    pub embeddings: Vec<Vec<f32>>,
    pub cached_at: String,
    pub access_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagSettings {
    pub max_document_size_mb: f64,
    pub max_collection_size_gb: f64,
    pub max_cached_documents: usize,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub auto_embedding: bool,
    pub background_processing: bool,
}

impl Default for RagSettings {
    fn default() -> Self {
        RagSettings {
            max_document_size_mb: 50.0,
            max_collection_size_gb: 2.0,
            max_cached_documents: 5,
            chunk_size: 512,
            chunk_overlap: 50,
            auto_embedding: true,
            background_processing: true,
        }
    }
}

#[derive(Clone)]
pub struct RagSystem {
    db_path: PathBuf,
    storage_path: PathBuf,
    settings: Arc<Mutex<RagSettings>>,
    embedding_cache: Arc<Mutex<HashMap<String, EmbeddingCache>>>,
}

impl RagSystem {
    pub fn new(app_handle: &tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let app_dir = app_handle.path().app_data_dir()?;
        let db_path = app_dir.join("rag_documents.db");
        let storage_path = app_dir.join("document_storage");
        
        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_path)?;
        
        let system = RagSystem {
            db_path,
            storage_path,
            settings: Arc::new(Mutex::new(RagSettings::default())),
            embedding_cache: Arc::new(Mutex::new(HashMap::new())),
        };
        
        system.initialize_database()?;
        Ok(system)
    }
    
    fn initialize_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        
        // Create documents table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
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
                metadata TEXT
            )",
            [],
        )?;
        
        // Create document_chunks table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS document_chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                start_char INTEGER NOT NULL,
                end_char INTEGER NOT NULL,
                embedding BLOB,
                metadata TEXT,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Create embeddings_cache table for temporary storage
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings_cache (
                document_id TEXT PRIMARY KEY,
                embeddings BLOB NOT NULL,
                cached_at TEXT NOT NULL,
                access_count INTEGER DEFAULT 0,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )?;
        
        // Create user_settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        
        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_document_chunks_document_id 
             ON document_chunks(document_id)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_last_accessed 
             ON documents(last_accessed)",
            [],
        )?;
        
        Ok(())
    }
    
    pub async fn upload_document(
        &self,
        file_name: String,
        file_content: Vec<u8>,
        file_type: String,
    ) -> Result<Document, Box<dyn std::error::Error>> {
        // Check file size limit
        let settings = self.settings.lock().unwrap();
        let file_size_mb = file_content.len() as f64 / (1024.0 * 1024.0);
        if file_size_mb > settings.max_document_size_mb {
            return Err(format!(
                "File size {:.2}MB exceeds limit of {:.2}MB",
                file_size_mb, settings.max_document_size_mb
            ).into());
        }
        drop(settings);
        
        // Generate unique ID for document
        let doc_id = Uuid::new_v4().to_string();
        
        // Save file to storage
        let file_path = self.storage_path.join(&doc_id).join(&file_name);
        fs::create_dir_all(file_path.parent().unwrap())?;
        fs::write(&file_path, &file_content)?;
        
        // Extract text content based on file type
        let text_content = self.extract_text_content(&file_content, &file_type)?;
        
        // Create document record
        let now = Utc::now().to_rfc3339();
        let document = Document {
            id: doc_id.clone(),
            file_name: file_name.clone(),
            file_path: file_path.to_string_lossy().to_string(),
            file_type,
            file_size: file_content.len() as i64,
            content: text_content.clone(),
            created_at: now.clone(),
            updated_at: now,
            access_count: 0,
            last_accessed: None,
            is_cached: false,
            metadata: None,
        };
        
        // Save to database
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO documents (
                id, file_name, file_path, file_type, file_size, content,
                created_at, updated_at, access_count, last_accessed, is_cached, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                document.metadata,
            ],
        )?;
        
        // Create chunks for the document
        self.create_document_chunks(&doc_id, &text_content)?;
        
        // Queue for embedding generation if auto-embedding is enabled
        let settings = self.settings.lock().unwrap();
        if settings.auto_embedding {
            // This would trigger background embedding generation
            self.queue_embedding_generation(&doc_id);
        }
        
        Ok(document)
    }
    
    fn extract_text_content(
        &self,
        file_content: &[u8],
        file_type: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Simple text extraction based on file type
        match file_type {
            t if t.contains("text") || t.contains("plain") => {
                Ok(String::from_utf8_lossy(file_content).to_string())
            }
            t if t.contains("pdf") => {
                // TODO: Implement PDF text extraction
                Ok("PDF content extraction pending implementation".to_string())
            }
            t if t.contains("image") => {
                // TODO: Implement OCR for images
                Ok("Image OCR pending implementation".to_string())
            }
            _ => {
                Ok(String::from_utf8_lossy(file_content).to_string())
            }
        }
    }
    
    fn create_document_chunks(
        &self,
        document_id: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let settings = self.settings.lock().unwrap();
        let chunk_size = settings.chunk_size;
        let chunk_overlap = settings.chunk_overlap;
        drop(settings);
        
        let conn = Connection::open(&self.db_path)?;
        let mut chunks = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut start = 0;
        let mut chunk_index = 0;
        
        while start < chars.len() {
            let end = std::cmp::min(start + chunk_size, chars.len());
            let chunk_content: String = chars[start..end].iter().collect();
            
            let chunk_id = Uuid::new_v4().to_string();
            chunks.push((
                chunk_id,
                document_id.to_string(),
                chunk_index,
                chunk_content,
                start as i32,
                end as i32,
            ));
            
            chunk_index += 1;
            start = if end >= chars.len() {
                chars.len()
            } else {
                end - chunk_overlap
            };
        }
        
        // Insert chunks into database
        for (id, doc_id, idx, content, start_char, end_char) in chunks {
            conn.execute(
                "INSERT INTO document_chunks (
                    id, document_id, chunk_index, content, start_char, end_char
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, doc_id, idx, content, start_char, end_char],
            )?;
        }
        
        Ok(())
    }
    
    fn queue_embedding_generation(&self, document_id: &str) {
        // TODO: Implement background embedding generation
        println!("Queuing embedding generation for document: {}", document_id);
    }
    
    pub fn get_all_documents(&self) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, file_name, file_path, file_type, file_size, content,
                    created_at, updated_at, access_count, last_accessed, is_cached, metadata
             FROM documents
             ORDER BY created_at DESC"
        )?;
        
        let documents = stmt.query_map([], |row| {
            Ok(Document {
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
                metadata: row.get(11)?,
            })
        })?;
        
        Ok(documents.collect::<Result<Vec<_>, _>>()?)
    }
    
    pub fn delete_document(&self, document_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        
        // Delete from database (chunks will be cascade deleted)
        conn.execute("DELETE FROM documents WHERE id = ?1", params![document_id])?;
        
        // Delete file from storage
        let doc_path = self.storage_path.join(document_id);
        if doc_path.exists() {
            fs::remove_dir_all(doc_path)?;
        }
        
        // Remove from embedding cache
        let mut cache = self.embedding_cache.lock().unwrap();
        cache.remove(document_id);
        
        Ok(())
    }
    
    pub fn search_documents(
        &self,
        query: &str,
        context_document_ids: Vec<String>,
    ) -> Result<Vec<DocumentChunk>, Box<dyn std::error::Error>> {
        // Simple text search for now, will be replaced with semantic search
        let conn = Connection::open(&self.db_path)?;
        
        let placeholders: Vec<String> = context_document_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect();
        
        let sql = if context_document_ids.is_empty() {
            "SELECT id, document_id, chunk_index, content, start_char, end_char, embedding, metadata
             FROM document_chunks
             WHERE content LIKE ?1
             ORDER BY chunk_index
             LIMIT 10".to_string()
        } else {
            format!(
                "SELECT id, document_id, chunk_index, content, start_char, end_char, embedding, metadata
                 FROM document_chunks
                 WHERE content LIKE ?1 AND document_id IN ({})
                 ORDER BY chunk_index
                 LIMIT 10",
                placeholders.join(", ")
            )
        };
        
        let mut stmt = conn.prepare(&sql)?;
        
        let query_pattern = format!("%{}%", query);
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&query_pattern];
        for id in &context_document_ids {
            params.push(id);
        }
        
        let chunks = stmt.query_map(params.as_slice(), |row| {
            Ok(DocumentChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                start_char: row.get(4)?,
                end_char: row.get(5)?,
                embedding: None, // TODO: Deserialize embedding blob
                metadata: row.get(7)?,
            })
        })?;
        
        Ok(chunks.collect::<Result<Vec<_>, _>>()?)
    }
    
    pub fn update_settings(&self, new_settings: RagSettings) -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = self.settings.lock().unwrap();
        *settings = new_settings;
        
        // Save settings to database
        let conn = Connection::open(&self.db_path)?;
        let settings_json = serde_json::to_string(&*settings)?;
        
        conn.execute(
            "INSERT OR REPLACE INTO user_settings (key, value) VALUES ('rag_settings', ?1)",
            params![settings_json],
        )?;
        
        Ok(())
    }
    
    pub fn get_settings(&self) -> RagSettings {
        let settings = self.settings.lock().unwrap();
        settings.clone()
    }
    
    pub fn get_storage_stats(&self) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        
        // Get total documents count
        let doc_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM documents",
            [],
            |row| row.get(0),
        )?;
        
        // Get total storage size
        let total_size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM documents",
            [],
            |row| row.get(0),
        )?;
        
        // Get cached documents count
        let cached_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE is_cached = 1",
            [],
            |row| row.get(0),
        )?;
        
        let mut stats = HashMap::new();
        stats.insert("total_documents".to_string(), serde_json::json!(doc_count));
        stats.insert("total_size_bytes".to_string(), serde_json::json!(total_size));
        stats.insert("total_size_mb".to_string(), serde_json::json!(total_size as f64 / (1024.0 * 1024.0)));
        stats.insert("cached_documents".to_string(), serde_json::json!(cached_count));
        
        let settings = self.settings.lock().unwrap();
        stats.insert("max_cached_documents".to_string(), serde_json::json!(settings.max_cached_documents));
        stats.insert("max_document_size_mb".to_string(), serde_json::json!(settings.max_document_size_mb));
        stats.insert("max_collection_size_gb".to_string(), serde_json::json!(settings.max_collection_size_gb));
        
        Ok(stats)
    }
}