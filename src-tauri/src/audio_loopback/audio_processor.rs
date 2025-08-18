// src-tauri/src/audio_loopback/audio_processor.rs
// use crate::audio_loopback::quality_filter::{estimate_transcription_confidence, is_transcription_quality_ok};
use anyhow::Result;
use tauri::{AppHandle, Emitter};
use base64::prelude::*;
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;

// Audio processing for transcription with improved quality filtering
#[tauri::command]
pub async fn process_audio_for_transcription(
    audio_data: Vec<u8>,
    sample_rate: u32,
    app_handle: AppHandle
) -> Result<String, String> {
    // First process the audio through our pipeline to match Python's fast_audio_process
    // println!("[PROCESS] Input: {} bytes, {} Hz", audio_data.len(), sample_rate); // Commented out: Audio loopback is working, reducing console noise for debugging focus
    
    let processed_samples = process_audio_chunk(
        &audio_data,
        16,  // We're receiving PCM16
        2,   // Stereo input expected
        sample_rate,
        16000  // Target Whisper sample rate
    );
    
    // println!("[PROCESS] Output: {} samples at 16kHz", processed_samples.len()); // Commented out: Audio loopback is working, reducing console noise for debugging focus
    
    // Check minimum audio length (1.5 seconds at 16kHz)
    let min_samples = (16000.0 * 1.5) as usize;
    if processed_samples.len() < min_samples {
        // println!("[PROCESS] Too short: {} samples < {} required", processed_samples.len(), min_samples); // Commented out: Audio loopback is working, reducing console noise for debugging focus
        return Ok("".to_string());
    }
    
    // Calculate RMS on processed samples
    let rms = (processed_samples.iter().map(|&x| x * x).sum::<f32>() / processed_samples.len() as f32).sqrt();
    let db_level = if rms > 0.0 { 20.0 * rms.log10() } else { -60.0 };
    
    // Python checks RMS on int16 samples: rms < 100 
    // For int16, RMS of 100 = 100/32768 = 0.00305 in float32
    if rms < 0.00305 {
        log_transcription_debug("[PROCESS] Audio too quiet - skipping", rms, db_level);
        return Ok("".to_string());
    }
    
    // Convert processed samples back to PCM16 bytes for Whisper
    let pcm16_samples: Vec<i16> = processed_samples.iter()
        .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect();
    
    let pcm16_bytes: Vec<u8> = pcm16_samples.iter()
        .flat_map(|&sample| sample.to_le_bytes())
        .collect();
    
    // SIMPLIFIED APPROACH: Use file-based method with improved quality filtering
    // The direct method was hanging, so let's use what works and fix the quality filtering
    log_transcription_debug("[MAIN] Using file-based transcription with improved filtering...", rms, db_level);
    
    let audio_base64 = base64::prelude::BASE64_STANDARD.encode(&pcm16_bytes);
    
    // Load settings to get the selected loopback whisper model
    let model_size = match crate::audio_loopback::settings::load_general_settings().await {
        Ok(Some(settings)) => {
            if let Some(model) = settings.get("loopbackWhisperModel") {
                if let Some(model_str) = model.as_str() {
                    model_str.to_string()
                } else {
                    "small".to_string() // Default for loopback - same as microphone
                }
            } else {
                "small".to_string() // Default for loopback - same as microphone
            }
        }
        Ok(None) => "small".to_string(), // No settings found, use default
        Err(_) => "small".to_string() // Error loading settings, use default
    };
    
    // println!("[AUDIO_PROCESSOR] Using Whisper model: {}", model_size); // Commented out: Audio loopback is working, reducing console noise for debugging focus
    
    let config = crate::speech::WhisperModelConfig {
        modelSize: model_size,
        language: Some("en".to_string()),
        enableVad: false,  // Matching Python script
        silenceThreshold: 0.01,
        maxSegmentLength: 30,
    };
    
    match crate::speech::transcribe_audio_base64(audio_base64, config).await {
        Ok(result) => {
            let text = result.text.trim();
            log_transcription_debug(&format!("[MAIN] Raw Whisper result: '{}'", text), rms, db_level);
            
            if !text.is_empty() && text.len() > 1 {
                let estimated_confidence = estimate_python_style_confidence(text);
                
                // Clean up the text - remove brackets and convert to proper format
                let cleaned_text = clean_whisper_output(text);
                log_transcription_debug(&format!("[MAIN] Cleaned text: '{}'", cleaned_text), rms, db_level);
                
                let is_quality_ok = is_python_style_quality_ok(&cleaned_text, estimated_confidence);
                
                if !is_quality_ok {
                    log_transcription_debug(&format!("[MAIN FILTERED] {} (conf: {:.3})", cleaned_text, estimated_confidence), rms, db_level);
                    return Ok("".to_string());
                }
                
                // println!("🎙️ LOOPBACK: {} (conf: {:.3})", cleaned_text, estimated_confidence); // Commented out: Audio loopback is working, reducing console noise for debugging focus
                log_transcription_debug(&format!("[MAIN SUCCESS] {} (conf: {:.3})", cleaned_text, estimated_confidence), rms, db_level);
                
                // Emit transcription event to frontend
                let _emit_result = app_handle.emit("loopback-transcription", serde_json::json!({
                    "text": cleaned_text,
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                    "source": "loopback",
                    "confidence": estimated_confidence,
                    "audioLevel": db_level
                }));
                
                return Ok(cleaned_text.to_string());
            }
            Ok("".to_string())
        },
        Err(e) => {
            log_transcription_debug(&format!("[MAIN ERROR] Transcription failed: {}", e), rms, db_level);
            Err(format!("Transcription failed: {}", e))
        }
    }
}

