// src-tauri/src/audio_loopback/capture_engine.rs
use crate::audio_loopback::types::*;
use crate::audio_loopback::device_enumerator::WASAPILoopbackEnumerator;
use crate::audio_loopback::audio_processor::{process_audio_for_transcription, process_audio_chunk, calculate_audio_level};
use anyhow::Result;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use wasapi::{DeviceCollection, Direction, Device, ShareMode, initialize_mta};
use base64::prelude::*;
use serde_json;

#[tauri::command]
pub async fn start_audio_loopback_capture(
    device_id: String,
    app_handle: AppHandle
) -> Result<String, String> {
    // Check if already capturing
    {
        let state = CAPTURE_STATE.lock().unwrap();
        if state.is_capturing {
            return Err("Audio capture already in progress".to_string());
        }
    }
    
    println!("🎤 Starting audio capture for device: {}", device_id);
    
    // Create stop channel
    let (stop_tx, stop_rx) = mpsc::channel::<()>(1);
    
    // Start capture in background thread
    let app_handle_clone = app_handle.clone();
    let device_id_clone = device_id.clone();
    
    let handle = tokio::task::spawn_blocking(move || {
        if let Err(e) = run_audio_capture_loop_sync(device_id_clone, app_handle_clone, stop_rx) {
            eprintln!("Audio capture error: {}", e);
        }
    });
    
    // Update state
    {
        let mut state = CAPTURE_STATE.lock().unwrap();
        state.is_capturing = true;
        state.capture_handle = Some(handle);
        state.stop_tx = Some(stop_tx);
    }
    
    Ok("Audio capture started".to_string())
}

#[tauri::command]
pub async fn stop_audio_loopback_capture() -> Result<(), String> {
    println!("⏹️ Stopping audio capture");
    
    let (stop_tx, handle) = {
        let mut state = CAPTURE_STATE.lock().unwrap();
        state.is_capturing = false;
        (state.stop_tx.take(), state.capture_handle.take())
    };
    
    // Send stop signal
    if let Some(tx) = stop_tx {
        let _ = tx.send(()).await;
    }
    
    // Wait for task to complete
    if let Some(handle) = handle {
        let _ = handle.await;
    }
    
    Ok(())
}

