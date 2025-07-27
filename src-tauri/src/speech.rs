use std::sync::{Arc, Mutex};

// Whisper-rs imports for transcription
use std::path::PathBuf;
use std::fs;
use base64::{Engine as _, engine::general_purpose};
use tempfile::NamedTempFile;
use anyhow::Result;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuConfig {
    pub use_gpu: bool,
    pub gpu_device: Option<i32>,
    pub gpu_type: GpuType,
    pub gpu_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GpuType {
    Cuda,
    Metal,
    OpenCL,
    Cpu,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub chunk_size: usize,
    pub silence_threshold: f32,
    pub silence_duration: f32,
    pub max_recording_duration: f32,
    pub gpu_config: Option<GpuConfig>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            chunk_size: 1024,
            silence_threshold: 0.01,
            silence_duration: 2.0,
            max_recording_duration: 30.0,
            gpu_config: None,
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
    pub static ref GPU_CONFIG: Arc<Mutex<Option<GpuConfig>>> = Arc::new(Mutex::new(None));
    static ref MODEL_CACHE_DIR: PathBuf = {
        let mut cache_dir = std::env::temp_dir();
        cache_dir.push("enteract");
        cache_dir.push("whisper_models");
        cache_dir
    };
}

// GPU detection functions
async fn detect_gpu_capabilities() -> GpuConfig {
    // Check for CUDA availability on Windows/Linux
    #[cfg(not(target_os = "macos"))]
    {
        // First try to get detailed GPU info with nvidia-smi
        if let Ok(nvidia_output) = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=gpu_name")
            .arg("--format=csv,noheader,nounits")
            .output()
        {
            if nvidia_output.status.success() {
                let gpu_name = String::from_utf8_lossy(&nvidia_output.stdout).trim().to_string();
                if !gpu_name.is_empty() {
                    return GpuConfig {
                        use_gpu: true,
                        gpu_device: Some(0),
                        gpu_type: GpuType::Cuda,
                        gpu_name: Some(gpu_name),
                    };
                }
            }
        }
        
        // Fallback to basic nvidia-smi check
        if let Ok(nvidia_output) = std::process::Command::new("nvidia-smi")
            .output()
        {
            if nvidia_output.status.success() {
                let output_str = String::from_utf8_lossy(&nvidia_output.stdout);
                if output_str.contains("NVIDIA") || output_str.contains("Driver Version") {
                    return GpuConfig {
                        use_gpu: true,
                        gpu_device: Some(0),
                        gpu_type: GpuType::Cuda,
                        gpu_name: Some("NVIDIA GPU".to_string()),
                    };
                }
            }
        }
    }
    
    // Check for Metal on macOS
    #[cfg(target_os = "macos")]
    {
        // macOS with Apple Silicon or discrete GPU supports Metal
        return GpuConfig {
            use_gpu: true,
            gpu_device: Some(0),
            gpu_type: GpuType::Metal,
            gpu_name: Some("Metal GPU".to_string()),
        };
    }
    
    // Fallback to CPU
    GpuConfig {
        use_gpu: false,
        gpu_device: None,
        gpu_type: GpuType::Cpu,
        gpu_name: None,
    }
}

// Whisper-rs commands for frontend
#[tauri::command]
pub async fn initialize_whisper_model(config: WhisperModelConfig) -> Result<String, String> {
    initialize_whisper_model_with_gpu(config, true).await
}

#[tauri::command]
pub async fn initialize_whisper_model_with_gpu(config: WhisperModelConfig, use_gpu: bool) -> Result<String, String> {
    println!("🔍 [WHISPER_INIT] Called with model: '{}', use_gpu: {}", config.modelSize, use_gpu);
    
    let gpu_config = if use_gpu {
        detect_gpu_capabilities().await
    } else {
        GpuConfig {
            use_gpu: false,
            gpu_device: None,
            gpu_type: GpuType::Cpu,
            gpu_name: None,
        }
    };
    
    println!("🔍 [WHISPER_INIT] GPU config: {:?}", gpu_config);
    
    let model_path = get_or_download_model(&config.modelSize).await?;
    println!("🔍 [WHISPER_INIT] Model path: {:?}", model_path);
    
    let mut params = WhisperContextParameters::default();
    
    if gpu_config.use_gpu {
        params.use_gpu(true);
        if let Some(device) = gpu_config.gpu_device {
            params.gpu_device(device);
        }
        println!("🔍 [WHISPER_INIT] GPU parameters set");
    }
    
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().ok_or("Invalid model path")?,
        params
    ).map_err(|e| format!("Failed to initialize Whisper context: {}", e))?;
    
    let mut whisper_ctx = WHISPER_CONTEXT.lock().unwrap();
    *whisper_ctx = Some(ctx);
    
    // Store GPU config
    let mut gpu_cfg = GPU_CONFIG.lock().unwrap();
    *gpu_cfg = Some(gpu_config.clone());
    
    let acceleration = if gpu_config.use_gpu {
        format!("{:?} ({})", gpu_config.gpu_type, gpu_config.gpu_name.unwrap_or_default())
    } else {
        "CPU".to_string()
    };
    
    let result_msg = format!("Whisper model '{}' initialized with {} acceleration", 
                           config.modelSize, acceleration);
    println!("✅ [WHISPER_INIT] {}", result_msg);
    
    Ok(result_msg)
}

