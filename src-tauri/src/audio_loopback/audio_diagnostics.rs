// Enhanced audio diagnostics and debugging
use std::fs::{OpenOptions, File};
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use std::sync::Mutex;
use anyhow::Result;
use serde_json::json;

// Thread-safe logger for audio pipeline
lazy_static::lazy_static! {
    static ref AUDIO_LOGGER: Mutex<Option<BufWriter<File>>> = Mutex::new(None);
    static ref LOG_PATH: PathBuf = {
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("audio_debug.log");
        path
    };
}

pub fn init_audio_logger() -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&*LOG_PATH)?;
    
    let mut writer = BufWriter::new(file);
    let header = format!("=== Audio Debug Session Started: {} ===\n\n", 
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    writer.write_all(header.as_bytes())?;
    writer.flush()?;
    
    let mut logger = AUDIO_LOGGER.lock().unwrap();
    *logger = Some(writer);
    
    println!("🔍 Audio diagnostics initialized: {:?}", *LOG_PATH);
    Ok(())
}

pub fn log_audio_event(category: &str, message: &str, data: Option<serde_json::Value>) {
    let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
    let log_entry = if let Some(data) = data {
        format!("[{}] {}: {} | {}\n", timestamp, category, message, data)
    } else {
        format!("[{}] {}: {}\n", timestamp, category, message)
    };
    
    // Always log to console
    println!("{}", log_entry.trim());
    
    // Log to file if available
    if let Ok(mut logger_guard) = AUDIO_LOGGER.lock() {
        if let Some(ref mut writer) = logger_guard.as_mut() {
            let _ = writer.write_all(log_entry.as_bytes());
            let _ = writer.flush();
        }
    }
}

pub fn log_audio_buffer_analysis(buffer: &[f32], stage: &str) {
    if buffer.is_empty() {
        log_audio_event("BUFFER", &format!("{}: EMPTY", stage), None);
        return;
    }
    
    let rms = (buffer.iter().map(|&x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    let max_amplitude = buffer.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
    let min_val = buffer.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max_val = buffer.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let db_level = if rms > 0.0 { 20.0 * rms.log10() } else { -60.0 };
    
    let data = json!({
        "samples": buffer.len(),
        "rms": format!("{:.6}", rms),
        "db_level": format!("{:.1}", db_level),
        "max_amplitude": format!("{:.6}", max_amplitude),
        "range": format!("[{:.3}, {:.3}]", min_val, max_val),
        "duration_ms": (buffer.len() as f32 / 16000.0 * 1000.0) as u32
    });
    
    log_audio_event("BUFFER", stage, Some(data));
}

pub fn log_device_info(device_name: &str, device_type: &str, sample_rate: u32, channels: u16) {
    let data = json!({
        "name": device_name,
        "type": device_type,
        "sample_rate": sample_rate,
        "channels": channels
    });
    
    log_audio_event("DEVICE", "Selected", Some(data));
}

pub fn log_transcription_attempt(text: &str, confidence: f32, duration_ms: u64, success: bool) {
    let data = json!({
        "text": text,
        "confidence": confidence,
        "duration_ms": duration_ms,
        "success": success,
        "length": text.len()
    });
    
    let category = if success { "TRANSCRIPTION_SUCCESS" } else { "TRANSCRIPTION_FAILED" };
    log_audio_event(category, &format!("'{}'", text), Some(data));
}

pub fn log_whisper_model_info(model_size: &str, model_path: &str, exists: bool) {
    let data = json!({
        "model_size": model_size,
        "model_path": model_path,
        "exists": exists
    });
    
    log_audio_event("WHISPER", "Model Check", Some(data));
}

pub fn log_error(category: &str, error: &str, context: Option<serde_json::Value>) {
    let data = if let Some(ctx) = context {
        json!({
            "error": error,
            "context": ctx
        })
    } else {
        json!({
            "error": error
        })
    };
    
    log_audio_event(&format!("ERROR_{}", category), error, Some(data));
}

// Audio format validation
pub fn validate_audio_format(data: &[u8], expected_bits: u16, expected_channels: u16) -> Result<(), String> {
    if data.is_empty() {
        return Err("Empty audio data".to_string());
    }
    
    let bytes_per_sample = expected_bits / 8;
    let bytes_per_frame = bytes_per_sample * expected_channels;
    
    if data.len() % bytes_per_frame as usize != 0 {
        return Err(format!(
            "Audio data length {} is not aligned to frame size {} ({}bit {}ch)",
            data.len(), bytes_per_frame, expected_bits, expected_channels
        ));
    }
    
    let frame_count = data.len() / bytes_per_frame as usize;
    let duration_ms = (frame_count as f32 / 16000.0 * 1000.0) as u32;
    
    log_audio_event("VALIDATION", "Audio format OK", Some(json!({
        "bytes": data.len(),
        "frames": frame_count,
        "duration_ms": duration_ms,
        "format": format!("{}bit {}ch", expected_bits, expected_channels)
    })));
    
    Ok(())
}

// System audio capability check
pub fn check_system_audio_capability() -> SystemAudioCapability {
    #[cfg(target_os = "windows")]
    {
        // Windows has native WASAPI loopback support
        SystemAudioCapability {
            platform: "Windows".to_string(),
            has_native_loopback: true,
            recommended_setup: "Use default system audio device".to_string(),
            limitations: None,
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS requires virtual audio device for system audio
        SystemAudioCapability {
            platform: "macOS".to_string(),
            has_native_loopback: false,
            recommended_setup: "Install BlackHole or Loopback.app for system audio capture".to_string(),
            limitations: Some("Native macOS doesn't support system audio loopback. Only microphone input is available without additional software.".to_string()),
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        SystemAudioCapability {
            platform: "Linux/Other".to_string(),
            has_native_loopback: false,
            recommended_setup: "Use PulseAudio or ALSA loopback".to_string(),
            limitations: Some("System audio capture varies by Linux distribution".to_string()),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemAudioCapability {
    pub platform: String,
    pub has_native_loopback: bool,
    pub recommended_setup: String,
    pub limitations: Option<String>,
}

pub fn flush_logs() {
    if let Ok(mut logger_guard) = AUDIO_LOGGER.lock() {
        if let Some(ref mut writer) = logger_guard.as_mut() {
            let _ = writer.flush();
        }
    }
}