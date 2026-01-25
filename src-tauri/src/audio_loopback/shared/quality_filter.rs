// src-tauri/src/audio_loopback/shared/quality_filter.rs
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

lazy_static! {
    /// Global state for tracking partial transcriptions
    static ref PARTIAL_TEXT_STATE: Arc<Mutex<PartialTextBuffer>> = Arc::new(Mutex::new(PartialTextBuffer::new()));
}

/// Buffer for managing partial transcriptions across segments
struct PartialTextBuffer {
    pending: String,
    last_update: std::time::Instant,
    timeout_ms: u64,
}

impl PartialTextBuffer {
    fn new() -> Self {
        Self {
            pending: String::new(),
            last_update: std::time::Instant::now(),
            timeout_ms: 5000, // Clear pending after 5s of silence
        }
    }

    fn is_expired(&self) -> bool {
        self.last_update.elapsed().as_millis() as u64 > self.timeout_ms
    }

    fn add_pending(&mut self, text: &str) {
        self.pending.push_str(text);
        self.pending.push(' ');
        self.last_update = std::time::Instant::now();
    }

    fn get_and_clear_if_expired(&mut self) -> String {
        if self.is_expired() {
            let result = self.pending.clone();
            self.pending.clear();
            result
        } else {
            String::new()
        }
    }

    fn take_pending(&mut self) -> String {
        let result = self.pending.clone();
        self.pending.clear();
        self.last_update = std::time::Instant::now();
        result
    }
}

/// Detect if text is an incomplete sentence
///
/// Incomplete sentences lack:
/// - Ending punctuation (. ! ?)
/// - May end mid-word (no space after last word)
pub fn is_incomplete_sentence(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }

    let trimmed = text.trim();

    // Check for ending punctuation
    let has_ending_punctuation = trimmed.ends_with('.')
        || trimmed.ends_with('!')
        || trimmed.ends_with('?')
        || trimmed.ends_with(','); // Include comma as potential pause

    if has_ending_punctuation {
        return false;
    }

    // No ending punctuation = incomplete
    true
}

/// Merge transcription with any pending partial text
///
/// Returns (merged_text, is_complete)
pub fn merge_with_pending(new_text: &str) -> (String, bool) {
    if let Ok(mut state) = PARTIAL_TEXT_STATE.lock() {
        // Check if pending text expired (long silence)
        let expired_pending = state.get_and_clear_if_expired();
        if !expired_pending.is_empty() {
            // Pending text expired - treat as separate
            // Process the new text independently
        }

        let pending = state.take_pending();

        if pending.is_empty() {
            // No pending text - check if new text is complete
            let is_complete = !is_incomplete_sentence(new_text);

            if is_complete {
                (new_text.to_string(), true)
            } else {
                // Incomplete - buffer for next chunk
                state.add_pending(new_text);
                (String::new(), false)
            }
        } else {
            // Merge pending with new text
            let merged = format!("{} {}", pending.trim(), new_text.trim());

            let is_complete = !is_incomplete_sentence(&merged);

            if is_complete {
                (merged, true)
            } else {
                // Still incomplete - keep buffering
                state.add_pending(&merged);
                (String::new(), false)
            }
        }
    } else {
        // Couldn't acquire lock - return as-is
        (new_text.to_string(), !is_incomplete_sentence(new_text))
    }
}

/// Enhanced confidence estimation with multiple factors
///
/// Factors:
/// - Audio energy (louder = more confident)
/// - Text coherence (complete sentences = higher)
/// - Word uniqueness (repetition = lower)
/// - Whisper's own confidence if available
pub fn estimate_confidence_enhanced(
    text: &str,
    audio_rms: Option<f32>,
    has_punctuation: bool,
) -> f32 {
    if text.len() < 3 {
        return 0.1;
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.1;
    }

    // Base confidence
    let mut confidence = 0.7;

    // Factor 1: Audio energy
    if let Some(rms) = audio_rms {
        // Normalize RMS to confidence boost
        // RMS > 0.01 = good, RMS < 0.003 = poor
        let energy_factor = (rms / 0.01).clamp(0.5, 1.2);
        confidence *= energy_factor;
    }

    // Factor 2: Text coherence (sentence completeness)
    if has_punctuation {
        confidence *= 1.1; // Boost for complete sentences
    } else {
        confidence *= 0.9; // Slight penalty for incomplete
    }

    // Factor 3: Word uniqueness (repetition check)
    let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
    let uniqueness_ratio = unique_words.len() as f32 / words.len() as f32;
    confidence *= uniqueness_ratio;

    // Factor 4: Artifact detection
    let filler_count = words.iter()
        .filter(|&&word| {
            let w = word.to_lowercase();
            w == "uh" || w == "um" || w == "ah" || w.len() == 1 ||
            w.contains("[") || w.contains("]") || w.contains("_") ||
            w == "crying" || w == "music" || w == "applause" || w == "silence"
        })
        .count();

    let filler_ratio = filler_count as f32 / words.len() as f32;
    confidence *= (1.0 - filler_ratio);

    // Heavy penalties for obvious artifacts
    let text_lower = text.to_lowercase();
    if text_lower.contains("crying") || text_lower.contains("music") ||
       text_lower.contains("applause") || text_lower.contains("silence") ||
       (text_lower.starts_with('(') && text_lower.ends_with(')')) {
        confidence *= 0.1;
    }

    confidence.clamp(0.05, 0.95)
}

// Sandbox-matching quality estimation functions (legacy)
pub fn estimate_transcription_confidence(text: &str) -> f32 {
    estimate_confidence_enhanced(text, None, !is_incomplete_sentence(text))
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
        // Background sounds
        "crying", "music", "applause", "silence", "laughter",
        "[music]", "[blank_audio]", "[inaudible]",
        "(music)", "(silence)", "(background noise)",
        "music playing", "upbeat music", "funky music",
        "electronic beeping", "beeping", "crashing", "swoosh",

        // YouTube/video artifacts
        "thanks for watching", "subscribe", "like and subscribe",
        "don't forget to subscribe", "hit that bell",
        "check the description", "link in description",

        // Whisper internal tokens
        "(", ")", "[", "]", "_beg_", "_end_", "_sot_", "_eot_",
        "<|", "|>",
    ];

    for artifact in &artifacts {
        if text_lower.contains(artifact) {
            return false;
        }
    }

    // Reject single words that are likely hallucinations on silence
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() == 1 {
        let word = words[0].to_lowercase();
        let single_word_artifacts = [
            // Common filler words
            "um", "uh", "ah", "hmm", "eh", "oh", "mm",
            // Articles and connectors (when alone)
            "a", "i", "the", "and", "or", "but", "so",
            // Common single-word hallucinations
            "you", "bye", "okay", "yes", "no", "yeah", "well",
            "thank", "thanks", "hello", "hi",
        ];
        if single_word_artifacts.contains(&word.as_str()) {
            return false;
        }
    }

    // Reject common two-word hallucinations
    if words.len() == 2 {
        let phrase = text_lower.trim();
        let two_word_artifacts = [
            "thank you", "you know", "i mean", "you see",
            "bye bye", "okay bye", "see you",
        ];
        if two_word_artifacts.contains(&phrase) {
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