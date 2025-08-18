use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            model_name: "simple-tfidf".to_string(),
            max_length: 512,
            normalize_embeddings: true,
            show_download_progress: false,
        }
    }
}

#[derive(Clone)]
pub struct EmbeddingService {
    config: EmbeddingConfig,
    cache_dir: PathBuf,
    vocabulary: Arc<Mutex<HashMap<String, usize>>>,
    idf_weights: Arc<Mutex<HashMap<String, f32>>>,
}

impl EmbeddingService {
    pub fn new(config: EmbeddingConfig, cache_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            config,
            cache_dir,
            vocabulary: Arc::new(Mutex::new(HashMap::new())),
            idf_weights: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        println!("Initialized simple TF-IDF embedding service");
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        true // Simple TF-IDF is always "initialized"
    }

    pub fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        println!("Creating embeddings for {} documents using TF-IDF", texts.len());
        
        // Simple TF-IDF implementation
        let mut embeddings = Vec::new();
        let mut all_terms = std::collections::HashSet::new();
        
        // Extract all terms
        let documents: Vec<Vec<String>> = texts.iter()
            .map(|text| self.tokenize(text))
            .collect();
        
        for doc in &documents {
            for term in doc {
                all_terms.insert(term.clone());
            }
        }
        
        let vocab: Vec<String> = all_terms.into_iter().collect();
        let vocab_size = vocab.len().min(300); // Limit vocabulary size for performance
        
        // Calculate IDF weights
        let mut idf_weights = HashMap::new();
        for term in &vocab[..vocab_size] {
            let doc_freq = documents.iter()
                .filter(|doc| doc.contains(term))
                .count();
            
            if doc_freq > 0 {
                let idf = ((documents.len() as f32) / (doc_freq as f32)).ln();
                idf_weights.insert(term.clone(), idf);
            }
        }
        
        // Create TF-IDF vectors
        for doc in documents {
            let mut embedding = vec![0.0; vocab_size];
            let doc_len = doc.len() as f32;
            
            for (i, term) in vocab[..vocab_size].iter().enumerate() {
                let tf = doc.iter().filter(|&t| t == term).count() as f32 / doc_len;
                let idf = idf_weights.get(term).unwrap_or(&0.0);
                embedding[i] = tf * idf;
            }
            
            // Normalize if requested
            if self.config.normalize_embeddings {
                let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for val in &mut embedding {
                        *val /= norm;
                    }
                }
            }
            
            embeddings.push(embedding);
        }
        
        Ok(embeddings)
    }

    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        // For query embedding, we'll use the same approach but with a single document
        let embeddings = self.embed_documents(vec![query.to_string()])?;
        embeddings.into_iter().next()
            .ok_or_else(|| anyhow!("Failed to create query embedding"))
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        // Simple tokenization - split on whitespace and punctuation
        text.to_lowercase()
            .split_whitespace()
            .map(|word| {
                word.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|word| word.len() > 2) // Filter out very short words
            .take(self.config.max_length)
            .collect()
    }
}