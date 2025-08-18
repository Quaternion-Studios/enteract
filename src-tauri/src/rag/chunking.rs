use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tiktoken_rs::cl100k_base;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub max_chunk_size: usize,
    pub min_chunk_size: usize,
    pub respect_sentence_boundaries: bool,
    pub respect_paragraph_boundaries: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 64,
            max_chunk_size: 800,
            min_chunk_size: 100,
            respect_sentence_boundaries: true,
            respect_paragraph_boundaries: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    pub content: String,
    pub start_char: usize,
    pub end_char: usize,
    pub token_count: usize,
    pub chunk_index: usize,
}

#[derive(Clone)]
pub struct ChunkingService {
    config: ChunkingConfig,
    tokenizer: tiktoken_rs::CoreBPE,
}

impl ChunkingService {
    pub fn new(config: Option<ChunkingConfig>) -> Result<Self> {
        let config = config.unwrap_or_default();
        let tokenizer = cl100k_base().map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;
        
        Ok(Self {
            config,
            tokenizer,
        })
    }
    
    pub fn chunk_text(&self, text: &str) -> Result<Vec<TextChunk>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        
        // First, try intelligent chunking with sentence/paragraph boundaries
        if self.config.respect_paragraph_boundaries || self.config.respect_sentence_boundaries {
            match self.intelligent_chunk(text) {
                Ok(chunks) if !chunks.is_empty() => return Ok(chunks),
                _ => {
                    // Fallback to token-based chunking
                    println!("Falling back to token-based chunking");
                }
            }
        }
        
        // Fallback to token-based chunking
        self.token_based_chunk(text)
    }
    
    fn intelligent_chunk(&self, text: &str) -> Result<Vec<TextChunk>> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_start = 0;
        let mut chunk_index = 0;
        
        // Split by paragraphs first if enabled
        let paragraphs = if self.config.respect_paragraph_boundaries {
            text.split("\n\n").collect::<Vec<_>>()
        } else {
            vec![text]
        };
        
        for paragraph in paragraphs {
            // Split by sentences if enabled
            let sentences = if self.config.respect_sentence_boundaries {
                self.split_sentences(paragraph)
            } else {
                vec![paragraph.to_string()]
            };
            
            for sentence in sentences {
                let sentence_tokens = self.count_tokens(&sentence)?;
                let current_tokens = if current_chunk.is_empty() {
                    0
                } else {
                    self.count_tokens(&current_chunk)?
                };
                
                // Check if adding this sentence would exceed chunk size
                if current_tokens + sentence_tokens > self.config.chunk_size && !current_chunk.is_empty() {
                    // Finalize current chunk
                    if current_tokens >= self.config.min_chunk_size {
                        let chunk_end = current_start + current_chunk.len();
                        chunks.push(TextChunk {
                            content: current_chunk.trim().to_string(),
                            start_char: current_start,
                            end_char: chunk_end,
                            token_count: current_tokens,
                            chunk_index,
                        });
                        chunk_index += 1;
                    }
                    
                    // Start new chunk with overlap
                    let previous_end = current_start + current_chunk.len();
                    current_chunk = self.create_overlap_content(&chunks, &sentence)?;
                    current_start = previous_end.saturating_sub(current_chunk.len().saturating_sub(sentence.len()));
                } else {
                    // Add sentence to current chunk
                    if !current_chunk.is_empty() {
                        current_chunk.push(' ');
                    }
                    current_chunk.push_str(&sentence);
                }
            }
        }
        
        // Add final chunk if it has content
        if !current_chunk.trim().is_empty() {
            let current_tokens = self.count_tokens(&current_chunk)?;
            if current_tokens >= self.config.min_chunk_size {
                chunks.push(TextChunk {
                    content: current_chunk.trim().to_string(),
                    start_char: current_start,
                    end_char: current_start + current_chunk.len(),
                    token_count: current_tokens,
                    chunk_index,
                });
            }
        }
        
        Ok(chunks)
    }
    
    fn token_based_chunk(&self, text: &str) -> Result<Vec<TextChunk>> {
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        let mut chunks = Vec::new();
        let mut chunk_index = 0;
        
        let mut start_idx = 0;
        while start_idx < tokens.len() {
            let end_idx = std::cmp::min(start_idx + self.config.chunk_size, tokens.len());
            let chunk_tokens = &tokens[start_idx..end_idx];
            
            // Decode tokens back to text
            let chunk_text = self.tokenizer.decode(chunk_tokens.to_vec())
                .map_err(|e| anyhow!("Failed to decode tokens: {}", e))?;
            
            if !chunk_text.trim().is_empty() {
                // Calculate character positions (approximate)
                let start_char = self.estimate_char_position(text, start_idx, &tokens)?;
                let end_char = self.estimate_char_position(text, end_idx, &tokens)?;
                
                chunks.push(TextChunk {
                    content: chunk_text.trim().to_string(),
                    start_char,
                    end_char,
                    token_count: chunk_tokens.len(),
                    chunk_index,
                });
                
                chunk_index += 1;
            }
            
            // Move start position with overlap
            start_idx = if end_idx >= tokens.len() {
                tokens.len()
            } else {
                end_idx.saturating_sub(self.config.chunk_overlap)
            };
        }
        
        Ok(chunks)
    }
    
    fn split_sentences(&self, text: &str) -> Vec<String> {
        // Simple sentence splitting - could be enhanced with more sophisticated NLP
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            current_sentence.push(ch);
            
            if matches!(ch, '.' | '!' | '?') {
                // Check if this is likely end of sentence
                if let Some(&next_char) = chars.peek() {
                    if next_char.is_whitespace() || next_char.is_uppercase() {
                        sentences.push(current_sentence.trim().to_string());
                        current_sentence.clear();
                        continue;
                    }
                } else {
                    // End of text
                    sentences.push(current_sentence.trim().to_string());
                    break;
                }
            }
        }
        
        // Add remaining content as final sentence
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
        }
        
        // Filter out very short sentences
        sentences.into_iter()
            .filter(|s| s.len() > 10)
            .collect()
    }
    
    fn create_overlap_content(&self, existing_chunks: &[TextChunk], new_content: &str) -> Result<String> {
        if existing_chunks.is_empty() || self.config.chunk_overlap == 0 {
            return Ok(new_content.to_string());
        }
        
        let last_chunk = &existing_chunks[existing_chunks.len() - 1];
        let last_chunk_tokens = self.count_tokens(&last_chunk.content)?;
        let overlap_size = std::cmp::min(self.config.chunk_overlap, last_chunk_tokens);
        
        if overlap_size == 0 {
            return Ok(new_content.to_string());
        }
        
        // Take last N tokens from previous chunk
        let last_tokens = self.tokenizer.encode_with_special_tokens(&last_chunk.content);
        let overlap_start = last_tokens.len().saturating_sub(overlap_size);
        let overlap_tokens = &last_tokens[overlap_start..];
        
        let overlap_text = self.tokenizer.decode(overlap_tokens.to_vec())
            .map_err(|e| anyhow!("Failed to decode overlap tokens: {}", e))?;
        
        Ok(format!("{} {}", overlap_text.trim(), new_content))
    }
    
    fn count_tokens(&self, text: &str) -> Result<usize> {
        Ok(self.tokenizer.encode_with_special_tokens(text).len())
    }
    
    fn estimate_char_position(&self, original_text: &str, token_idx: usize, all_tokens: &[usize]) -> Result<usize> {
        if token_idx == 0 {
            return Ok(0);
        }
        
        if token_idx >= all_tokens.len() {
            return Ok(original_text.len());
        }
        
        // Decode tokens up to this position to estimate character position
        let tokens_subset = &all_tokens[0..token_idx];
        let decoded_text = self.tokenizer.decode(tokens_subset.to_vec())
            .map_err(|e| anyhow!("Failed to decode tokens for position estimation: {}", e))?;
        
        Ok(decoded_text.len())
    }
    
    pub fn get_config(&self) -> &ChunkingConfig {
        &self.config
    }
    
    pub fn update_config(&mut self, new_config: ChunkingConfig) {
        self.config = new_config;
    }
}

