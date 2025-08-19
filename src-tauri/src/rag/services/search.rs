use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, STORED, TEXT, FAST, Field, Value};
use tantivy::{Index, IndexWriter, IndexReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub bm25_weight: f32,
    pub vector_weight: f32,
    pub max_results: usize,
    pub min_score_threshold: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            bm25_weight: 0.7,
            vector_weight: 0.3,
            max_results: 50,
            min_score_threshold: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_id: String,
    pub document_id: String,
    pub content: String,
    pub score: f32,
    pub bm25_score: f32,
    pub vector_score: f32,
    pub metadata: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone)]
pub struct SearchService {
    index: Arc<Index>,
    reader: Arc<IndexReader>,
    writer: Arc<Mutex<Option<IndexWriter>>>,
    schema: Schema,
    fields: SearchFields,
    config: SearchConfig,
}

#[derive(Debug, Clone)]
pub struct SearchFields {
    pub chunk_id: Field,
    pub document_id: Field,
    pub content: Field,
    pub embedding: Field,
    pub metadata: Field,
}

impl SearchService {
    pub fn new(index_dir: PathBuf, config: Option<SearchConfig>) -> Result<Self> {
        let config = config.unwrap_or_default();
        
        // Build schema
        let mut schema_builder = Schema::builder();
        
        let chunk_id = schema_builder.add_text_field("chunk_id", STORED | FAST);
        let document_id = schema_builder.add_text_field("document_id", STORED | FAST);
        let content = schema_builder.add_text_field("content", TEXT | STORED);
        let embedding = schema_builder.add_bytes_field("embedding", STORED | FAST);
        let metadata = schema_builder.add_text_field("metadata", STORED);
        
        let schema = schema_builder.build();
        let fields = SearchFields {
            chunk_id,
            document_id,
            content,
            embedding,
            metadata,
        };
        
        // Create or open index
        std::fs::create_dir_all(&index_dir)?;
        
        // Clean up potential stale lock files
        let lock_file = index_dir.join(".tantivy-writer.lock");
        if lock_file.exists() {
            println!("Found potential stale lock file, attempting to remove: {:?}", lock_file);
            let _ = std::fs::remove_file(&lock_file); // Ignore errors, might be in use
        }
        
        let index = if index_dir.join("meta.json").exists() {
            Index::open_in_dir(&index_dir)?
        } else {
            Index::create_in_dir(&index_dir, schema.clone())?
        };
        
        // Set up reader with auto-reload
        let reader = index
            .reader_builder()
            .try_into()?;
        
        Ok(Self {
            index: Arc::new(index),
            reader: Arc::new(reader),
            writer: Arc::new(Mutex::new(None)),
            schema,
            fields,
            config,
        })
    }
    
    pub fn initialize_writer(&self) -> Result<()> {
        let mut writer_guard = self.writer.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        
        if writer_guard.is_some() {
            return Ok(());
        }
        
        // Create index writer with a reasonable heap size (50MB)
        let writer = self.index.writer(50_000_000)?;
        *writer_guard = Some(writer);
        
        Ok(())
    }
    
    pub fn add_documents(&self, chunks: Vec<DocumentChunk>) -> Result<()> {
        let mut writer_guard = self.writer.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        let writer = writer_guard.as_mut().ok_or_else(|| anyhow!("Writer not initialized"))?;
        
        for chunk in chunks {
            let mut doc = tantivy::doc!();
            
            doc.add_text(self.fields.chunk_id, &chunk.id);
            doc.add_text(self.fields.document_id, &chunk.document_id);
            doc.add_text(self.fields.content, &chunk.content);
            
            if let Some(embedding) = chunk.embedding {
                let embedding_bytes = embedding_to_bytes(&embedding);
                doc.add_bytes(self.fields.embedding, embedding_bytes);
            }
            
            if let Some(metadata) = chunk.metadata {
                doc.add_text(self.fields.metadata, &metadata);
            }
            
            writer.add_document(doc)?;
        }
        
        Ok(())
    }
    
    pub fn commit(&self) -> Result<()> {
        let mut writer_guard = self.writer.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        if let Some(writer) = writer_guard.as_mut() {
            writer.commit()?;
        }
        Ok(())
    }
    
