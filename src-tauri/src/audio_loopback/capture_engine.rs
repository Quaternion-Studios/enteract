// Platform-agnostic audio capture engine
use crate::audio_loopback::types::*;
use crate::audio_loopback::platform::{get_audio_backend, AudioCaptureBackend};
use crate::audio_loopback::audio_processor::calculate_audio_level;
use crate::audio_loopback::audio_diagnostics::{
    init_audio_logger, log_audio_event, log_audio_buffer_analysis, 
    log_device_info, log_error, validate_audio_format, flush_logs
};
use anyhow::Result;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use base64::prelude::*;
use serde_json;

// Platform-specific imports
#[cfg(target_os = "windows")]
use {
    wasapi::{DeviceCollection, Direction, Device, ShareMode, initialize_mta},
    crate::audio_loopback::platform::windows::WindowsAudioBackend,
    crate::audio_loopback::audio_processor::process_audio_chunk,
};

#[tauri::command]
pub async fn start_audio_loopback_capture(
    device_id: String,
    app_handle: AppHandle
) -> Result<String, String> {
    // Initialize diagnostics
    if let Err(e) = init_audio_logger() {
        println!("Warning: Failed to initialize audio logger: {}", e);
    }
    
    // Check if already capturing
    {
        let state = CAPTURE_STATE.lock().unwrap();
        if state.is_capturing {
            log_error("CAPTURE", "Already capturing", Some(serde_json::json!({"device_id": device_id})));
            return Err("Audio capture already in progress".to_string());
        }
    }
    
    log_audio_event("CAPTURE", "Starting audio capture", Some(serde_json::json!({"device_id": device_id})));
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

// Platform-specific capture loop for Windows
#[cfg(target_os = "windows")]
fn run_audio_capture_loop_sync(
    device_id: String,
    app_handle: AppHandle,
    mut stop_rx: mpsc::Receiver<()>
) -> Result<()> {
    initialize_mta().map_err(|_| anyhow::anyhow!("Failed to initialize COM"))?;
    
    let enumerator = WindowsAudioBackend::new()?;
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
    
    // Validate format
    let bits_per_sample = format.get_bitspersample();
    let channels = format.get_nchannels();
    
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
    
    // Transcription buffer setup
    let mut transcription_buffer: Vec<f32> = Vec::new();
    let transcription_buffer_duration = 4.0;
    let transcription_buffer_size = (16000.0 * transcription_buffer_duration) as usize;
    let mut last_transcription = Instant::now();
    let transcription_interval = Duration::from_millis(800);
    let min_audio_length = 1.5;
    let min_audio_samples = (16000.0 * min_audio_length) as usize;
    
    // Main capture loop
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
        
        // Process audio
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
            
            let buffer_rms = (transcription_buffer.iter().map(|&x| x * x).sum::<f32>() / transcription_buffer.len() as f32).sqrt();
            
            if buffer_rms > 0.00305 {
                let chunk_data = transcription_buffer[..std::cmp::min(transcription_buffer.len(), transcription_buffer_size)].to_vec();
                
                let base64_audio = BASE64_STANDARD.encode(&chunk_data.iter()
                    .flat_map(|&x| x.to_le_bytes().to_vec())
                    .collect::<Vec<u8>>());
                
                let payload = serde_json::json!({
                    "audio": base64_audio,
                    "sample_rate": 16000,
                    "channels": 1,
                    "bits_per_sample": 32,
                    "timestamp": start_time.elapsed().as_millis()
                });
                
                let _ = app_handle.emit("audio-chunk-ready", payload);
                
                last_transcription = now;
                let shift_amount = transcription_buffer_size / 2;
                if transcription_buffer.len() > shift_amount {
                    transcription_buffer.drain(0..shift_amount);
                }
            }
        }
        
        // Audio level updates
        if now.duration_since(last_emit) > Duration::from_millis(100) {
            let elapsed = start_time.elapsed().as_secs_f64();
            let samples_per_sec = if elapsed > 0.0 {
                (total_samples as f64 / elapsed) as u32
            } else {
                0
            };
            
            let level = calculate_audio_level(&processed_audio);
            
            let _ = app_handle.emit("audio-level", serde_json::json!({
                "level": level,
                "capturing": true,
                "samples_per_sec": samples_per_sec,
                "device": device_id
            }));
            
            last_emit = now;
        }
    }
    
    // Cleanup
    let _ = audio_client.stop_stream();
    
    println!("🛑 Audio capture stopped");
    Ok(())
}

