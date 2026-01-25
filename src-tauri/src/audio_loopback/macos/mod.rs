// src-tauri/src/audio_loopback/macos/mod.rs
// macOS audio loopback implementation (CPAL/CoreAudio)
//
// Phase 2 (en-03o): macOS CPAL audio implementation
// - Device enumeration via CPAL
// - CoreAudio Taps for macOS 14.2+
// - Fallback to virtual devices (BlackHole) for older macOS

pub mod device_enumerator;
pub mod capture_engine;

pub use device_enumerator::*;
pub use capture_engine::*;
