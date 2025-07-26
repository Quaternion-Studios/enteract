use std::sync::{Arc, Mutex};

// Whisper-rs imports for transcription
use std::path::PathBuf;
use std::fs;
use base64::{Engine as _, engine::general_purpose};
use tempfile::NamedTempFile;
use anyhow::Result;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub chunk_size: usize,
    pub silence_threshold: f32,
    pub silence_duration: f32,
    pub max_recording_duration: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            chunk_size: 1024,
            silence_threshold: 0.01,
            silence_duration: 2.0,
            max_recording_duration: 30.0,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpeechTranscription {
    pub text: String,
    pub confidence: f32,
    pub duration: f32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SpeechState {
    pub is_listening: bool,
    pub is_recording: bool,
    pub last_transcription: Option<SpeechTranscription>,
}

impl Default for SpeechState {
    fn default() -> Self {
        Self {
            is_listening: false,
            is_recording: false,
            last_transcription: None,
        }
    }
}

// Whisper-rs structures for transcription
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WhisperModelConfig {
    pub modelSize: String,
    pub language: Option<String>,
    pub enableVad: bool,
    pub silenceThreshold: f32,
    pub maxSegmentLength: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub start_time: f32,
    pub end_time: f32,
    pub language: Option<String>,
}

// Global whisper context
lazy_static::lazy_static! {
    pub static ref WHISPER_CONTEXT: Arc<Mutex<Option<WhisperContext>>> = Arc::new(Mutex::new(None));
    static ref MODEL_CACHE_DIR: PathBuf = {
        let mut cache_dir = std::env::temp_dir();
        cache_dir.push("enteract");
        cache_dir.push("whisper_models");
        cache_dir
    };
}

// Whisper-rs commands for frontend
#[tauri::command]
pub async fn initialize_whisper_model(config: WhisperModelConfig) -> Result<String, String> {
    let model_path = get_or_download_model(&config.modelSize).await?;
    
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().ok_or("Invalid model path")?,
        WhisperContextParameters::default()
    ).map_err(|e| format!("Failed to initialize Whisper context: {}", e))?;
    
    let mut whisper_ctx = WHISPER_CONTEXT.lock().unwrap();
    *whisper_ctx = Some(ctx);
    
    Ok(format!("Whisper model '{}' initialized successfully", config.modelSize))
}

#[tauri::command]
pub async fn transcribe_audio_base64(audioData: String, config: WhisperModelConfig) -> Result<TranscriptionResult, String> {
    // Decode base64 audio data
    let audio_bytes = general_purpose::STANDARD
        .decode(&audioData)
        .map_err(|e| format!("Failed to decode base64 audio: {}", e))?;
    
    // Create temporary file for audio - using .pcm extension for raw PCM data
    let temp_file = NamedTempFile::with_suffix(".pcm")
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    fs::write(temp_file.path(), audio_bytes)
        .map_err(|e| format!("Failed to write audio to temp file: {}", e))?;
    
    transcribe_audio_file(temp_file.path().to_string_lossy().to_string(), config).await
}

#[tauri::command]
pub async fn transcribe_audio_file(file_path: String, config: WhisperModelConfig) -> Result<TranscriptionResult, String> {
    // Ensure model is initialized
    let needs_init = {
        let whisper_ctx = WHISPER_CONTEXT.lock().unwrap();
        whisper_ctx.is_none()
    };
    
    if needs_init {
        initialize_whisper_model(config.clone()).await?;
    }
    
    // Load and preprocess audio
    let audio_data = load_audio_file(&file_path)?;
    
    // Get Whisper context
    let whisper_ctx = WHISPER_CONTEXT.lock().unwrap();
    let ctx = whisper_ctx.as_ref().ok_or("Whisper context not initialized")?;
    
    // Set up transcription parameters - MATCHING PYTHON SCRIPT
    // Python uses: beam_size=1, best_of=1, temperature=0.0
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    
    // Python passes language=None for auto-detection
    if let Some(ref lang) = config.language {
        if lang != "auto" && !lang.is_empty() {
            params.set_language(Some(lang));
        } else {
            params.set_language(None);  // Auto-detect like Python
        }
    } else {
        params.set_language(None);  // Auto-detect like Python
    }
    
    // Match Python settings
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);      // Python: suppress_blank=True
    params.set_single_segment(false);     // Allow multiple segments
    params.set_no_context(true);          // Python: condition_on_previous_text=False
    params.set_temperature(0.0);          // Python: temperature=0.0
    params.set_no_timestamps(true);       // Python: without_timestamps=True
    
    // Run transcription
    let mut state = ctx.create_state().map_err(|e| format!("Failed to create state: {}", e))?;
    state.full(params, &audio_data)
        .map_err(|e| format!("Transcription failed: {}", e))?;
    
    // Extract results
    let num_segments = state.full_n_segments()
        .map_err(|e| format!("Failed to get segment count: {}", e))?;
    
    let mut full_text = String::new();
    let mut total_confidence = 0.0;
    let mut start_time: f32 = f32::MAX;
    let mut end_time: f32 = 0.0;
    
    for i in 0..num_segments {
        let segment_text = state.full_get_segment_text(i)
            .map_err(|e| format!("Failed to get segment text: {}", e))?;
        
        let segment_start = state.full_get_segment_t0(i)
            .map_err(|e| format!("Failed to get segment start time: {}", e))? as f32 / 100.0;
        
        let segment_end = state.full_get_segment_t1(i)
            .map_err(|e| format!("Failed to get segment end time: {}", e))? as f32 / 100.0;
        
        full_text.push_str(&segment_text);
        start_time = start_time.min(segment_start);
        end_time = end_time.max(segment_end);
        total_confidence += 1.0; // Whisper doesn't provide confidence scores directly
    }
    
    let avg_confidence = if num_segments > 0 { total_confidence / num_segments as f32 } else { 0.0 };
    
    Ok(TranscriptionResult {
        text: full_text.trim().to_string(),
        confidence: avg_confidence,
        start_time,
        end_time,
        language: config.language,
    })
}

