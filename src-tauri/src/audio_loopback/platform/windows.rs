// Windows-specific audio capture using WASAPI
use crate::audio_loopback::types::*;
use crate::audio_loopback::platform::{AudioCaptureBackend, AudioCaptureStream};
use anyhow::Result;
use std::sync::mpsc;
use wasapi::{DeviceCollection, Direction, Device, ShareMode, get_default_device, initialize_mta};

pub struct WindowsAudioBackend {
    render_collection: DeviceCollection,
    capture_collection: DeviceCollection,
}

impl WindowsAudioBackend {
    pub fn new() -> Result<Self> {
        initialize_mta()
            .map_err(|_| anyhow::anyhow!("Failed to initialize COM"))?;
            
        let render_collection = DeviceCollection::new(&Direction::Render)
            .map_err(|_| anyhow::anyhow!("Failed to create render device collection"))?;
        let capture_collection = DeviceCollection::new(&Direction::Capture)
            .map_err(|_| anyhow::anyhow!("Failed to create capture device collection"))?;
        
        Ok(Self { 
            render_collection,
            capture_collection,
        })
    }
    
    fn scan_render_devices(&self, default_id: &str) -> Result<Vec<AudioLoopbackDevice>> {
        let mut devices = Vec::new();
        let count = self.render_collection.get_count()
            .map_err(|_| anyhow::anyhow!("Failed to get render device count"))?;
        
        for i in 0..count {
            let device = self.render_collection.get_device_at_index(i)
                .map_err(|_| anyhow::anyhow!("Failed to get render device at index {}", i))?;
            
            if let Ok(name) = device.get_friendlyname() {
                if let Ok(id) = device.get_id() {
                    let is_default = id == default_id;
                    
                    devices.push(AudioLoopbackDevice {
                        id: id.clone(),
                        name: if is_default { 
                            format!("{} (Default Output)", name) 
                        } else { 
                            name.clone() 
                        },
                        is_default,
                        sample_rate: 48000,
                        channels: 2,
                        format: "f32".to_string(),
                        device_type: DeviceType::Render,
                        loopback_method: LoopbackMethod::RenderLoopback,
                    });
                }
            }
        }
        
        Ok(devices)
    }
    