    pub fn search_bm25(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.fields.content]);
        
        let query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            
            let chunk_id = retrieved_doc
                .get_first(self.fields.chunk_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let document_id = retrieved_doc
                .get_first(self.fields.document_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let content = retrieved_doc
                .get_first(self.fields.content)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let metadata = retrieved_doc
                .get_first(self.fields.metadata)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            results.push(SearchResult {
                chunk_id,
                document_id,
                content,
                score,
                bm25_score: score,
                vector_score: 0.0,
                metadata,
                title: None,
            });
        }
        
        Ok(results)
    }
    
    pub fn search_vector(&self, _query_embedding: &[f32], _limit: usize) -> Result<Vec<SearchResult>> {
        // For now, return empty results since vector search is complex with current Tantivy API
        // This can be implemented later with proper HNSW index
        println!("Vector search not yet implemented - falling back to BM25");
        Ok(Vec::new())
    }
    
    pub fn hybrid_search(&self, query: &str, query_embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        // Get BM25 results
        let bm25_results = self.search_bm25(query, limit * 2)?; // Get more for fusion
        
        // Get vector results
        let vector_results = self.search_vector(query_embedding, limit * 2)?;
        
        // If vector search is not implemented, just return BM25 results
        if vector_results.is_empty() {
            eprintln!("🔍 Vector search returned no results, using BM25 only");
            let mut results = bm25_results;
            results.truncate(limit);
            return Ok(results);
        }
        
        // Perform reciprocal rank fusion
        let fused_results = self.reciprocal_rank_fusion(bm25_results, vector_results, limit)?;
        
        Ok(fused_results)
    }
    
    fn reciprocal_rank_fusion(
        &self,
        bm25_results: Vec<SearchResult>,
        vector_results: Vec<SearchResult>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let k = 60.0; // RRF parameter
        let mut score_map: HashMap<String, (SearchResult, f32)> = HashMap::new();
        
        // Process BM25 results
        for (rank, mut result) in bm25_results.into_iter().enumerate() {
            let rrf_score = self.config.bm25_weight / (k + rank as f32 + 1.0);
            result.score = rrf_score;
            score_map.insert(result.chunk_id.clone(), (result, rrf_score));
        }
        
        // Process vector results and merge
        for (rank, result) in vector_results.into_iter().enumerate() {
            let rrf_score = self.config.vector_weight / (k + rank as f32 + 1.0);
            
            if let Some((mut existing_result, existing_score)) = score_map.remove(&result.chunk_id) {
                // Merge scores
                existing_result.score = existing_score + rrf_score;
                existing_result.vector_score = result.vector_score;
                score_map.insert(result.chunk_id.clone(), (existing_result, existing_score + rrf_score));
            } else {
                let mut new_result = result;
                new_result.score = rrf_score;
                score_map.insert(new_result.chunk_id.clone(), (new_result, rrf_score));
            }
        }
        
        // Sort by final score
        let mut final_results: Vec<SearchResult> = score_map.into_values().map(|(result, _)| result).collect();
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        final_results.truncate(limit);
        
        Ok(final_results)
    }
    
    pub fn delete_document(&self, document_id: &str) -> Result<()> {
        let mut writer_guard = self.writer.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        let writer = writer_guard.as_mut().ok_or_else(|| anyhow!("Writer not initialized"))?;
        
        let term = tantivy::Term::from_field_text(self.fields.document_id, document_id);
        writer.delete_term(term);
        
        Ok(())
    }
    
    pub fn clear_index(&self) -> Result<()> {
        let mut writer_guard = self.writer.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        let writer = writer_guard.as_mut().ok_or_else(|| anyhow!("Writer not initialized"))?;
        
        writer.delete_all_documents()?;
        writer.commit()?;
        
        Ok(())
    }
    
    pub fn close_writer(&self) -> Result<()> {
        let mut writer_guard = self.writer.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        if let Some(mut writer) = writer_guard.take() {
            writer.commit()?;
            println!("IndexWriter closed and committed");
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: Option<String>,
}

// Utility functions
fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &value in embedding {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn bytes_to_embedding(bytes: &[u8]) -> Result<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return Err(anyhow!("Invalid embedding bytes length"));
    }
    
    let mut embedding = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        embedding.push(value);
    }
    
    Ok(embedding)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_embedding_serialization() {
        let embedding = vec![1.0, 2.0, 3.0, 4.0];
        let bytes = embedding_to_bytes(&embedding);
        let recovered = bytes_to_embedding(&bytes).unwrap();
        
        for (original, recovered) in embedding.iter().zip(recovered.iter()) {
            assert!((original - recovered).abs() < 1e-6);
        }
    }
    
    #[test]
    fn test_search_service_creation() {
        let temp_dir = tempdir().unwrap();
        let service = SearchService::new(temp_dir.path().to_path_buf(), None);
        assert!(service.is_ok());
    }
}