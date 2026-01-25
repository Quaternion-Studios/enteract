// src-tauri/src/audio_loopback/traits.rs
// Platform-agnostic trait definitions for audio loopback
//
// These traits abstract over WASAPI (Windows) and CPAL/CoreAudio (macOS),
// enabling dependency injection and comprehensive unit testing with mocks.

use crate::audio_loopback::types::AudioLoopbackDevice;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for enumerating audio devices capable of loopback capture
///
/// Implementors:
/// - Windows: WASAPILoopbackEnumerator (uses wasapi crate)
/// - macOS: CPALLoopbackEnumerator (uses cpal 0.17+)
#[async_trait]
pub trait AudioDeviceEnumerator: Send + Sync {
    /// Enumerate all available loopback devices on the system
    ///
    /// Returns a list of devices capable of capturing system audio.
    /// The list may include:
    /// - Render devices with loopback capability (Windows WASAPI, macOS CoreAudio Taps)
    /// - Virtual audio devices (BlackHole on macOS, Stereo Mix on Windows)
    /// - Capture devices configured for loopback
    fn enumerate_loopback_devices(&self) -> Result<Vec<AudioLoopbackDevice>>;

    /// Find a specific device by its unique ID
    ///
    /// # Arguments
    /// * `device_id` - Platform-specific device identifier (GUID on Windows, device UID on macOS)
    fn find_device_by_id(&self, device_id: &str) -> Result<Option<AudioLoopbackDevice>>;

    /// Automatically select the best available loopback device
    ///
    /// Selection priority:
    /// 1. Default render device with loopback
    /// 2. Any render device with loopback
    /// 3. Virtual loopback device (Stereo Mix, BlackHole)
    /// 4. First available capture device
    fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>>;

    /// Test if a device is currently accessible and functional
    ///
    /// # Arguments
    /// * `device_id` - Device ID to test
    ///
    /// # Returns
    /// `true` if the device can be initialized for capture, `false` otherwise
    fn test_device_capability(&self, device_id: &str) -> bool;
}

/// Trait for capturing audio from a loopback device
///
/// Implementors:
/// - Windows: WASAPILoopbackCapture
/// - macOS: CPALLoopbackCapture
///
/// # Lifecycle
/// 1. Create instance with device ID
/// 2. Call `start()` to begin capture
/// 3. Periodically call `get_audio_samples()` to retrieve data
/// 4. Call `stop()` when done
#[async_trait]
pub trait AudioCaptureEngine: Send + Sync {
    /// Start capturing audio from the device
    ///
    /// # Arguments
    /// * `device_id` - ID of the device to capture from
    ///
    /// # Returns
    /// Ok(()) if capture started successfully, Err otherwise
    ///
    /// # Notes
    /// - This is async because initialization may involve COM/OS calls
    /// - Capture runs in a background thread/task
    /// - Calling start() while already capturing should return an error
    async fn start(&mut self, device_id: &str) -> Result<()>;

    /// Stop capturing audio
    ///
    /// Gracefully stops the capture thread and releases device resources.
    async fn stop(&mut self) -> Result<()>;

    /// Check if currently capturing
    fn is_capturing(&self) -> bool;

    /// Retrieve captured audio samples (non-blocking)
    ///
    /// # Returns
    /// Vector of f32 samples (normalized to [-1.0, 1.0])
    /// Empty vector if no new samples available
    ///
    /// # Format
    /// - Mono (1 channel)
    /// - 16 kHz sample rate (resampled for Whisper)
    /// - f32 samples in range [-1.0, 1.0]
    fn get_audio_samples(&mut self) -> Vec<f32>;
}

/// Trait for audio processing operations (platform-agnostic)
///
/// This includes operations that are the same across platforms:
/// - Format conversion (PCM16 -> f32)
/// - Resampling (device rate -> 16kHz for Whisper)
/// - Channel mixing (stereo -> mono)
/// - Audio level calculation
/// - DC offset removal
pub trait AudioProcessor: Send + Sync {
    /// Process raw audio chunk from device into normalized samples
    ///
    /// # Arguments
    /// * `data` - Raw audio bytes from device
    /// * `bits_per_sample` - 16 or 32
    /// * `channels` - 1 (mono) or 2 (stereo)
    /// * `source_sample_rate` - Device sample rate (e.g., 48000 Hz)
    /// * `target_sample_rate` - Target rate for output (typically 16000 Hz for Whisper)
    ///
    /// # Returns
    /// Vector of normalized f32 samples at target sample rate
    fn process_audio_chunk(
        &self,
        data: &[u8],
        bits_per_sample: u16,
        channels: u16,
        source_sample_rate: u32,
        target_sample_rate: u32,
    ) -> Vec<f32>;

    /// Calculate audio level in dB
    ///
    /// # Arguments
    /// * `samples` - Normalized f32 samples
    ///
    /// # Returns
    /// Audio level in dBFS (decibels relative to full scale)
    /// Range: -∞ dB (silence) to 0 dB (full scale)
    fn calculate_audio_level(&self, samples: &[f32]) -> f32;

    /// Remove DC offset from audio samples
    ///
    /// DC offset is a constant value added to all samples, which can
    /// cause pops and clicks in audio processing.
    ///
    /// # Arguments
    /// * `samples` - Mutable slice of f32 samples
    fn remove_dc_offset(&self, samples: &mut [f32]);
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests verify that our traits can be used with mockall
    // The actual mock implementations will be in the test module

    #[test]
    fn test_traits_are_object_safe() {
        // This test ensures our traits can be used as trait objects
        // which is required for dependency injection
        fn _assert_object_safe(
            _enumerator: Box<dyn AudioDeviceEnumerator>,
            _capture: Box<dyn AudioCaptureEngine>,
            _processor: Box<dyn AudioProcessor>,
        ) {
            // If this compiles, the traits are object-safe
        }
    }
}