// Main audio capture loop with reduced logging
fn run_audio_capture_loop_sync(
    device_id: String,
    app_handle: AppHandle,
    mut stop_rx: mpsc::Receiver<()>
) -> Result<()> {
    initialize_mta().map_err(|_| anyhow::anyhow!("Failed to initialize COM"))?;
    
    let enumerator = WASAPILoopbackEnumerator::new()?;
    let device_info = enumerator.find_device_by_id(&device_id)?
        .ok_or_else(|| anyhow::anyhow!("Device not found"))?;
    
    let wasapi_device = find_wasapi_device(&device_info)?;
    
    // Setup audio client
    let mut audio_client = wasapi_device.get_iaudioclient()
        .map_err(|_| anyhow::anyhow!("Failed to get audio client"))?;
    let format = audio_client.get_mixformat()
        .map_err(|_| anyhow::anyhow!("Failed to get mix format"))?;
    let (_, min_time) = audio_client.get_periods()
        .map_err(|_| anyhow::anyhow!("Failed to get periods"))?;
    
    // Always use Direction::Capture for loopback capture
    let (direction, use_loopback) = match device_info.device_type {
        DeviceType::Render => (Direction::Capture, true),
        DeviceType::Capture => (Direction::Capture, false),
    };
    
    // Initialize with retry logic
    let mut init_attempts = 0;
    let max_attempts = 3;
    
    loop {
        init_attempts += 1;
        
        match audio_client.initialize_client(&format, min_time, &direction, &ShareMode::Shared, use_loopback) {
            Ok(_) => break,
            Err(_) => {
                if init_attempts >= max_attempts {
                    return Err(anyhow::anyhow!(
                        "Failed to initialize audio client after {} attempts. Device may be busy.", 
                        max_attempts
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
                audio_client = wasapi_device.get_iaudioclient()
                    .map_err(|_| anyhow::anyhow!("Failed to get fresh audio client"))?;
            }
        }
    }
    
    // Get capture client
    let capture_client = audio_client.get_audiocaptureclient()
        .map_err(|_| anyhow::anyhow!("Failed to get capture client"))?;
    let h_event = audio_client.set_get_eventhandle()
        .map_err(|_| anyhow::anyhow!("Failed to get event handle"))?;
    
    println!("✅ Audio capture initialized - {} Hz, {} channels, {} bits", 
             format.get_samplespersec(), format.get_nchannels(), format.get_bitspersample());
    println!("📊 Device sample rate: {} Hz, Whisper target: 16000 Hz", format.get_samplespersec());
    
    // Validate format
    let bits_per_sample = format.get_bitspersample();
    let channels = format.get_nchannels();
    let _sample_rate = format.get_samplespersec();  // Unused variable warning fix
    
    if bits_per_sample != 16 && bits_per_sample != 32 {
        return Err(anyhow::anyhow!("Unsupported bits per sample: {}", bits_per_sample));
    }
    
    // Start the stream
    audio_client.start_stream()
        .map_err(|_| anyhow::anyhow!("Failed to start stream"))?;
    
    std::thread::sleep(Duration::from_millis(100));
    
    let start_time = Instant::now();
    let mut total_samples = 0u64;
    let mut last_emit = Instant::now();
    let mut error_count = 0u32;
    
    // Transcription buffer setup - MATCHING PYTHON CONFIG
    let mut transcription_buffer: Vec<f32> = Vec::new();
    let transcription_buffer_duration = 4.0;  // Python: BUFFER_DURATION = 4.0
    // Important: Buffer size is at 16kHz (Whisper rate), not device rate
    let transcription_buffer_size = (16000.0 * transcription_buffer_duration) as usize;
    let mut last_transcription = Instant::now();
    let transcription_interval = Duration::from_millis(800);  // Python: PROCESSING_INTERVAL = 0.8
    let min_audio_length = 1.5;  // Python: MIN_AUDIO_LENGTH = 1.5
    let min_audio_samples = (16000.0 * min_audio_length) as usize;  // At 16kHz
    
    // Main capture loop with reduced logging
    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        
        if h_event.wait_for_event(100).is_err() {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        
        let frames_available = match capture_client.get_next_nbr_frames() {
            Ok(Some(frames)) if frames > 0 => frames,
            _ => {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
        };
        
        let bytes_per_sample = bits_per_sample / 8;
        let bytes_per_frame = bytes_per_sample * channels as u16;
        
        if frames_available == 0 || bytes_per_frame == 0 {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        
        let calculated_buffer_size = frames_available as usize * bytes_per_frame as usize;
        if calculated_buffer_size > 1_048_576 {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        
        let safe_buffer_size = std::cmp::max(calculated_buffer_size, 4096);
        let mut buffer = vec![0u8; safe_buffer_size];
        
        let (frames_read, _flags) = match capture_client.read_from_device(bytes_per_frame as usize, &mut buffer) {
            Ok(result) => {
                if error_count > 0 {
                    error_count = std::cmp::max(0, error_count - 1);
                }
                result
            },
            Err(_) => {
                error_count += 1;
                if error_count > 10 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
        };
        
        if frames_read == 0 {
            continue;
        }
        
        let actual_bytes = frames_read as usize * bytes_per_frame as usize;
        let actual_bytes = if actual_bytes > safe_buffer_size {
            safe_buffer_size
        } else {
            actual_bytes
        };
        
        let audio_data = &buffer[..actual_bytes];
        
        // Detect completely silent audio
        let is_completely_silent = audio_data.iter().all(|&b| b == 0);
        if is_completely_silent && frames_read > 100 {
            // Only log this occasionally to avoid spam
            if start_time.elapsed().as_secs() % 30 == 0 {
                println!("⚠️ No system audio detected - check device selection");
            }
        }
        
        // Process audio - MATCHING PYTHON PIPELINE
        // Python always outputs at 16kHz for Whisper
        let processed_audio = process_audio_chunk(
            audio_data,
            bits_per_sample,
            channels,
            format.get_samplespersec(),
            16000  // Always resample to 16kHz for Whisper
        );
        
        total_samples += processed_audio.len() as u64;
        transcription_buffer.extend_from_slice(&processed_audio);
        
        // Trim buffer
        if transcription_buffer.len() > transcription_buffer_size * 2 {
            let excess = transcription_buffer.len() - transcription_buffer_size;
            transcription_buffer.drain(0..excess);
        }
        
        // Try transcription
        let now = Instant::now();
        if transcription_buffer.len() >= min_audio_samples && 
           now.duration_since(last_transcription) > transcription_interval {
            
            // Python checks RMS > 100 for int16, which is ~0.00305 for float32
            let buffer_rms = (transcription_buffer.iter().map(|&x| x * x).sum::<f32>() / transcription_buffer.len() as f32).sqrt();
            let buffer_level = calculate_audio_level(&transcription_buffer);
            
            // Log buffer state every transcription attempt
            println!("[CAPTURE] Buffer: {} samples, RMS: {:.6}, Level: {:.1}dB", 
                     transcription_buffer.len(), buffer_rms, buffer_level);
            
            if buffer_rms > 0.00305 {  // Match Python's RMS threshold
                // The transcription buffer already contains mono f32 samples at 16kHz
                // We need to convert to stereo PCM16 bytes for the transcription function
                // which expects stereo input (it will convert back to mono)
                let int16_samples: Vec<i16> = transcription_buffer.iter()
                    .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
                    .collect();
                
                // Create stereo PCM16 by duplicating mono samples
                let mut stereo_pcm16_bytes = Vec::with_capacity(int16_samples.len() * 4);
                for &sample in &int16_samples {
                    let bytes = sample.to_le_bytes();
                    stereo_pcm16_bytes.extend_from_slice(&bytes);  // Left channel
                    stereo_pcm16_bytes.extend_from_slice(&bytes);  // Right channel (duplicate)
                }
                let pcm16_bytes = stereo_pcm16_bytes;
                
                let app_handle_clone = app_handle.clone();
                let audio_bytes_clone = pcm16_bytes.clone();
                // Important: We're passing 16kHz since we already resampled
                let sample_rate = 16000;
                
                println!("[CAPTURE] Sending {} bytes for transcription (RMS: {:.6})", 
                         pcm16_bytes.len(), buffer_rms);
                
                tokio::spawn(async move {
                    match process_audio_for_transcription(
                        audio_bytes_clone,
                        sample_rate,
                        app_handle_clone
                    ).await {
                        Ok(text) => {
                            if !text.is_empty() {
                                println!("[CAPTURE] Transcription result: '{}'", text);
                            }
                        },
                        Err(e) => println!("[CAPTURE] Transcription error: {}", e)
                    }
                });
                
                last_transcription = now;
                
                // Keep overlap - Python uses 1.0 second at 16kHz
                let overlap_duration = 1.0;
                let overlap_size = (16000.0 * overlap_duration) as usize;
                if transcription_buffer.len() > overlap_size {
                    let samples_to_remove = transcription_buffer.len() - overlap_size;
                    transcription_buffer.drain(0..samples_to_remove);
                }
            }
        }
        
        // Emit audio chunk periodically with reduced logging
        let now = Instant::now();
        if now.duration_since(last_emit) > Duration::from_millis(100) {
            let pcm16_data: Vec<i16> = processed_audio.iter()
                .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect();
            
            let audio_bytes: Vec<u8> = pcm16_data.iter()
                .flat_map(|&sample| sample.to_le_bytes())
                .collect();
            
            let level = calculate_audio_level(&processed_audio);
            
            let _emit_result = app_handle.emit("audio-chunk", serde_json::json!({
                "deviceId": device_id,
                "audioData": base64::prelude::BASE64_STANDARD.encode(&audio_bytes),
                "sampleRate": device_info.sample_rate,
                "channels": 1,
                "level": level,
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "duration": start_time.elapsed().as_secs(),
                "totalSamples": total_samples
            }));
            
            last_emit = now;
        }
    }
    
    let _ = audio_client.stop_stream();
    println!("Audio capture stopped");
    
    Ok(())
}

// Helper function to find WASAPI device
fn find_wasapi_device(device_info: &AudioLoopbackDevice) -> Result<Device> {
    let direction = match device_info.device_type {
        DeviceType::Render => Direction::Render,
        DeviceType::Capture => Direction::Capture,
    };
    
    let device_collection = DeviceCollection::new(&direction)
        .map_err(|_| anyhow::anyhow!("Failed to create device collection"))?;
    let device_count = device_collection.get_nbr_devices()
        .map_err(|_| anyhow::anyhow!("Failed to get device count"))?;
    
    for i in 0..device_count {
        if let Ok(device) = device_collection.get_device_at_index(i) {
            if let Ok(id) = device.get_id() {
                if id == device_info.id {
                    return Ok(device);
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("Could not find device with ID: {}", device_info.id))
}