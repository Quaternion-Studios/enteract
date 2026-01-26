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

// macOS implementation using CPAL (Phase 2: en-03o)
#[cfg(target_os = "macos")]
use std::sync::Mutex as StdMutex;

#[cfg(target_os = "macos")]
lazy_static::lazy_static! {
    static ref MACOS_CAPTURE_ENGINE: StdMutex<Option<macos::CPALCaptureEngine>> = StdMutex::new(None);
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn enumerate_loopback_devices() -> Result<Vec<AudioLoopbackDevice>, String> {
    let enumerator = macos::CPALDeviceEnumerator::new();
    enumerator
        .enumerate_loopback_devices()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn auto_select_best_device() -> Result<Option<AudioLoopbackDevice>, String> {
    let enumerator = macos::CPALDeviceEnumerator::new();
    enumerator
        .auto_select_best_device()
        .map_err(|e| format!("Failed to auto-select device: {}", e))
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn test_audio_device(device_id: String) -> Result<bool, String> {
    let enumerator = macos::CPALDeviceEnumerator::new();
    Ok(enumerator.test_device_capability(&device_id))
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn start_audio_loopback_capture(device_id: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    // Stop existing capture if any (take ownership, release lock before await)
    let existing = {
        if let Ok(mut engine) = MACOS_CAPTURE_ENGINE.lock() {
            engine.take()
        } else {
            None
        }
    };

    if let Some(mut existing_engine) = existing {
        let _ = existing_engine.stop().await;
    }

    // Create and start new capture engine
    // Determine device type by enumerating devices
    let enumerator = macos::CPALDeviceEnumerator::new();
    let devices = enumerator.enumerate_loopback_devices()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?;

    let device_type = devices.iter()
        .find(|d| d.id == device_id)
        .map(|d| d.device_type.clone())
        .unwrap_or(types::DeviceType::Capture); // Default to Capture if not found

    let mut new_engine = macos::CPALCaptureEngine::new(device_id.clone(), device_type);
    new_engine.start(app_handle).await?;

    // Store engine
    if let Ok(mut engine) = MACOS_CAPTURE_ENGINE.lock() {
        *engine = Some(new_engine);
    }

    Ok(format!("Started audio loopback capture on device: {}", device_id))
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn stop_audio_loopback_capture() -> Result<(), String> {
    // Take ownership, release lock before await
    let existing = {
        if let Ok(mut engine) = MACOS_CAPTURE_ENGINE.lock() {
            engine.take()
        } else {
            None
        }
    };

    if let Some(mut existing_engine) = existing {
        existing_engine.stop().await?;
    }

    Ok(())
}