// Document processing utilities
pub fn extract_text_from_pdf(content: &[u8]) -> Result<String> {
    // Simple PDF text extraction - could be enhanced with better PDF libraries
    pdf_extract::extract_text_from_mem(content)
        .map_err(|e| anyhow!("PDF extraction failed: {}", e))
}

pub fn extract_text_from_docx(content: &[u8]) -> Result<String> {
    // TODO: Implement DOCX extraction
    // For now, return placeholder
    Ok("DOCX text extraction not yet implemented".to_string())
}

pub fn clean_text(text: &str) -> String {
    // Basic text cleaning
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .filter(|&c| c.is_ascii() || c.is_whitespace() || c.is_alphanumeric())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_service_creation() {
        let service = ChunkingService::new(None);
        assert!(service.is_ok());
    }
    
    #[test]
    fn test_sentence_splitting() {
        let service = ChunkingService::new(None).unwrap();
        let text = "This is sentence one. This is sentence two! Is this sentence three?";
        let sentences = service.split_sentences(text);
        
        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("sentence one"));
        assert!(sentences[1].contains("sentence two"));
        assert!(sentences[2].contains("sentence three"));
    }
    
    #[test]
    fn test_text_cleaning() {
        let dirty_text = "  This   is  \n\n messy    text.  \t\n  ";
        let clean = clean_text(dirty_text);
        assert_eq!(clean, "This is messy text.");
    }
    
    #[test]
    fn test_token_counting() {
        let service = ChunkingService::new(None).unwrap();
        let text = "Hello world";
        let count = service.count_tokens(text).unwrap();
        assert!(count > 0);
    }
}