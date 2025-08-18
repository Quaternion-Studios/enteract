use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub max_length: usize,
    pub normalize_embeddings: bool,
    pub embedding_dimension: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "simple-text-embedding".to_string(),
            max_length: 512,
            normalize_embeddings: true,
            embedding_dimension: 384, // Match BGE-small dimensions
        }
    }
}

/// Simple embedding service that generates deterministic embeddings based on text features
/// This is a placeholder that can be replaced with a real embedding model later
#[derive(Clone)]
pub struct SimpleEmbeddingService {
    config: EmbeddingConfig,
    cache_dir: PathBuf,
    cache: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    initialized: Arc<Mutex<bool>>,
}

impl SimpleEmbeddingService {
    pub fn new(cache_dir: PathBuf, config: Option<EmbeddingConfig>) -> Self {
        let config = config.unwrap_or_default();
        
        Self {
            config,
            cache_dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
            initialized: Arc::new(Mutex::new(false)),
        }
    }
    
    pub async fn initialize(&self) -> Result<()> {
        let mut initialized = self.initialized.lock().map_err(|e| anyhow!("Mutex lock failed: {}", e))?;
        
        if *initialized {
            return Ok(());
        }
        
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&self.cache_dir)?;
        
        *initialized = true;
        println!("Simple embedding service initialized (dimension: {})", self.config.embedding_dimension);
        
        Ok(())
    }
    
    pub fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        
        for text in texts {
            let embedding = self.generate_embedding(&text)?;
            embeddings.push(embedding);
        }
        
        Ok(embeddings)
    }
    
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        self.generate_embedding(query)
    }
    
    /// Generate a deterministic embedding based on text features
    /// This is a simplified approach that creates embeddings based on:
    /// - Character n-grams
    /// - Word frequencies
    /// - Text statistics
    fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        if let Ok(cache) = self.cache.lock() {
            if let Some(cached) = cache.get(text) {
                return Ok(cached.clone());
            }
        }
        
        let dimension = self.config.embedding_dimension;
        let mut embedding = vec![0.0_f32; dimension];
        
        // Normalize and clean text
        let text_lower = text.to_lowercase();
        let chars: Vec<char> = text_lower.chars().collect();
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        
        if chars.is_empty() {
            return Ok(embedding);
        }
        
        // Feature 1: Character trigrams (first third of dimensions)
        let trigram_dims = dimension / 3;
        for i in 0..chars.len().saturating_sub(2) {
            let trigram = format!("{}{}{}", chars[i], chars[i+1], chars[i+2]);
            let hash = self.hash_string(&trigram);
            let idx = (hash % trigram_dims as u64) as usize;
            embedding[idx] += 1.0;
        }
        
        // Feature 2: Word unigrams and bigrams (second third of dimensions)
        let word_dims = dimension / 3;
        let word_offset = trigram_dims;
        
        for word in &words {
            let hash = self.hash_string(word);
            let idx = word_offset + (hash % word_dims as u64) as usize;
            embedding[idx] += 1.0;
        }
        
        // Word bigrams
        for i in 0..words.len().saturating_sub(1) {
            let bigram = format!("{} {}", words[i], words[i+1]);
            let hash = self.hash_string(&bigram);
            let idx = word_offset + (hash % word_dims as u64) as usize;
            embedding[idx] += 0.5; // Lower weight for bigrams
        }
        
        // Feature 3: Statistical features (last third of dimensions)
        let stat_offset = 2 * (dimension / 3);
        let remaining_dims = dimension - stat_offset;
        
        // Text length features
        embedding[stat_offset] = (text.len() as f32).ln();
        embedding[stat_offset + 1] = (words.len() as f32).ln();
        
        // Character distribution features
        let mut char_counts = HashMap::new();
        for c in &chars {
            *char_counts.entry(*c).or_insert(0.0) += 1.0;
        }
        
        // Vowel ratio
        let vowels = ['a', 'e', 'i', 'o', 'u'];
        let vowel_count: f32 = vowels.iter()
            .map(|v| char_counts.get(v).unwrap_or(&0.0))
            .sum();
        embedding[stat_offset + 2] = vowel_count / chars.len() as f32;
        
        // Digit ratio
        let digit_count = chars.iter().filter(|c| c.is_numeric()).count() as f32;
        embedding[stat_offset + 3] = digit_count / chars.len() as f32;
        
        // Punctuation ratio
        let punct_count = chars.iter().filter(|c| c.is_ascii_punctuation()).count() as f32;
        embedding[stat_offset + 4] = punct_count / chars.len() as f32;
        
        // Average word length
        if !words.is_empty() {
            let avg_word_len = words.iter().map(|w| w.len()).sum::<usize>() as f32 / words.len() as f32;
            embedding[stat_offset + 5] = avg_word_len;
        }
        
        // Semantic hashing for remaining dimensions
        for i in 6..remaining_dims {
            let seed = format!("{}_{}", text, i);
            let hash = self.hash_string(&seed);
            embedding[stat_offset + i] = ((hash % 1000) as f32 / 1000.0) - 0.5;
        }
        
        // Normalize if configured
        if self.config.normalize_embeddings {
            normalize_embedding(&mut embedding);
        }
        
        // Cache the result
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(text.to_string(), embedding.clone());
        }
        
        Ok(embedding)
    }
    
    /// Simple hash function for strings
    fn hash_string(&self, s: &str) -> u64 {
        let mut hash = 5381u64;
        for byte in s.bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u64);
        }
        hash
    }
    
    pub fn get_dimension(&self) -> Result<usize> {
        Ok(self.config.embedding_dimension)
    }
    
    pub fn is_initialized(&self) -> bool {
        if let Ok(initialized) = self.initialized.lock() {
            *initialized
        } else {
            false
        }
    }
    
    pub fn get_config(&self) -> &EmbeddingConfig {
        &self.config
    }
}

// Utility functions
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
        let service = SimpleEmbeddingService::new(temp_dir.path().to_path_buf(), None);
        
        assert!(!service.is_initialized());
        service.initialize().await.unwrap();
        assert!(service.is_initialized());
    }
    
    #[test]
    fn test_embedding_generation() {
        let temp_dir = tempdir().unwrap();
        let service = SimpleEmbeddingService::new(temp_dir.path().to_path_buf(), None);
        
        let text = "Hello world";
        let embedding = service.generate_embedding(text).unwrap();
        
        assert_eq!(embedding.len(), 384);
        
        // Check that embeddings are deterministic
        let embedding2 = service.generate_embedding(text).unwrap();
        assert_eq!(embedding, embedding2);
        
        // Check that different texts produce different embeddings
        let different_text = "Goodbye world";
        let different_embedding = service.generate_embedding(different_text).unwrap();
        assert_ne!(embedding, different_embedding);
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
    
    #[test]
    fn test_normalization() {
        let mut embedding = vec![3.0, 4.0];
        normalize_embedding(&mut embedding);
        
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }
}