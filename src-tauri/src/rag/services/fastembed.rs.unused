use anyhow::{Result, anyhow};
use fastembed::{EmbeddingModel, FlagEmbedding, InitOptions, EmbeddingBase};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub max_length: usize,
    pub normalize_embeddings: bool,
    pub show_download_progress: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "BAAI/bge-small-en-v1.5".to_string(),
            max_length: 512,
            normalize_embeddings: true,
            show_download_progress: true,
        }
    }
}

#[derive(Clone)]
pub struct EmbeddingService {
    model: Arc<Mutex<Option<FlagEmbedding>>>,
    config: EmbeddingConfig,
    cache_dir: PathBuf,
}

impl EmbeddingService {
    pub fn new(cache_dir: PathBuf, config: Option<EmbeddingConfig>) -> Self {
        let config = config.unwrap_or_default();
        
        Self {
            model: Arc::new(Mutex::new(None)),
            config,
            cache_dir,
        }
    }
    
    pub async fn initialize(&self) -> Result<()> {
        let mut model_guard = self.model.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        
        if model_guard.is_some() {
            return Ok(()); // Already initialized
        }
        
        // Determine the model to use
        let model_type = match self.config.model_name.as_str() {
            "BAAI/bge-small-en-v1.5" => EmbeddingModel::BGESmallENV15,
            "BAAI/bge-base-en-v1.5" => EmbeddingModel::BGEBaseENV15,
            "sentence-transformers/all-MiniLM-L6-v2" => EmbeddingModel::AllMiniLML6V2,
            _ => EmbeddingModel::BGESmallENV15, // Default fallback
        };
        
        // Initialize model with custom options
        let init_options = InitOptions {
            model_name: Some(model_type),
            cache_dir: Some(self.cache_dir.clone()),
            show_download_progress: self.config.show_download_progress,
            ..Default::default()
        };
        
        println!("Initializing embedding model: {}", self.config.model_name);
        println!("Cache directory: {:?}", self.cache_dir);
        
        let model = FlagEmbedding::try_new(init_options)
            .map_err(|e| anyhow!("Failed to initialize embedding model: {}", e))?;
        
        *model_guard = Some(model);
        println!("Embedding model initialized successfully");
        
        Ok(())
    }
    
    pub fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let model_guard = self.model.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        
        let model = model_guard.as_ref()
            .ok_or_else(|| anyhow!("Embedding model not initialized"))?;
        
        let texts_ref: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        
        println!("Embedding {} documents", texts.len());
        
        let embeddings = model.embed(texts_ref, None)
            .map_err(|e| anyhow!("Failed to generate embeddings: {}", e))?;
        
        println!("Generated {} embeddings", embeddings.len());
        
        Ok(embeddings)
    }
    
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let model_guard = self.model.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        
        let model = model_guard.as_ref()
            .ok_or_else(|| anyhow!("Embedding model not initialized"))?;
        
        let embeddings = model.embed(vec![query], None)
            .map_err(|e| anyhow!("Failed to generate query embedding: {}", e))?;
        
        embeddings.into_iter().next()
            .ok_or_else(|| anyhow!("No embedding generated for query"))
    }
    
    pub fn get_dimension(&self) -> Result<usize> {
        // BGE Small EN v1.5 has 384 dimensions
        match self.config.model_name.as_str() {
            "BAAI/bge-small-en-v1.5" => Ok(384),
            "BAAI/bge-base-en-v1.5" => Ok(768),
            "sentence-transformers/all-MiniLM-L6-v2" => Ok(384),
            _ => Ok(384), // Default for BGE Small
        }
    }
    
    pub fn is_initialized(&self) -> bool {
        if let Ok(model_guard) = self.model.lock() {
            model_guard.is_some()
        } else {
            false
        }
    }
    
    pub fn get_config(&self) -> &EmbeddingConfig {
        &self.config
    }
}

// Utility functions for embedding operations
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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

pub fn normalize_embedding(embedding: &mut [f32]) {
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in embedding.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_embedding_service_initialization() {
        let temp_dir = tempdir().unwrap();
        let service = EmbeddingService::new(temp_dir.path().to_path_buf(), None);
        
        // This test would require model download, so we'll skip actual initialization
        assert!(!service.is_initialized());
    }
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
        
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    }
}