#[tauri::command]
pub async fn transcribe_audio_base64(audioData: String, config: WhisperModelConfig) -> Result<TranscriptionResult, String> {
    println!("🔍 [TRANSCRIBE] Called with model: '{}', language: {:?}", config.modelSize, config.language);
    
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
    println!("🔍 [TRANSCRIBE_FILE] Called with model: '{}', file: {}", config.modelSize, file_path);
    
    // Ensure model is initialized
    let needs_init = {
        let whisper_ctx = WHISPER_CONTEXT.lock().unwrap();
        whisper_ctx.is_none()
    };
    
    if needs_init {
        println!("🔍 [TRANSCRIBE_FILE] Model not initialized, initializing with model: '{}'", config.modelSize);
        initialize_whisper_model(config.clone()).await?;
    } else {
        println!("🔍 [TRANSCRIBE_FILE] Model already initialized, using existing model");
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

#[tauri::command]
pub async fn get_available_models_for_device(device_type: String) -> Result<Vec<String>, String> {
    let gpu_config = detect_gpu_capabilities().await;
    
    match device_type.as_str() {
        "microphone" => {
            // Microphone should use fast models for real-time performance
            Ok(vec![
                "tiny".to_string(),
                "base".to_string(),
                "small".to_string(),
            ])
        },
        "loopback" => {
            // System audio loopback can use larger models if GPU is available
            let mut models = vec![
                "tiny".to_string(),
                "base".to_string(),
                "small".to_string(),
            ];
            
            // Only add medium if GPU is available and capable
            if is_gpu_capable_for_medium(&gpu_config) {
                models.push("medium".to_string());
            }
            
            Ok(models)
        },
        _ => Err("Invalid device type. Use 'microphone' or 'loopback'".to_string())
    }
}

// Check if GPU is capable of running medium model efficiently
fn is_gpu_capable_for_medium(gpu_config: &GpuConfig) -> bool {
    if !gpu_config.use_gpu {
        return false;
    }
    
    match gpu_config.gpu_type {
        GpuType::Cuda => {
            // For NVIDIA GPUs, if CUDA is available and working, assume it's capable
            // Modern NVIDIA drivers (like 566.41) with CUDA 12.7 should handle medium models
            if let Some(ref gpu_name) = gpu_config.gpu_name {
                let name_lower = gpu_name.to_lowercase();
                // Any modern NVIDIA GPU (RTX, GTX 10 series+, or professional cards)
                name_lower.contains("rtx") || 
                name_lower.contains("gtx") ||
                name_lower.contains("quadro") ||
                name_lower.contains("tesla") ||
                name_lower.contains("nvidia") ||
                name_lower.contains("geforce") ||
                // If nvidia-smi works, the GPU is likely capable
                true
            } else {
                // If CUDA is detected but no specific GPU name, still allow it
                // This covers cases where nvidia-smi works but parsing fails
                true
            }
        },
        GpuType::Metal => {
            // Apple Silicon M1/M2/M3 can handle medium model well
            true
        },
        _ => false
    }
}

#[tauri::command]
pub async fn get_gpu_info() -> Result<GpuConfig, String> {
    let gpu_config = detect_gpu_capabilities().await;
    Ok(gpu_config)
}

#[tauri::command]
pub async fn get_current_gpu_config() -> Result<Option<GpuConfig>, String> {
    let gpu_cfg = GPU_CONFIG.lock().unwrap();
    Ok(gpu_cfg.clone())
}

#[tauri::command]
pub async fn debug_gpu_capability_check() -> Result<serde_json::Value, String> {
    let gpu_config = detect_gpu_capabilities().await;
    let is_capable = is_gpu_capable_for_medium(&gpu_config);
    
    Ok(serde_json::json!({
        "gpu_config": gpu_config,
        "is_capable_for_medium": is_capable,
        "gpu_acceleration_enabled": gpu_config.use_gpu,
        "available_models_loopback": if is_capable {
            vec!["tiny", "base", "small", "medium"]
        } else {
            vec!["tiny", "base", "small"]
        }
    }))
}

#[tauri::command]
pub async fn reload_whisper_model_for_loopback(model_size: String) -> Result<String, String> {
    println!("🔄 [MODEL_RELOAD] Reloading Whisper model for loopback: '{}'", model_size);
    
    let config = WhisperModelConfig {
        modelSize: model_size.clone(),
        language: Some("en".to_string()),
        enableVad: false,
        silenceThreshold: 0.01,
        maxSegmentLength: 30,
    };
    
    // Force reload by initializing with GPU acceleration
    let result = initialize_whisper_model_with_gpu(config, true).await?;
    println!("✅ [MODEL_RELOAD] Model '{}' reloaded for loopback", model_size);
    
    Ok(result)
}

// Helper functions for Whisper
async fn get_or_download_model(model_size: &str) -> Result<PathBuf, String> {
    let model_path = get_model_path(model_size);
    
    println!("🔍 [MODEL_CHECK] Checking model file: {:?}", model_path);
    
    if !model_path.exists() {
        println!("🔍 [MODEL_CHECK] Model file does not exist, downloading...");
        download_model(model_size).await?;
    } else if !is_valid_model_file(&model_path) {
        println!("🔍 [MODEL_CHECK] Model file exists but is invalid, re-downloading...");
        fs::remove_file(&model_path)
            .map_err(|e| format!("Failed to remove invalid model: {}", e))?;
        download_model(model_size).await?;
    } else {
        println!("✅ [MODEL_CHECK] Model file exists and is valid: {:?}", model_path);
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
    println!("📥 [DOWNLOAD] Starting download for model: {}", model_size);
    
    fs::create_dir_all(&*MODEL_CACHE_DIR)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;
    
    let model_url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_size
    );
    
    let model_path = get_model_path(model_size);
    
    println!("📥 [DOWNLOAD] Downloading Whisper model '{}' from: {}", model_size, model_url);
    println!("📥 [DOWNLOAD] Target path: {:?}", model_path);
    
    let response = reqwest::get(&model_url).await
        .map_err(|e| format!("Failed to download model: {}", e))?;
    
    if !response.status().is_success() {
        let error_msg = format!("Failed to download model: HTTP {}", response.status());
        println!("❌ [DOWNLOAD] {}", error_msg);
        return Err(error_msg);
    }
    
    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read model data: {}", e))?;
    
    println!("📥 [DOWNLOAD] Downloaded {} bytes", bytes.len());
    
    fs::write(&model_path, bytes)
        .map_err(|e| format!("Failed to save model: {}", e))?;
    
    println!("✅ [DOWNLOAD] Successfully downloaded Whisper model '{}' to: {:?}", model_size, model_path);
    
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

