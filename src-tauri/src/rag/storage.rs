use anyhow::Result;
use rusqlite::{Connection, params, OptionalExtension};
use std::path::PathBuf;
use chrono::Utc;

use super::types::{EnhancedDocument, EnhancedDocumentChunk, StorageStats};

pub struct RagStorage {
    db_path: PathBuf,
}

impl RagStorage {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let storage = Self { db_path };
        storage.initialize_database()?;
        Ok(storage)
    }

    pub fn initialize_database(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        // Create documents table
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
                is_cached BOOLEAN DEFAULT FALSE,
                embedding_status TEXT DEFAULT 'pending',
                chunk_count INTEGER DEFAULT 0,
                metadata TEXT,
                content_hash TEXT
            )",
            [],
        )?;

        // Create chunks table
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
                FOREIGN KEY (document_id) REFERENCES enhanced_documents (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create indices for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_document_id ON enhanced_document_chunks(document_id)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_embedding_status ON enhanced_documents(embedding_status)",
            [],
        )?;

        Ok(())
    }

    pub fn save_document(&self, document: &EnhancedDocument) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        conn.execute(
            "INSERT OR REPLACE INTO enhanced_documents 
             (id, file_name, file_path, file_type, file_size, content, created_at, updated_at, 
              access_count, last_accessed, is_cached, embedding_status, chunk_count, metadata, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                document.is_cached,
                document.embedding_status,
                document.chunk_count,
                document.metadata,
                document.content_hash
            ],
        )?;

        Ok(())
    }

    pub fn get_document(&self, id: &str) -> Result<Option<EnhancedDocument>> {
        let conn = Connection::open(&self.db_path)?;
        
        let mut stmt = conn.prepare(
            "SELECT id, file_name, file_path, file_type, file_size, content, created_at, updated_at,
                    access_count, last_accessed, is_cached, embedding_status, chunk_count, metadata, content_hash
             FROM enhanced_documents WHERE id = ?1"
        )?;

        let document = stmt.query_row(params![id], |row| {
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
                is_cached: row.get(10)?,
                embedding_status: row.get(11)?,
                chunk_count: row.get(12)?,
                metadata: row.get(13)?,
                content_hash: row.get(14)?,
            })
        }).optional()?;

        Ok(document)
    }

    pub fn get_all_documents(&self) -> Result<Vec<EnhancedDocument>> {
        let conn = Connection::open(&self.db_path)?;
        
        let mut stmt = conn.prepare(
            "SELECT id, file_name, file_path, file_type, file_size, content, created_at, updated_at,
                    access_count, last_accessed, is_cached, embedding_status, chunk_count, metadata, content_hash
             FROM enhanced_documents ORDER BY created_at DESC"
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
                is_cached: row.get(10)?,
                embedding_status: row.get(11)?,
                chunk_count: row.get(12)?,
                metadata: row.get(13)?,
                content_hash: row.get(14)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(documents)
    }

    pub fn delete_document(&self, id: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        // Delete chunks first due to foreign key constraint
        conn.execute("DELETE FROM enhanced_document_chunks WHERE document_id = ?1", params![id])?;
        conn.execute("DELETE FROM enhanced_documents WHERE id = ?1", params![id])?;
        
        Ok(())
    }

    pub fn save_chunks(&self, chunks: &[EnhancedDocumentChunk]) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let tx = conn.unchecked_transaction()?;

        for chunk in chunks {
            let embedding_blob: Option<Vec<u8>> = chunk.embedding.as_ref().map(|emb| {
                emb.iter().flat_map(|f| f.to_le_bytes()).collect()
            });

            tx.execute(
                "INSERT OR REPLACE INTO enhanced_document_chunks 
                 (id, document_id, chunk_index, content, start_char, end_char, token_count, embedding, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    chunk.id,
                    chunk.document_id,
                    chunk.chunk_index,
                    chunk.content,
                    chunk.start_char,
                    chunk.end_char,
                    chunk.token_count,
                    embedding_blob,
                    chunk.metadata
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_chunks_for_document(&self, document_id: &str) -> Result<Vec<EnhancedDocumentChunk>> {
        let conn = Connection::open(&self.db_path)?;
        
        let mut stmt = conn.prepare(
            "SELECT id, document_id, chunk_index, content, start_char, end_char, token_count, embedding, metadata
             FROM enhanced_document_chunks WHERE document_id = ?1 ORDER BY chunk_index"
        )?;

        let chunks = stmt.query_map(params![document_id], |row| {
            let embedding_blob: Option<Vec<u8>> = row.get(7)?;
            let embedding = embedding_blob.map(|blob| {
                blob.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect()
            });

            Ok(EnhancedDocumentChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                start_char: row.get(4)?,
                end_char: row.get(5)?,
                token_count: row.get(6)?,
                embedding,
                similarity_score: None,
                bm25_score: None,
                metadata: row.get(8)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(chunks)
    }

    pub fn update_document_embedding_status(&self, document_id: &str, status: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        conn.execute(
            "UPDATE enhanced_documents SET embedding_status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, Utc::now().to_rfc3339(), document_id],
        )?;

        Ok(())
    }

    pub fn get_storage_stats(&self) -> Result<StorageStats> {
        let conn = Connection::open(&self.db_path)?;
        
        let document_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM enhanced_documents",
            [],
            |row| row.get(0)
        )?;

        let chunk_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM enhanced_document_chunks",
            [],
            |row| row.get(0)
        )?;

        // Calculate database file size
        let db_size_bytes = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(StorageStats {
            total_documents: document_count,
            total_chunks: chunk_count,
            database_size_mb: db_size_bytes as f64 / (1024.0 * 1024.0),
            storage_size_mb: 0.0, // Will be calculated elsewhere
            index_size_mb: 0.0,   // Will be calculated elsewhere
            cache_size_mb: 0.0,   // Will be calculated elsewhere
            last_updated: Utc::now().to_rfc3339(),
        })
    }
}