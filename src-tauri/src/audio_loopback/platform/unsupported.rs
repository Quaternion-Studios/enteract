// Unsupported platform stub
use crate::audio_loopback::types::*;
use crate::audio_loopback::platform::{AudioCaptureBackend, AudioCaptureStream};
use anyhow::Result;

pub struct UnsupportedBackend;

impl AudioCaptureBackend for UnsupportedBackend {
    fn enumerate_devices(&self) -> Result<Vec<AudioLoopbackDevice>> {
        Err(anyhow::anyhow!("Audio loopback is not supported on this platform"))
    }
    
    fn find_device_by_id(&self, _device_id: &str) -> Result<Option<AudioLoopbackDevice>> {
        Err(anyhow::anyhow!("Audio loopback is not supported on this platform"))
    }
    
    fn start_capture(&self, _device_id: &str) -> Result<AudioCaptureStream> {
        Err(anyhow::anyhow!("Audio loopback is not supported on this platform"))
    }
    
    fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>> {
        Err(anyhow::anyhow!("Audio loopback is not supported on this platform"))
    }
}