// src-tauri/src/audio_loopback/shared/quality_filter.rs

// Sandbox-matching quality estimation functions
pub fn estimate_transcription_confidence(text: &str) -> f32 {
    if text.len() < 3 {
        return 0.1;
    }
    
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.1;
    }
    
    // Check for repetition (lower confidence)
    let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
    let uniqueness_ratio = unique_words.len() as f32 / words.len() as f32;
    
    // Check for common filler words or artifacts
    let filler_count = words.iter()
        .filter(|&&word| {
            let w = word.to_lowercase();
            w == "uh" || w == "um" || w == "ah" || w.len() == 1 || 
            w.contains("[") || w.contains("]") || w.contains("_") || // Whisper artifacts
            w == "crying" || w == "music" || w == "applause" || w == "silence"
        })
        .count();
    
    let filler_ratio = filler_count as f32 / words.len() as f32;
    
    // Heavily penalize transcriptions that are likely audio artifacts
    let text_lower = text.to_lowercase();
    let mut artifact_penalty = 1.0;
    
    // Major penalties for obvious artifacts
    if text_lower.contains("crying") || text_lower.contains("music") || 
       text_lower.contains("applause") || text_lower.contains("silence") ||
       text_lower.starts_with("(") && text_lower.ends_with(")") {
        artifact_penalty = 0.1;
    }
    
    // Medium penalties for repetitive or low-quality text
    if uniqueness_ratio < 0.5 || filler_ratio > 0.3 {
        artifact_penalty *= 0.5;
    }
    
    let base_confidence = 0.7;
    let confidence = base_confidence * uniqueness_ratio * (1.0 - filler_ratio) * artifact_penalty;
    
    confidence.clamp(0.05, 0.95)
}

pub fn is_transcription_quality_ok(text: &str, confidence: f32) -> bool {
    if text.len() < 2 {
        return false;
    }
    
    // Stricter confidence threshold to filter out artifacts
    if confidence < 0.5 {
        return false;
    }
    
    let text_lower = text.to_lowercase().trim().to_string();
    
    // Immediately reject obvious Whisper artifacts
    let artifacts = [
        "crying", "music", "applause", "silence", "laughter",
        "thanks for watching", "subscribe", "like and subscribe",
        "(", ")", "[", "]", "_beg_", "_end_", "_sot_", "_eot_"
    ];
    
    for artifact in &artifacts {
        if text_lower.contains(artifact) {
            return false;
        }
    }
    
    // Reject single words that are likely artifacts
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() == 1 {
        let word = words[0].to_lowercase();
        let single_word_artifacts = [
            "um", "uh", "ah", "hmm", "eh", "oh", "mm", 
            "a", "i", "the", "and", "or", "but", "so"
        ];
        if single_word_artifacts.contains(&word.as_str()) {
            return false;
        }
    }
    
    // Repetition check with stricter threshold
    if words.len() > 3 {
        let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
        let unique_ratio = unique_words.len() as f32 / words.len() as f32;
        if unique_ratio < 0.4 {
            return false;
        }
    }
    
    // Reject text that's mostly punctuation or symbols
    let alpha_chars = text.chars().filter(|c| c.is_alphabetic()).count();
    let total_chars = text.chars().count();
    if total_chars > 0 && (alpha_chars as f32 / total_chars as f32) < 0.5 {
        return false;
    }
    
    true
}