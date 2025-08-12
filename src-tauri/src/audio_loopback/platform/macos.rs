// macOS-specific audio capture using cpal
use crate::audio_loopback::types::*;
use crate::audio_loopback::platform::{AudioCaptureBackend, AudioCaptureStream};
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;

pub struct MacOSAudioBackend {
    host: cpal::Host,
}

impl MacOSAudioBackend {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        Ok(Self { host })
    }
    
    fn device_to_loopback(&self, device: &cpal::Device, is_default: bool, is_input: bool) -> Result<AudioLoopbackDevice> {
        let name = device.name()
            .unwrap_or_else(|_| "Unknown Device".to_string());
        
        // Get supported config
        let config = if is_input {
            device.default_input_config()
        } else {
            device.default_output_config()
        }.map_err(|e| anyhow::anyhow!("Failed to get device config: {}", e))?;
        
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        
        // Generate a unique ID for the device
        let device_id = format!("{}_{}", 
            if is_input { "input" } else { "output" },
            name.replace(" ", "_").to_lowercase()
        );
        
        let display_name = if is_default {
            format!("{} (Default {})", name, if is_input { "Input" } else { "Output" })
        } else {
            name.clone()
        };
        
        Ok(AudioLoopbackDevice {
            id: device_id,
            name: display_name,
            is_default,
            sample_rate,
            channels,
            format: "f32".to_string(),
            device_type: if is_input { DeviceType::Capture } else { DeviceType::Render },
            loopback_method: if is_input { 
                LoopbackMethod::CaptureDevice 
            } else { 
                LoopbackMethod::RenderLoopback 
            },
        })
    }
}

impl AudioCaptureBackend for MacOSAudioBackend {
    fn enumerate_devices(&self) -> Result<Vec<AudioLoopbackDevice>> {
        let mut devices = Vec::new();
        
        // Get default devices
        let default_input = self.host.default_input_device();
        let default_output = self.host.default_output_device();
        
        let default_input_name = default_input.as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();
        let default_output_name = default_output.as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();
        
        // Enumerate input devices (microphones, etc.)
        for device in self.host.input_devices()
            .map_err(|e| anyhow::anyhow!("Failed to enumerate input devices: {}", e))? {
            
            let device_name = device.name().unwrap_or_default();
            let is_default = device_name == default_input_name;
            
            if let Ok(loopback_device) = self.device_to_loopback(&device, is_default, true) {
                devices.push(loopback_device);
            }
        }
        
        // Note: macOS doesn't natively support loopback capture of output devices
        // through Core Audio. We can only capture from input devices.
        // For system audio capture, users would need to use a virtual audio device
        // like BlackHole or Loopback.app
        
        // Add a note about system audio capture limitations
        if devices.is_empty() {
            println!("⚠️  No audio input devices found.");
            println!("    For system audio capture on macOS, consider installing:");
            println!("    - BlackHole (https://github.com/ExistentialAudio/BlackHole)");
            println!("    - Loopback.app (https://rogueamoeba.com/loopback/)");
        }
        
        Ok(devices)
    }
    
    fn find_device_by_id(&self, device_id: &str) -> Result<Option<AudioLoopbackDevice>> {
        let devices = self.enumerate_devices()?;
        Ok(devices.into_iter().find(|d| d.id == device_id))
    }
    
    fn start_capture(&self, device_id: &str) -> Result<AudioCaptureStream> {
        let device_info = self.find_device_by_id(device_id)?
            .ok_or_else(|| anyhow::anyhow!("Device not found"))?;
        
        // Find the actual cpal device
        let is_input = matches!(device_info.device_type, DeviceType::Capture);
        let devices: Vec<cpal::Device> = if is_input {
            self.host.input_devices()
                .map_err(|e| anyhow::anyhow!("Failed to get input devices: {}", e))?
                .collect()
        } else {
            // macOS doesn't support output device capture directly
            return Err(anyhow::anyhow!("macOS does not support direct output device capture. Please use a virtual audio device."));
        };
        
        let device = devices.into_iter()
            .find(|d| {
                if let Ok(name) = d.name() {
                    let expected_name = device_info.id.replace("input_", "").replace("_", " ");
                    name.to_lowercase() == expected_name.to_lowercase()
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Could not find device: {}", device_info.name))?;
        
        // Get the device config
        let config = device.default_input_config()
            .map_err(|e| anyhow::anyhow!("Failed to get input config: {}", e))?;
        
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        
        // Create channel for audio data
        let (tx, rx) = mpsc::channel();
        
        // Build and start the stream
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let _ = tx.send(data.to_vec());
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None
                )?
            },
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let samples: Vec<f32> = data.iter()
                            .map(|&s| s as f32 / i16::MAX as f32)
                            .collect();
                        let _ = tx.send(samples);
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None
                )?
            },
            cpal::SampleFormat::U16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let samples: Vec<f32> = data.iter()
                            .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                            .collect();
                        let _ = tx.send(samples);
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None
                )?
            },
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };
        
        stream.play()
            .map_err(|e| anyhow::anyhow!("Failed to start stream: {}", e))?;
        
        // Keep the stream alive
        let _stream = Box::leak(Box::new(stream));
        
        Ok(AudioCaptureStream {
            sample_rate,
            channels,
            receiver: rx,
            stop_handle: Box::new(move || {
                // In a real implementation, we'd properly manage stream lifetime
                Ok(())
            }),
        })
    }
    
    fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>> {
        let devices = self.enumerate_devices()?;
        
        // On macOS, prefer the default input device
        if let Some(device) = devices.iter()
            .find(|d| d.is_default && matches!(d.device_type, DeviceType::Capture)) {
            return Ok(Some(device.clone()));
        }
        
        // Otherwise, return the first available input device
        Ok(devices.into_iter()
            .find(|d| matches!(d.device_type, DeviceType::Capture)))
    }
}