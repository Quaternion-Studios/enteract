// src-tauri/src/audio_loopback/types.rs
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// Audio capture state management
lazy_static::lazy_static! {
    pub static ref CAPTURE_STATE: Arc<Mutex<CaptureState>> = Arc::new(Mutex::new(CaptureState::default()));
}

#[derive(Default)]
pub struct CaptureState {
    pub is_capturing: bool,
    pub capture_handle: Option<tokio::task::JoinHandle<()>>,
    pub stop_tx: Option<mpsc::Sender<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLoopbackDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: String,
    pub device_type: DeviceType,
    pub loopback_method: LoopbackMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Render,
    Capture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopbackMethod {
    // Windows-specific methods
    RenderLoopback,  // WASAPI render endpoint loopback
    CaptureDevice,   // WASAPI capture device
    StereoMix,       // Legacy Windows Stereo Mix

    // macOS-specific methods
    #[cfg(target_os = "macos")]
    CoreAudioTap,    // macOS 14.2+ CoreAudio Taps (preferred)
    #[cfg(target_os = "macos")]
    VirtualDevice,   // Virtual audio devices (BlackHole, etc.)
    #[cfg(target_os = "macos")]
    ScreenCaptureKit,// macOS 13+ ScreenCaptureKit audio
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceSettings {
    #[serde(alias = "selected_loopback_device")]
    pub selectedLoopbackDevice: Option<String>,
    #[serde(alias = "loopback_enabled")]
    pub loopbackEnabled: bool,
    #[serde(alias = "buffer_size")]
    pub bufferSize: u32,
    #[serde(alias = "sample_rate")]
    pub sampleRate: u32,
}

impl Default for AudioDeviceSettings {
    fn default() -> Self {
        Self {
            selectedLoopbackDevice: None,
            loopbackEnabled: false,
            bufferSize: 4096,
            sampleRate: 16000,
        }
    }
}