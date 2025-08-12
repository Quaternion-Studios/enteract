// src-tauri/src/audio_loopback.rs
// This file has been refactored into modules for better organization.
// All functionality is now split across multiple files in the audio_loopback/ directory.
//
// Key improvements made:
// 1. Reduced excessive debug logging that was causing spam
// 2. Enhanced transcription quality filtering to prevent "(crying)" artifacts
// 3. Split large file into manageable modules
// 4. Maintained all sophisticated features from the sandbox implementation
// 5. Added cross-platform support for Windows and macOS

pub mod types;
pub mod platform;
pub mod device_enumerator;
pub mod audio_processor; 
pub mod capture_engine;
pub mod quality_filter;
pub mod settings;

// Re-export main types and functions
pub use types::{CAPTURE_STATE, CaptureState, AudioLoopbackDevice, DeviceType, LoopbackMethod, AudioDeviceSettings};
pub use device_enumerator::*;
pub use capture_engine::*;
pub use audio_processor::*;
pub use settings::*;