// Audio processing functions - matching sandbox implementation
pub fn process_audio_chunk(
    audio_data: &[u8],
    bits_per_sample: u16,
    channels: u16,
    input_sample_rate: u32,
    output_sample_rate: u32
) -> Vec<f32> {
    if audio_data.is_empty() || channels == 0 || (bits_per_sample != 16 && bits_per_sample != 32) {
        // println!("[CHUNK] Invalid input: empty={}, channels={}, bits={}", 
        //          audio_data.is_empty(), channels, bits_per_sample);
        // Commented out: Audio loopback is working, reducing console noise for debugging focus
        return Vec::new();
    }
    
    // println!("[CHUNK] Processing: {} bytes, {}bit, {}ch, {}Hz -> {}Hz",
    //          audio_data.len(), bits_per_sample, channels, input_sample_rate, output_sample_rate);
    // Commented out: Audio loopback is working, reducing console noise for debugging focus
    
    // Step 1: Convert to i16 samples - MATCHING PYTHON EXACTLY
    let mut i16_samples = Vec::new();
    
    match bits_per_sample {
        32 => {
            if audio_data.len() % 4 != 0 { return Vec::new(); }
            for chunk in audio_data.chunks_exact(4) {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if sample.is_finite() && sample.abs() <= 2.0 {
                    let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    i16_samples.push(sample_i16);
                } else {
                    i16_samples.push(0);
                }
            }
        },
        16 => {
            if audio_data.len() % 2 != 0 { return Vec::new(); }
            for chunk in audio_data.chunks_exact(2) {
                let sample_i16 = i16::from_le_bytes([chunk[0], chunk[1]]);
                i16_samples.push(sample_i16);
            }
        },
        _ => return Vec::new()
    }
    
    // Step 2: EXACT Python stereo handling logic
    let mut audio_mono = if channels == 2 {
        // Reshape into stereo pairs
        let stereo_pairs: Vec<[i16; 2]> = i16_samples
            .chunks_exact(2)
            .map(|chunk| [chunk[0], chunk[1]])
            .collect();
        
        if stereo_pairs.is_empty() {
            Vec::new()
        } else {
            let left_channel: Vec<i16> = stereo_pairs.iter().map(|pair| pair[0]).collect();
            let right_channel: Vec<i16> = stereo_pairs.iter().map(|pair| pair[1]).collect();
            
            // Calculate stereo difference - Python: np.mean(np.abs(left - right))
            let stereo_diff: f32 = left_channel.iter()
                .zip(right_channel.iter())
                .map(|(&l, &r)| ((l as f32) - (r as f32)).abs())
                .sum::<f32>() / left_channel.len() as f32;
            
            if stereo_diff > 200.0 {
                // True stereo - Python: np.mean(channel**2)
                let left_rms: f32 = left_channel.iter()
                    .map(|&s| (s as f32) * (s as f32))
                    .sum::<f32>() / left_channel.len() as f32;
                
                let right_rms: f32 = right_channel.iter()
                    .map(|&s| (s as f32) * (s as f32))
                    .sum::<f32>() / right_channel.len() as f32;
                
                if left_rms > right_rms { left_channel } else { right_channel }
            } else {
                // Mono in stereo format - use left channel
                left_channel
            }
        }
    } else {
        i16_samples
    };
    
    // Step 3: DC offset removal - EXACTLY matching Python
    if !audio_mono.is_empty() {
        let dc_offset: f32 = audio_mono.iter()
            .map(|&s| s as f32)
            .sum::<f32>() / audio_mono.len() as f32;
        
        // Python checks if abs(dc_offset) > 100
        if dc_offset.abs() > 100.0 {
            let dc_offset_i16 = dc_offset as i16;
            for sample in &mut audio_mono {
                *sample = sample.saturating_sub(dc_offset_i16);
            }
        }
    }
    
    // Step 4: Resampling - MATCHING Python decimation logic
    if input_sample_rate != output_sample_rate && !audio_mono.is_empty() {
        audio_mono = match (input_sample_rate, output_sample_rate) {
            (48000, 16000) => {
                // Python: audio_mono[::3] - simple 3:1 decimation
                audio_mono.into_iter().step_by(3).collect()
            },
            (44100, 16000) => {
                // Python approximate decimation
                let factor = input_sample_rate as f32 / output_sample_rate as f32;
                let mut resampled = Vec::new();
                let mut i = 0.0;
                while i < audio_mono.len() as f32 {
                    let index = i as usize;
                    if index < audio_mono.len() {
                        resampled.push(audio_mono[index]);
                    }
                    i += factor;
                }
                resampled
            },
            _ => {
                // For other rates, keep as-is (Python would use scipy.signal.resample)
                // We'll rely on Whisper to handle the resampling
                audio_mono
            }
        };
    }
    
    // Step 5: Convert to f32 normalized [-1.0, 1.0]
    audio_mono.iter().map(|&sample| sample as f32 / 32768.0).collect()
}


