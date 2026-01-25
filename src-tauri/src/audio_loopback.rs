// src-tauri/src/audio_loopback.rs
// This file has been refactored into modules for better organization.
// All functionality is now split across multiple files in the audio_loopback/ directory.
//
// Key improvements made:
// 1. Reduced excessive debug logging that was causing spam
// 2. Enhanced transcription quality filtering to prevent "(crying)" artifacts
// 3. Split large file into manageable modules
// 4. Maintained all sophisticated features from the sandbox implementation
//
// TODO (Phase 1 - en-pfn): Platform abstraction layer
// Currently Windows-only. Phase 1 will create trait-based abstractions and macOS implementation.

#[cfg(target_os = "windows")]
pub mod types;
#[cfg(target_os = "windows")]
pub mod device_enumerator;
#[cfg(target_os = "windows")]
pub mod audio_processor;
#[cfg(target_os = "windows")]
pub mod capture_engine;
#[cfg(target_os = "windows")]
pub mod quality_filter;
#[cfg(target_os = "windows")]
pub mod settings;

// Re-export main types and functions (Windows only for now)
#[cfg(target_os = "windows")]
pub use types::{CAPTURE_STATE, CaptureState, AudioLoopbackDevice, DeviceType, LoopbackMethod, AudioDeviceSettings};
#[cfg(target_os = "windows")]
pub use device_enumerator::*;
#[cfg(target_os = "windows")]
pub use capture_engine::*;
#[cfg(target_os = "windows")]
pub use audio_processor::*;
#[cfg(target_os = "windows")]
pub use settings::*;

// macOS stubs - to be implemented in Phase 1
#[cfg(target_os = "macos")]
pub mod types {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AudioLoopbackDevice {
        pub id: String,
        pub name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AudioDeviceSettings {
        pub device_id: String,
    }
}

#[cfg(target_os = "macos")]
use types::{AudioLoopbackDevice, AudioDeviceSettings};

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn enumerate_loopback_devices() -> Result<Vec<AudioLoopbackDevice>, String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn auto_select_best_device() -> Result<Option<AudioLoopbackDevice>, String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn test_audio_device(_device_id: String) -> Result<bool, String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn save_audio_settings(_settings: AudioDeviceSettings) -> Result<(), String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn load_audio_settings() -> Result<Option<AudioDeviceSettings>, String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn save_general_settings(_settings: std::collections::HashMap<String, serde_json::Value>) -> Result<(), String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn load_general_settings() -> Result<Option<std::collections::HashMap<String, serde_json::Value>>, String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn start_audio_loopback_capture(_device_id: String, _app_handle: tauri::AppHandle) -> Result<String, String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn stop_audio_loopback_capture() -> Result<(), String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn process_audio_for_transcription(
    _audio_bytes: Vec<u8>,
    _sample_rate: u32,
    _app_handle: tauri::AppHandle,
) -> Result<String, String> {
    Err("Audio loopback not yet implemented for macOS. Coming in Phase 1.".to_string())
}