    fn scan_capture_devices(&self, default_id: &str) -> Result<Vec<AudioLoopbackDevice>> {
        let mut devices = Vec::new();
        let count = self.capture_collection.get_count()
            .map_err(|_| anyhow::anyhow!("Failed to get capture device count"))?;
        
        for i in 0..count {
            let device = self.capture_collection.get_device_at_index(i)
                .map_err(|_| anyhow::anyhow!("Failed to get capture device at index {}", i))?;
            
            if let Ok(name) = device.get_friendlyname() {
                let name_lower = name.to_lowercase();
                if name_lower.contains("stereo mix") || 
                   name_lower.contains("what u hear") || 
                   name_lower.contains("wave out mix") ||
                   name_lower.contains("loopback") {
                    
                    if let Ok(id) = device.get_id() {
                        let is_default = id == default_id;
                        
                        devices.push(AudioLoopbackDevice {
                            id: id.clone(),
                            name: if is_default { 
                                format!("{} (Default Input)", name) 
                            } else { 
                                name.clone() 
                            },
                            is_default,
                            sample_rate: 48000,
                            channels: 2,
                            format: "f32".to_string(),
                            device_type: DeviceType::Capture,
                            loopback_method: LoopbackMethod::StereoMix,
                        });
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    fn find_wasapi_device(&self, device_info: &AudioLoopbackDevice) -> Result<Device> {
        let collection = match device_info.device_type {
            DeviceType::Render => &self.render_collection,
            DeviceType::Capture => &self.capture_collection,
        };
        
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
}

impl AudioCaptureBackend for WindowsAudioBackend {
    fn enumerate_devices(&self) -> Result<Vec<AudioLoopbackDevice>> {
        let mut loopback_devices = Vec::new();
        
        let default_render = get_default_device(&Direction::Render).ok();
        let default_capture = get_default_device(&Direction::Capture).ok();
        
        let default_render_id = default_render.as_ref()
            .and_then(|d| d.get_id().ok())
            .unwrap_or_default();
        let default_capture_id = default_capture.as_ref()
            .and_then(|d| d.get_id().ok())
            .unwrap_or_default();
        
        if let Ok(render_devices) = self.scan_render_devices(&default_render_id) {
            loopback_devices.extend(render_devices);
        }
        
        if let Ok(capture_devices) = self.scan_capture_devices(&default_capture_id) {
            loopback_devices.extend(capture_devices);
        }
        
        if loopback_devices.is_empty() && default_render.is_some() {
            if let Ok(id) = default_render.as_ref().unwrap().get_id() {
                if let Ok(name) = default_render.as_ref().unwrap().get_friendlyname() {
                    loopback_devices.push(AudioLoopbackDevice {
                        id,
                        name: format!("{} (Default - Fallback)", name),
                        is_default: true,
                        sample_rate: 48000,
                        channels: 2,
                        format: "f32".to_string(),
                        device_type: DeviceType::Render,
                        loopback_method: LoopbackMethod::RenderLoopback,
                    });
                }
            }
        }
        
        Ok(loopback_devices)
    }
    
    fn find_device_by_id(&self, device_id: &str) -> Result<Option<AudioLoopbackDevice>> {
        let devices = self.enumerate_devices()?;
        Ok(devices.into_iter().find(|d| d.id == device_id))
    }
    
    fn start_capture(&self, device_id: &str) -> Result<AudioCaptureStream> {
        let device_info = self.find_device_by_id(device_id)?
            .ok_or_else(|| anyhow::anyhow!("Device not found"))?;
        
        let wasapi_device = self.find_wasapi_device(&device_info)?;
        
        // Setup audio client
        let mut audio_client = wasapi_device.get_iaudioclient()
            .map_err(|_| anyhow::anyhow!("Failed to get audio client"))?;
        let format = audio_client.get_mixformat()
            .map_err(|_| anyhow::anyhow!("Failed to get mix format"))?;
        
        let (direction, use_loopback) = match device_info.device_type {
            DeviceType::Render => (Direction::Capture, true),
            DeviceType::Capture => (Direction::Capture, false),
        };
        
        audio_client.initialize_client(
            &format,
            0,
            &direction,
            &ShareMode::Shared,
            use_loopback,
        ).map_err(|e| anyhow::anyhow!("Failed to initialize client: {:?}", e))?;
        
        let capture_client = audio_client.get_audiocaptureclient()
            .map_err(|_| anyhow::anyhow!("Failed to get capture client"))?;
        
        audio_client.start()
            .map_err(|_| anyhow::anyhow!("Failed to start audio client"))?;
        
        let (tx, rx) = mpsc::channel();
        let sample_rate = format.n_samples_per_sec;
        let channels = format.n_channels;
        
        // Create capture thread
        let handle = std::thread::spawn(move || {
            loop {
                if let Ok(packet_size) = capture_client.get_next_packet_size() {
                    if packet_size > 0 {
                        if let Ok(buffer) = capture_client.read_from_device(packet_size) {
                            let samples: Vec<f32> = buffer.iter()
                                .map(|&s| s as f32 / i16::MAX as f32)
                                .collect();
                            if tx.send(samples).is_err() {
                                break;
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        
        Ok(AudioCaptureStream {
            sample_rate,
            channels,
            receiver: rx,
            stop_handle: Box::new(move || {
                // Stop logic would go here
                Ok(())
            }),
        })
    }
    
    fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>> {
        let devices = self.enumerate_devices()?;
        
        // Priority: Default render device > Any render device > Stereo Mix > Any capture
        if let Some(device) = devices.iter()
            .find(|d| d.is_default && matches!(d.device_type, DeviceType::Render)) {
            return Ok(Some(device.clone()));
        }
        
        if let Some(device) = devices.iter()
            .find(|d| matches!(d.device_type, DeviceType::Render)) {
            return Ok(Some(device.clone()));
        }
        
        if let Some(device) = devices.iter()
            .find(|d| matches!(d.loopback_method, LoopbackMethod::StereoMix)) {
            return Ok(Some(device.clone()));
        }
        
        Ok(devices.into_iter().next())
    }
}