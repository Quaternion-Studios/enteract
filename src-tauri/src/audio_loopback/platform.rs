// Platform-specific audio capture implementations
use crate::audio_loopback::types::*;
use anyhow::Result;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub mod unsupported;

// Platform-agnostic interface
pub trait AudioCaptureBackend: Send + Sync {
    fn enumerate_devices(&self) -> Result<Vec<AudioLoopbackDevice>>;
    fn find_device_by_id(&self, device_id: &str) -> Result<Option<AudioLoopbackDevice>>;
    fn start_capture(&self, device_id: &str) -> Result<AudioCaptureStream>;
    fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>>;
}

pub struct AudioCaptureStream {
    pub sample_rate: u32,
    pub channels: u16,
    pub receiver: std::sync::mpsc::Receiver<Vec<f32>>,
    pub stop_handle: Box<dyn Send + Sync + Fn() -> Result<()>>,
}

// Factory function to get the appropriate backend
pub fn get_audio_backend() -> Result<Box<dyn AudioCaptureBackend>> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsAudioBackend::new()?))
    }
    
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSAudioBackend::new()?))
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Ok(Box::new(unsupported::UnsupportedBackend))
    }
}