#[tauri::command]
pub async fn check_whisper_model_availability(modelSize: String) -> Result<bool, String> {
    let model_path = get_model_path(&modelSize);
    Ok(model_path.exists())
}

#[tauri::command]
pub async fn download_whisper_model(modelSize: String) -> Result<String, String> {
    let model_path = get_model_path(&modelSize);
    if model_path.exists() {
        fs::remove_file(&model_path)
            .map_err(|e| format!("Failed to remove existing model: {}", e))?;
    }
    
    get_or_download_model(&modelSize).await?;
    Ok(format!("Model '{}' downloaded successfully", modelSize))
}

#[tauri::command]
pub async fn list_available_models() -> Result<Vec<String>, String> {
    Ok(vec![
        "tiny".to_string(),
        "base".to_string(),
        "small".to_string(),
        "medium".to_string(),
        "large".to_string(),
    ])
}

// Helper functions for Whisper
async fn get_or_download_model(model_size: &str) -> Result<PathBuf, String> {
    let model_path = get_model_path(model_size);
    
    if !model_path.exists() || !is_valid_model_file(&model_path) {
        if model_path.exists() {
            fs::remove_file(&model_path)
                .map_err(|e| format!("Failed to remove invalid model: {}", e))?;
        }
        download_model(model_size).await?;
    }
    
    Ok(model_path)
}

fn is_valid_model_file(path: &PathBuf) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        metadata.len() > 1_000_000 // 1MB minimum
    } else {
        false
    }
}

fn get_model_path(model_size: &str) -> PathBuf {
    let mut path = MODEL_CACHE_DIR.clone();
    path.push(format!("ggml-{}.bin", model_size));
    path
}

async fn download_model(model_size: &str) -> Result<(), String> {
    fs::create_dir_all(&*MODEL_CACHE_DIR)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;
    
    let model_url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_size
    );
    
    let model_path = get_model_path(model_size);
    
    println!("Downloading Whisper model '{}' from: {}", model_size, model_url);
    
    let response = reqwest::get(&model_url).await
        .map_err(|e| format!("Failed to download model: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to download model: HTTP {}", response.status()));
    }
    
    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read model data: {}", e))?;
    
    fs::write(&model_path, bytes)
        .map_err(|e| format!("Failed to save model: {}", e))?;
    
    println!("Successfully downloaded Whisper model '{}' to: {:?}", model_size, model_path);
    
    Ok(())
}

fn load_audio_file(file_path: &str) -> Result<Vec<f32>, String> {
    let audio_bytes = fs::read(file_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;
    
    println!("[WHISPER] Loading audio file: {} bytes from {}", audio_bytes.len(), file_path);
    
    let mut audio_f32 = Vec::new();
    for chunk in audio_bytes.chunks(2) {
        if chunk.len() == 2 {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0;
            audio_f32.push(sample);
        }
    }
    
    println!("[WHISPER] Converted to {} f32 samples", audio_f32.len());
    
    // Check if audio is silent
    let rms = (audio_f32.iter().map(|&x| x * x).sum::<f32>() / audio_f32.len() as f32).sqrt();
    println!("[WHISPER] Audio RMS: {:.6}", rms);
    
    Ok(audio_f32)
}
