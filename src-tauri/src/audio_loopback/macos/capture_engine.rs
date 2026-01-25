// src-tauri/src/audio_loopback/macos/capture_engine.rs
// macOS audio capture engine using CPAL
//
// Phase 2 (en-03o): macOS CPAL audio implementation
// References: Jack's research (resources/MACOS_AUDIO_LOOPBACK_RESEARCH.md)

use crate::audio_loopback::shared::audio_processor::process_audio_for_transcription;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// macOS audio capture engine using CPAL
pub struct CPALCaptureEngine {
    device_id: String,
    is_capturing: bool,
    stream: Option<cpal::Stream>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    stop_tx: Option<mpsc::Sender<()>>,
}

impl CPALCaptureEngine {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            is_capturing: false,
            stream: None,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            stop_tx: None,
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

        // Create channel for stop signal
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        // Clone audio buffer for callback
        let audio_buffer = Arc::clone(&self.audio_buffer);
        let app_handle_clone = app_handle.clone();

        // Build input stream based on sample format
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        Self::process_audio_callback(data, &audio_buffer, &app_handle_clone);
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
                        Self::process_audio_callback(&f32_data, &audio_buffer, &app_handle_clone);
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
                        Self::process_audio_callback(&f32_data, &audio_buffer, &app_handle_clone);
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

        // Spawn task to wait for stop signal
        let stream_handle = Arc::new(Mutex::new(self.stream.take()));
        tokio::spawn(async move {
            let _ = stop_rx.recv().await;
            if let Some(stream) = stream_handle.lock().unwrap().take() {
                let _ = stream.pause();
                println!("[CPAL] Stream stopped");
            }
        });

        Ok(())
    }

    /// Stop capturing audio
    pub async fn stop(&mut self) -> Result<(), String> {
        if !self.is_capturing {
            return Ok(());
        }

        // Send stop signal
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(()).await;
        }

        // Clear audio buffer
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            buffer.clear();
        }

        self.stream = None;
        self.is_capturing = false;

        println!("[CPAL] Capture stopped");
        Ok(())
    }

    /// Check if currently capturing
    pub fn is_capturing(&self) -> bool {
        self.is_capturing
    }

    /// Get audio samples from buffer
    pub fn get_audio_samples(&mut self) -> Vec<f32> {
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            let samples = buffer.clone();
            buffer.clear();
            samples
        } else {
            Vec::new()
        }
    }

    /// Audio callback handler
    ///
    /// Processes incoming audio data and buffers it for transcription.
    /// Called by CPAL on audio thread.
    fn process_audio_callback(
        data: &[f32],
        audio_buffer: &Arc<Mutex<Vec<f32>>>,
        app_handle: &tauri::AppHandle,
    ) {
        // Append to buffer
        if let Ok(mut buffer) = audio_buffer.lock() {
            buffer.extend_from_slice(data);

            // Process when we have enough samples (4 seconds at 16kHz = 64,000 samples)
            const TARGET_SAMPLE_RATE: u32 = 16000;
            const BUFFER_DURATION_SECS: u32 = 4;
            const BUFFER_SIZE: usize = (TARGET_SAMPLE_RATE * BUFFER_DURATION_SECS) as usize;

            if buffer.len() >= BUFFER_SIZE {
                // Take samples for processing
                let samples_to_process: Vec<f32> = buffer.drain(..BUFFER_SIZE).collect();

                // Process in background (don't block audio thread)
                let app_handle_clone = app_handle.clone();
                tokio::spawn(async move {
                    // Convert f32 samples to PCM16 bytes (what process_audio_for_transcription expects)
                    let pcm_bytes: Vec<u8> = samples_to_process
                        .iter()
                        .flat_map(|&sample| {
                            let i16_sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            i16_sample.to_le_bytes()
                        })
                        .collect();

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
