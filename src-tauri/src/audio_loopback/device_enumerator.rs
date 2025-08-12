// Platform-agnostic device enumeration
use crate::audio_loopback::types::*;
use crate::audio_loopback::platform::{get_audio_backend, AudioCaptureBackend};
use anyhow::Result;

// Re-export platform-specific backend for backward compatibility
#[cfg(target_os = "windows")]
pub use crate::audio_loopback::platform::windows::WindowsAudioBackend as WASAPILoopbackEnumerator;

// Platform-agnostic device enumeration
pub struct AudioDeviceEnumerator {
    backend: Box<dyn AudioCaptureBackend>,
}

impl AudioDeviceEnumerator {
    pub fn new() -> Result<Self> {
        let backend = get_audio_backend()?;
        Ok(Self { backend })
    }
    
    pub fn enumerate_loopback_devices(&self) -> Result<Vec<AudioLoopbackDevice>> {
        self.backend.enumerate_devices()
    }
    
    pub fn find_device_by_id(&self, device_id: &str) -> Result<Option<AudioLoopbackDevice>> {
        self.backend.find_device_by_id(device_id)
    }
    
    pub fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>> {
        self.backend.auto_select_best_device()
    }
}

// Tauri command implementations
#[tauri::command]
pub async fn enumerate_loopback_devices() -> Result<Vec<AudioLoopbackDevice>, String> {
    let enumerator = AudioDeviceEnumerator::new()
        .map_err(|e| format!("Failed to create device enumerator: {}", e))?;
    
    enumerator.enumerate_loopback_devices()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))
}

#[tauri::command]
pub async fn auto_select_best_device() -> Result<Option<AudioLoopbackDevice>, String> {
    let enumerator = AudioDeviceEnumerator::new()
        .map_err(|e| format!("Failed to create device enumerator: {}", e))?;
    
    enumerator.auto_select_best_device()
        .map_err(|e| format!("Failed to auto-select device: {}", e))
}

#[tauri::command]
pub async fn test_audio_device(device_id: String) -> Result<bool, String> {
    let enumerator = AudioDeviceEnumerator::new()
        .map_err(|e| format!("Failed to create device enumerator: {}", e))?;
    
    // Just check if the device exists
    match enumerator.find_device_by_id(&device_id) {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(format!("Error testing device: {}", e))
    }
}