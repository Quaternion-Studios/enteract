// src-tauri/src/audio_loopback/macos/capture_engine.rs
// macOS audio capture engine using CPAL
//
// Phase 2 (en-03o): macOS CPAL audio implementation
// References: Jack's research (resources/MACOS_AUDIO_LOOPBACK_RESEARCH.md)

use crate::audio_loopback::shared::audio_processor::{process_audio_for_transcription, SmartBuffer};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// macOS audio capture engine using CPAL
pub struct CPALCaptureEngine {
    device_id: String,
    is_capturing: bool,
    stream: Option<cpal::Stream>,
    smart_buffer: Arc<Mutex<SmartBuffer>>,
}

impl CPALCaptureEngine {
    pub fn new(device_id: String) -> Self {
        const TARGET_SAMPLE_RATE: u32 = 16000;
        Self {
            device_id,
            is_capturing: false,
            stream: None,
            smart_buffer: Arc::new(Mutex::new(SmartBuffer::new(TARGET_SAMPLE_RATE))),
        }
    }

    /// Start capturing audio from the specified device
    pub async fn start(&mut self, app_handle: tauri::AppHandle) -> Result<(), String> {
        if self.is_capturing {
            return Err("Already capturing".to_string());
        }

        // Get CPAL host and find device
        let host = cpal::default_host();
        let device = self
            .find_device(&host, &self.device_id)
            .ok_or("Device not found")?;

        // Get default input configuration
        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get device config: {}", e))?;

        let sample_rate = config.sample_rate();
        println!("[CPAL] Starting capture: device={}, sample_rate={:?}, channels={}, format={:?}",
            self.device_id, sample_rate, config.channels(), config.sample_format());

        // Clone smart buffer for callback
        let smart_buffer = Arc::clone(&self.smart_buffer);
        let app_handle_clone = app_handle.clone();

        // Build input stream based on sample format
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        Self::process_audio_callback(data, &smart_buffer, &app_handle_clone);
                    },
                    |err| eprintln!("[CPAL] Stream error: {}", err),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        // Convert i16 to f32
                        let f32_data: Vec<f32> = data
                            .iter()
                            .map(|&sample| sample as f32 / i16::MAX as f32)
                            .collect();
                        Self::process_audio_callback(&f32_data, &smart_buffer, &app_handle_clone);
                    },
                    |err| eprintln!("[CPAL] Stream error: {}", err),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        // Convert u16 to f32
                        let f32_data: Vec<f32> = data
                            .iter()
                            .map(|&sample| {
                                (sample as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)
                            })
                            .collect();
                        Self::process_audio_callback(&f32_data, &smart_buffer, &app_handle_clone);
                    },
                    |err| eprintln!("[CPAL] Stream error: {}", err),
                    None,
                )
            }
            _ => {
                return Err(format!("Unsupported sample format: {:?}", config.sample_format()));
            }
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        // Start the stream
        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.stream = Some(stream);
        self.is_capturing = true;

        Ok(())
    }

    /// Stop capturing audio
    pub async fn stop(&mut self) -> Result<(), String> {
        if !self.is_capturing {
            return Ok(());
        }

        // Pause and drop the stream
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
            drop(stream);
        }

        self.is_capturing = false;

        println!("[CPAL] Capture stopped");
        Ok(())
    }

    /// Check if currently capturing
    pub fn is_capturing(&self) -> bool {
        self.is_capturing
    }

    /// Audio callback handler with smart buffering
    ///
    /// Processes incoming audio data using silence-based segmentation.
    /// Called by CPAL on audio thread.
    fn process_audio_callback(
        data: &[f32],
        smart_buffer: &Arc<Mutex<SmartBuffer>>,
        app_handle: &tauri::AppHandle,
    ) {
        const TARGET_SAMPLE_RATE: u32 = 16000;

        // Add samples to smart buffer and check if a segment is ready
        if let Ok(mut buffer) = smart_buffer.lock() {
            if let Some(segment) = buffer.add_samples(data) {
                // Segment is ready for transcription
                println!("[CPAL] Smart segment ready: {:.2}s (silence-based)",
                         segment.len() as f32 / TARGET_SAMPLE_RATE as f32);

                // Process in background (don't block audio thread)
                let app_handle_clone = app_handle.clone();
                std::thread::spawn(move || {
                    // Convert f32 samples to PCM16 bytes (what process_audio_for_transcription expects)
                    let pcm_bytes: Vec<u8> = segment
                        .iter()
                        .flat_map(|&sample| {
                            let i16_sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            i16_sample.to_le_bytes()
                        })
                        .collect();

                    // Run async function in new Tokio runtime
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = process_audio_for_transcription(
                            pcm_bytes,
                            TARGET_SAMPLE_RATE,
                            app_handle_clone,
                        )
                        .await
                        {
                            eprintln!("[CPAL] Audio processing error: {}", e);
                        }
                    });
                });
            }
        }
    }

    /// Find CPAL device by ID (name)
    fn find_device(&self, host: &cpal::Host, device_id: &str) -> Option<cpal::Device> {
        let devices = host.input_devices().ok()?;

        for device in devices {
            if let Ok(name) = device.name() {
                if name == device_id {
                    return Some(device);
                }
            }
        }

        None
    }
}

impl Drop for CPALCaptureEngine {
    fn drop(&mut self) {
        // Ensure stream is stopped when engine is dropped
        if self.is_capturing {
            if let Some(stream) = self.stream.take() {
                let _ = stream.pause();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = CPALCaptureEngine::new("test_device".to_string());
        assert!(!engine.is_capturing());
        assert_eq!(engine.device_id, "test_device");
    }

    #[test]
    fn test_sample_conversion() {
        // Test i16 to f32 conversion
        let i16_samples: Vec<i16> = vec![0, i16::MAX, i16::MIN];
        let f32_samples: Vec<f32> = i16_samples
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect();

        assert!((f32_samples[0] - 0.0).abs() < 0.0001);
        assert!((f32_samples[1] - 1.0).abs() < 0.0001);
        assert!((f32_samples[2] + 1.0).abs() < 0.01); // i16::MIN / i16::MAX ≈ -1.0
    }
}
