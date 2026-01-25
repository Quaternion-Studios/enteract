// src-tauri/src/audio_loopback/mod.rs
// Audio loopback module for system audio capture
//
// Currently implemented for Windows using WASAPI.
// macOS implementation coming in Phase 1.

pub mod types;
pub mod settings;
pub mod audio_processor;
pub mod quality_filter;

// Platform-specific modules
#[cfg(target_os = "windows")]
pub mod device_enumerator;

#[cfg(target_os = "windows")]
pub mod capture_engine;

// TODO: Add macOS modules in Phase 1
// #[cfg(target_os = "macos")]
// pub mod macos;

// Test modules (macOS only for now)
#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests;
