// Audio diagnostics commands for troubleshooting
use crate::audio_loopback::audio_diagnostics::{
    check_system_audio_capability, SystemAudioCapability, log_audio_event, flush_logs
};
use base64::Engine;
use crate::audio_loopback::device_enumerator::AudioDeviceEnumerator;
use anyhow::Result;
use serde_json::json;

#[tauri::command]
pub async fn diagnose_audio_system() -> Result<AudioSystemDiagnosis, String> {
    log_audio_event("DIAGNOSTICS", "Starting system audio diagnosis", None);
    
    let capability = check_system_audio_capability();
    
    // Check available devices
    let device_result = match AudioDeviceEnumerator::new() {
        Ok(enumerator) => match enumerator.enumerate_loopback_devices() {
            Ok(devices) => {
                log_audio_event("DIAGNOSTICS", &format!("Found {} audio devices", devices.len()), 
                    Some(json!({"device_count": devices.len()})));
                Ok(devices)
            }
            Err(e) => {
                log_audio_event("DIAGNOSTICS", &format!("Failed to enumerate devices: {}", e), None);
                Err(format!("Device enumeration failed: {}", e))
            }
        }
        Err(e) => {
            log_audio_event("DIAGNOSTICS", &format!("Failed to create enumerator: {}", e), None);
            Err(format!("Failed to create audio enumerator: {}", e))
        }
    };
    
    // Check Whisper model availability
    let whisper_status = check_whisper_models().await;
    
    flush_logs();
    
    // Create local clones to avoid move issues
    let device_error = if device_result.is_err() { 
        Some(device_result.as_ref().unwrap_err().clone())
    } else { 
        None 
    };
    
    let devices = device_result.clone().unwrap_or_default();
    
    Ok(AudioSystemDiagnosis {
        platform_capability: capability.clone(),
        available_devices: devices,
        device_enumeration_error: device_error,
        whisper_models: whisper_status,
        recommendations: generate_recommendations(&capability, &device_result),
    })
}

#[tauri::command]
pub async fn test_whisper_transcription(test_phrase: String) -> Result<WhisperTestResult, String> {
    log_audio_event("TEST", "Starting Whisper test", Some(json!({"test_phrase": test_phrase})));
    
    // Create a simple test audio buffer (sine wave at 440Hz for 2 seconds)
    let sample_rate = 16000;
    let duration = 2.0;
    let frequency = 440.0;
    let samples_count = (sample_rate as f32 * duration) as usize;
    
    let mut test_audio: Vec<f32> = Vec::with_capacity(samples_count);
    for i in 0..samples_count {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.1; // Low amplitude
        test_audio.push(sample);
    }
    
    // Convert to PCM16 for Whisper
    let pcm16_samples: Vec<i16> = test_audio.iter()
        .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect();
    
    let pcm16_bytes: Vec<u8> = pcm16_samples.iter()
        .flat_map(|&sample| sample.to_le_bytes())
        .collect();
    
    let audio_base64 = base64::prelude::BASE64_STANDARD.encode(&pcm16_bytes);
    
    let config = crate::speech::WhisperModelConfig {
        modelSize: "small".to_string(),
        language: Some("en".to_string()),
        enableVad: false,
        silenceThreshold: 0.01,
        maxSegmentLength: 30,
    };
    
    let start_time = std::time::Instant::now();
    
    match crate::speech::transcribe_audio_base64(audio_base64, config).await {
        Ok(result) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            log_audio_event("TEST", "Whisper test completed", Some(json!({
                "duration_ms": duration_ms,
                "result": result.text,
                "confidence": result.confidence
            })));
            
            Ok(WhisperTestResult {
                success: true,
                transcription: result.text,
                confidence: result.confidence,
                processing_time_ms: duration_ms,
                error: None,
            })
        }
        Err(e) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            log_audio_event("TEST", &format!("Whisper test failed: {}", e), Some(json!({
                "duration_ms": duration_ms,
                "error": e.to_string()
            })));
            
            Ok(WhisperTestResult {
                success: false,
                transcription: String::new(),
                confidence: 0.0,
                processing_time_ms: duration_ms,
                error: Some(e),
            })
        }
    }
}

#[tauri::command]
pub async fn get_audio_debug_log() -> Result<String, String> {
    use std::fs;
    let log_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("audio_debug.log");
    
    match fs::read_to_string(&log_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Failed to read debug log: {}", e))
    }
}

async fn check_whisper_models() -> Vec<WhisperModelStatus> {
    let models = vec!["tiny", "base", "small", "medium", "large"];
    let mut statuses = Vec::new();
    
    for model in models {
        let available = match crate::speech::check_whisper_model_availability(model.to_string()).await {
            Ok(available) => available,
            Err(_) => false,
        };
        
        statuses.push(WhisperModelStatus {
            model_name: model.to_string(),
            available,
            size_estimate: match model {
                "tiny" => "39 MB",
                "base" => "74 MB", 
                "small" => "244 MB",
                "medium" => "769 MB",
                "large" => "1550 MB",
                _ => "Unknown",
            }.to_string(),
        });
    }
    
    statuses
}

fn generate_recommendations(
    capability: &SystemAudioCapability, 
    device_result: &Result<Vec<crate::audio_loopback::types::AudioLoopbackDevice>, String>
) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    if !capability.has_native_loopback {
        recommendations.push(capability.recommended_setup.clone());
    }
    
    match device_result {
        Ok(devices) => {
            if devices.is_empty() {
                recommendations.push("No audio devices found. Check system audio settings.".to_string());
            } else if devices.len() == 1 {
                recommendations.push("Only one audio device available. For best results, ensure it supports the intended audio source.".to_string());
            }
        }
        Err(_) => {
            recommendations.push("Cannot enumerate audio devices. Check audio driver installation.".to_string());
        }
    }
    
    recommendations.push("Ensure Whisper models are downloaded for optimal transcription performance.".to_string());
    recommendations.push("Test transcription with known audio to verify the pipeline is working.".to_string());
    
    recommendations
}

#[derive(Debug, serde::Serialize)]
pub struct AudioSystemDiagnosis {
    pub platform_capability: SystemAudioCapability,
    pub available_devices: Vec<crate::audio_loopback::types::AudioLoopbackDevice>,
    pub device_enumeration_error: Option<String>,
    pub whisper_models: Vec<WhisperModelStatus>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct WhisperModelStatus {
    pub model_name: String,
    pub available: bool,
    pub size_estimate: String,
}

#[derive(Debug, serde::Serialize)]
pub struct WhisperTestResult {
    pub success: bool,
    pub transcription: String,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub error: Option<String>,
}