// src-tauri/src/audio_loopback.rs
// Platform-abstracted audio loopback system
//
// Phase 1 (en-pfn): Platform abstraction layer with trait-based architecture
//
// Architecture:
// - traits.rs: Platform-agnostic interfaces (AudioDeviceEnumerator, AudioCaptureEngine, AudioProcessor)
// - types.rs: Shared data structures (AudioLoopbackDevice, CaptureState, etc.)
// - windows/: WASAPI implementation for Windows
// - macos/: CPAL/CoreAudio implementation for macOS
// - shared/: Cross-platform utilities (audio processing, quality filtering, settings)

// Core modules (always available)
pub mod traits;
pub mod types;

// Platform-specific implementations
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// Shared utilities (cross-platform)
pub mod shared;

// Platform dispatch: Re-export the appropriate platform implementation
#[cfg(target_os = "windows")]
pub use windows::{
    enumerate_loopback_devices,
    auto_select_best_device,
    test_audio_device,
    start_audio_loopback_capture,
    stop_audio_loopback_capture,
};

// Shared functions (cross-platform)
pub use shared::{
    process_audio_for_transcription,
    save_audio_settings,
    load_audio_settings,
    save_general_settings,
    load_general_settings,
};

// Re-export common types
pub use types::{
    CAPTURE_STATE,
    CaptureState,
    AudioLoopbackDevice,
    DeviceType,
    LoopbackMethod,
    AudioDeviceSettings,
};

// macOS implementation (stub commands for now - will be moved to macos/ module)
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn enumerate_loopback_devices() -> Result<Vec<AudioLoopbackDevice>, String> {
    Err("Audio loopback not yet implemented for macOS. Phase 1 in progress.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn auto_select_best_device() -> Result<Option<AudioLoopbackDevice>, String> {
    Err("Audio loopback not yet implemented for macOS. Phase 1 in progress.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn test_audio_device(_device_id: String) -> Result<bool, String> {
    Err("Audio loopback not yet implemented for macOS. Phase 1 in progress.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn start_audio_loopback_capture(_device_id: String, _app_handle: tauri::AppHandle) -> Result<String, String> {
    Err("Audio loopback not yet implemented for macOS. Phase 1 in progress.".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn stop_audio_loopback_capture() -> Result<(), String> {
    Err("Audio loopback not yet implemented for macOS. Phase 1 in progress.".to_string())
}
