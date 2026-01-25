// src-tauri/src/audio_loopback/shared/mod.rs
// Platform-agnostic audio processing and utilities

pub mod audio_processor;
pub mod quality_filter;
pub mod settings;
pub mod vad;

// Re-export for convenience
pub use audio_processor::*;
pub use quality_filter::*;
pub use settings::*;
pub use vad::*;