// Platform-specific capture loop for macOS and others
#[cfg(not(target_os = "windows"))]
fn run_audio_capture_loop_sync(
    device_id: String,
    app_handle: AppHandle,
    mut stop_rx: mpsc::Receiver<()>
) -> Result<()> {
    use rubato::{Resampler, FftFixedInOut};
    let backend = get_audio_backend()?;
    let stream = backend.start_capture(&device_id)?;
    
    println!("✅ Audio capture initialized - {} Hz, {} channels", 
             stream.sample_rate, stream.channels);
    
    let start_time = Instant::now();
    let mut total_samples = 0u64;
    let mut last_emit = Instant::now();
    
    // Transcription buffer setup
    let mut transcription_buffer: Vec<f32> = Vec::new();
    let transcription_buffer_duration = 4.0;
    let transcription_buffer_size = (16000.0 * transcription_buffer_duration) as usize;
    let mut last_transcription = Instant::now();
    let transcription_interval = Duration::from_millis(800);
    let min_audio_length = 1.5;
    let min_audio_samples = (16000.0 * min_audio_length) as usize;
    
    // Main capture loop
    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        
        // Try to receive audio data
        match stream.receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(audio_data) => {
                // Process if we need to resample
                let processed_audio = if stream.sample_rate != 16000 {
                    // Resample to 16kHz
                    let resampler = rubato::FftFixedInOut::<f32>::new(
                        stream.sample_rate as usize,
                        16000,
                        audio_data.len() / stream.channels as usize,
                        stream.channels as usize,
                    );
                    
                    if let Ok(mut resampler) = resampler {
                        let mut input = vec![Vec::new(); stream.channels as usize];
                        for (i, sample) in audio_data.iter().enumerate() {
                            input[i % stream.channels as usize].push(*sample);
                        }
                        
                        if let Ok(output) = resampler.process(&input, None) {
                            // Convert to mono if needed
                            if output.len() > 1 {
                                output[0].iter()
                                    .zip(output[1].iter())
                                    .map(|(l, r)| (l + r) / 2.0)
                                    .collect()
                            } else {
                                output[0].clone()
                            }
                        } else {
                            audio_data
                        }
                    } else {
                        audio_data
                    }
                } else {
                    audio_data
                };
                
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
                    
                    let buffer_rms = (transcription_buffer.iter().map(|&x| x * x).sum::<f32>() / transcription_buffer.len() as f32).sqrt();
                    
                    if buffer_rms > 0.00305 {
                        let chunk_data = transcription_buffer[..std::cmp::min(transcription_buffer.len(), transcription_buffer_size)].to_vec();
                        
                        let base64_audio = BASE64_STANDARD.encode(&chunk_data.iter()
                            .flat_map(|&x| x.to_le_bytes().to_vec())
                            .collect::<Vec<u8>>());
                        
                        let payload = serde_json::json!({
                            "audio": base64_audio,
                            "sample_rate": 16000,
                            "channels": 1,
                            "bits_per_sample": 32,
                            "timestamp": start_time.elapsed().as_millis()
                        });
                        
                        let _ = app_handle.emit("audio-chunk-ready", payload);
                        
                        last_transcription = now;
                        let shift_amount = transcription_buffer_size / 2;
                        if transcription_buffer.len() > shift_amount {
                            transcription_buffer.drain(0..shift_amount);
                        }
                    }
                }
                
                // Audio level updates
                if now.duration_since(last_emit) > Duration::from_millis(100) {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let samples_per_sec = if elapsed > 0.0 {
                        (total_samples as f64 / elapsed) as u32
                    } else {
                        0
                    };
                    
                    let level = calculate_audio_level(&processed_audio);
                    
                    let _ = app_handle.emit("audio-level", serde_json::json!({
                        "level": level,
                        "capturing": true,
                        "samples_per_sec": samples_per_sec,
                        "device": device_id.clone()
                    }));
                    
                    last_emit = now;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // No data available, continue
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Stream disconnected
                break;
            }
        }
    }
    
    // Call stop handle
    if let Err(e) = (stream.stop_handle)() {
        eprintln!("Error stopping stream: {}", e);
    }
    
    println!("🛑 Audio capture stopped");
    Ok(())
}

// Helper function for Windows
#[cfg(target_os = "windows")]
fn find_wasapi_device(device_info: &AudioLoopbackDevice) -> Result<Device> {
    let collection = match device_info.device_type {
        DeviceType::Render => DeviceCollection::new(&Direction::Render),
        DeviceType::Capture => DeviceCollection::new(&Direction::Capture),
    }.map_err(|_| anyhow::anyhow!("Failed to create device collection"))?;
    
    let count = collection.get_count()
        .map_err(|_| anyhow::anyhow!("Failed to get device count"))?;
    
    for i in 0..count {
        let device = collection.get_device_at_index(i)
            .map_err(|_| anyhow::anyhow!("Failed to get device at index {}", i))?;
        
        if let Ok(id) = device.get_id() {
            if id == device_info.id {
                return Ok(device);
            }
        }
    }
    
    Err(anyhow::anyhow!("Device not found: {}", device_info.id))
}