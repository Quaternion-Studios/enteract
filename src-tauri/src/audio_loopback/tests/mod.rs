// src-tauri/src/audio_loopback/tests/mod.rs
// Test module for macOS audio loopback implementation
//
// This module contains test stubs that will be implemented during Phase 1.
// All tests are marked #[ignore] until the implementation provides the
// necessary traits and functionality.
//
// To run tests (once implemented):
//   cargo test --lib                    # Run all non-ignored tests
//   cargo test --lib -- --ignored       # Run ignored tests
//   cargo test device_enumeration       # Run specific test module

#[cfg(test)]
#[cfg(target_os = "macos")]
mod device_enumeration_tests;

#[cfg(test)]
#[cfg(target_os = "macos")]
mod capture_lifecycle_tests;

#[cfg(test)]
#[cfg(target_os = "macos")]
mod sample_rate_tests;