pub fn calculate_audio_level(audio_data: &[f32]) -> f32 {
    if audio_data.is_empty() {
        return -60.0;
    }
    
    let rms = (audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
    
    if rms > 0.0 {
        20.0 * rms.log10().max(-60.0)
    } else {
        -60.0
    }
}

// Debug logging function to file
fn log_transcription_debug(text: &str, rms: f32, db_level: f32) {
    let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
    let log_entry = format!("[{}] RMS: {:.6} ({:.1}dB) | {}\n", timestamp, rms, db_level, text);
    
    // Get absolute path for log file
    let log_path = if let Ok(current_dir) = std::env::current_dir() {
        current_dir.join("transcription_debug.txt")
    } else {
        std::path::PathBuf::from("C:\\_dev\\enteract\\transcription_debug.txt")
    };
    
    // Also log to console for debugging
    // println!("[DEBUG] {}", log_entry.trim()); // Commented out: Audio loopback is working, reducing console noise for debugging focus
    
    let file_exists = log_path.exists();
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        if !file_exists {
            // println!("[DEBUG] Creating debug log at: {:?}", log_path); // Commented out: Audio loopback is working, reducing console noise for debugging focus
            let header = format!("=== Transcription Debug Log Started: {} ===\n\n", 
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = file.write_all(header.as_bytes());
        }
        let _ = file.write_all(log_entry.as_bytes());
        let _ = file.flush();
    } else {
        // println!("[DEBUG] Failed to open log file at: {:?}", log_path); // Commented out: Audio loopback is working, reducing console noise for debugging focus
    }
}

// Python-style quality filtering (more lenient)
fn is_python_style_quality_ok(text: &str, confidence: f32) -> bool {
    if text.len() < 2 {
        return false;
    }
    
    // Use Python script's confidence threshold (0.35 instead of 0.5)
    if confidence < 0.35 {
        return false;
    }
    
    // Simple repetition check (matching Python script)
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() > 4 {
        let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
        let unique_ratio = unique_words.len() as f32 / words.len() as f32;
        if unique_ratio < 0.3 {
            return false;
        }
    }
    
    // Allow more artifacts through (like Python script) - only block obvious ones
    let text_lower = text.to_lowercase();
    if text_lower.contains("thanks for watching") || 
       text_lower.contains("subscribe") ||
       text_lower.contains("like and subscribe") {
        return false;
    }
    
    true
}

// Python-style confidence estimation (simpler and more lenient)
fn estimate_python_style_confidence(text: &str) -> f32 {
    if text.len() < 2 {
        return 0.2;
    }
    
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.2;
    }
    
    // Base confidence is higher than our previous implementation
    let mut confidence = 0.8;
    
    // Check for obvious artifacts but don't penalize as heavily
    let text_lower = text.to_lowercase();
    if text_lower.contains("(") && text_lower.contains(")") {
        confidence *= 0.7; // Less penalty than before
    }
    
    // Simple repetition check
    if words.len() > 3 {
        let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
        let uniqueness_ratio = unique_words.len() as f32 / words.len() as f32;
        confidence *= uniqueness_ratio;
    }
    
    confidence.clamp(0.1, 0.95)
}

// Clean up Whisper output - convert bracketed artifacts to readable text
fn clean_whisper_output(text: &str) -> String {
    let text = text.trim();
    
    // Convert common Whisper bracket artifacts to readable text
    let cleaned = text
        .replace("[BLANK_AUDIO]", "")
        .replace("[MUSIC PLAYING]", "(music)")
        .replace("[MUSIC]", "(music)")
        .replace("[music]", "(music)")
        .replace("[BEEPING]", "(beeping)")
        .replace("[BANG]", "(sound)")
        .replace("[coughing]", "(coughing)")
        .replace("[electronic beeping]", "(electronic beeping)")
        .replace("[upbeat music]", "(upbeat music)")
        .replace("[funky music]", "(funky music)")
        .replace("[crashing]", "(crashing)")
        .replace("[swoosh]", "(swoosh)")
        // Remove multiple spaces and trim
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");
    
    // If the result is just parenthetical content, keep it
    // If it has actual words, prioritize those
    if cleaned.is_empty() {
        text.to_string()
    } else {
        cleaned
    }
}

