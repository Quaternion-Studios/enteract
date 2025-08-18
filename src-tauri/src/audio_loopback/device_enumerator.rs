// src-tauri/src/audio_loopback/device_enumerator.rs
use crate::audio_loopback::types::*;
use anyhow::Result;
use wasapi::{DeviceCollection, Direction, Device, ShareMode, get_default_device, initialize_mta};

// WASAPI Device Enumerator Implementation
pub struct WASAPILoopbackEnumerator {
    render_collection: DeviceCollection,
    capture_collection: DeviceCollection,
}

impl WASAPILoopbackEnumerator {
    pub fn new() -> Result<Self> {
        // Initialize COM for WASAPI
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
    
    pub fn enumerate_loopback_devices(&self) -> Result<Vec<AudioLoopbackDevice>> {
        // println!("🔍 Scanning for loopback devices..."); // Commented out: Audio loopback is working, reducing console noise for debugging focus
        
        let mut loopback_devices = Vec::new();
        
        // Get default devices for comparison
        let default_render = get_default_device(&Direction::Render).ok();
        let default_capture = get_default_device(&Direction::Capture).ok();
        
        let default_render_id = default_render.as_ref().and_then(|d| d.get_id().ok()).unwrap_or_default();
        let default_capture_id = default_capture.as_ref().and_then(|d| d.get_id().ok()).unwrap_or_default();
        
        // Strategy 1: Try render devices with loopback
        if let Ok(render_devices) = self.scan_render_devices(&default_render_id) {
            loopback_devices.extend(render_devices);
        }
        
        // Strategy 2: Try capture devices that might be loopback
        if let Ok(capture_devices) = self.scan_capture_devices(&default_capture_id) {
            loopback_devices.extend(capture_devices);
        }
        
        // Strategy 3: Look for stereo mix and similar devices
        if let Ok(stereo_devices) = self.scan_stereo_mix_devices(&default_capture_id) {
            loopback_devices.extend(stereo_devices);
        }
        
        // Remove duplicates by ID
        loopback_devices.sort_by(|a, b| a.id.cmp(&b.id));
        loopback_devices.dedup_by(|a, b| a.id == b.id);
        
        // println!("✅ Found {} loopback device(s)", loopback_devices.len()); // Commented out: Audio loopback is working, reducing console noise for debugging focus
        Ok(loopback_devices)
    }
    
    fn scan_render_devices(&self, default_id: &str) -> Result<Vec<AudioLoopbackDevice>> {
        let device_count = self.render_collection.get_nbr_devices()
            .map_err(|_| anyhow::anyhow!("Failed to get render device count"))?;
        let mut devices = Vec::new();
        
        for i in 0..device_count {
            if let Ok(device) = self.render_collection.get_device_at_index(i) {
                if let Ok(device_info) = self.create_render_device_info(&device, default_id) {
                    if self.test_render_loopback_capability(&device) {
                        devices.push(device_info);
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    fn scan_capture_devices(&self, default_id: &str) -> Result<Vec<AudioLoopbackDevice>> {
        let device_count = self.capture_collection.get_nbr_devices()
            .map_err(|_| anyhow::anyhow!("Failed to get capture device count"))?;
        let mut devices = Vec::new();
        
        for i in 0..device_count {
            if let Ok(device) = self.capture_collection.get_device_at_index(i) {
                if let Ok(name) = device.get_friendlyname() {
                    if self.is_potential_loopback_capture_device(&name) {
                        if let Ok(device_info) = self.create_capture_device_info(&device, default_id) {
                            if self.test_capture_device_capability(&device) {
                                devices.push(device_info);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    fn scan_stereo_mix_devices(&self, default_id: &str) -> Result<Vec<AudioLoopbackDevice>> {
        let device_count = self.capture_collection.get_nbr_devices()
            .map_err(|_| anyhow::anyhow!("Failed to get stereo mix device count"))?;
        let mut devices = Vec::new();
        
        for i in 0..device_count {
            if let Ok(device) = self.capture_collection.get_device_at_index(i) {
                if let Ok(name) = device.get_friendlyname() {
                    if self.is_stereo_mix_device(&name) {
                        if let Ok(mut device_info) = self.create_capture_device_info(&device, default_id) {
                            device_info.loopback_method = LoopbackMethod::StereoMix;
                            if self.test_capture_device_capability(&device) {
                                devices.push(device_info);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    fn is_potential_loopback_capture_device(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        name_lower.contains("loopback") || 
        name_lower.contains("what u hear") ||
        name_lower.contains("what you hear") ||
        name_lower.contains("wave") ||
        name_lower.contains("mix") ||
        (name_lower.contains("speakers") && name_lower.contains("capture"))
    }
    
    fn is_stereo_mix_device(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        name_lower.contains("stereo mix") ||
        name_lower.contains("stereomix") ||
        name_lower.contains("wave") ||
        name_lower.contains("what u hear") ||
        name_lower.contains("what you hear")
    }
    
    fn create_render_device_info(&self, device: &Device, default_id: &str) -> Result<AudioLoopbackDevice> {
        let id = device.get_id()
            .map_err(|_| anyhow::anyhow!("Failed to get device ID"))?;
        let name = device.get_friendlyname().unwrap_or_else(|_| "Unknown Render Device".to_string());
        let is_default = id == default_id;
        
        let (sample_rate, channels, format) = self.get_device_format(device)?;
        
        Ok(AudioLoopbackDevice {
            id,
            name,
            is_default,
            sample_rate,
            channels,
            format,
            device_type: DeviceType::Render,
            loopback_method: LoopbackMethod::RenderLoopback,
        })
    }
    
    fn create_capture_device_info(&self, device: &Device, default_id: &str) -> Result<AudioLoopbackDevice> {
        let id = device.get_id()
            .map_err(|_| anyhow::anyhow!("Failed to get device ID"))?;
        let name = device.get_friendlyname().unwrap_or_else(|_| "Unknown Capture Device".to_string());
        let is_default = id == default_id;
        
        let (sample_rate, channels, format) = self.get_device_format(device)?;
        
        Ok(AudioLoopbackDevice {
            id,
            name,
            is_default,
            sample_rate,
            channels,
            format,
            device_type: DeviceType::Capture,
            loopback_method: LoopbackMethod::CaptureDevice,
        })
    }
    
    fn get_device_format(&self, device: &Device) -> Result<(u32, u16, String)> {
        let audio_client = device.get_iaudioclient()
            .map_err(|_| anyhow::anyhow!("Failed to get audio client"))?;
        let format = audio_client.get_mixformat()
            .map_err(|_| anyhow::anyhow!("Failed to get mix format"))?;
        
        let sample_rate = format.get_samplespersec();
        let channels = format.get_nchannels();
        let bits_per_sample = format.get_bitspersample();
        
        let format_str = if bits_per_sample == 32 {
            "IEEE Float 32bit".to_string()
        } else if bits_per_sample == 24 {
            "PCM 24bit".to_string()
        } else if bits_per_sample == 16 {
            "PCM 16bit".to_string()
        } else {
            format!("PCM {}bit", bits_per_sample)
        };
        
        Ok((sample_rate, channels, format_str))
    }
    
    pub fn test_render_loopback_capability(&self, device: &Device) -> bool {
        self.try_initialize_render_loopback(device).is_ok()
    }
    
    pub fn test_capture_device_capability(&self, device: &Device) -> bool {
        self.try_initialize_capture_device(device).is_ok()
    }
    
    fn try_initialize_render_loopback(&self, device: &Device) -> Result<()> {
        let mut audio_client = device.get_iaudioclient()
            .map_err(|_| anyhow::anyhow!("Failed to get audio client"))?;
        let mix_format = audio_client.get_mixformat()
            .map_err(|_| anyhow::anyhow!("Failed to get mix format"))?;
        
        // Use Direction::Capture for render loopback
        let result = audio_client.initialize_client(
            &mix_format,
            10_000_000, // 1 second buffer in 100ns units
            &Direction::Capture,
            &ShareMode::Shared,
            true, // use_loopback
        );
        
        if result.is_ok() {
            if audio_client.get_audiocaptureclient().is_ok() {
                return Ok(());
            }
        }
        
        result.map_err(|_| anyhow::anyhow!("WASAPI render loopback initialization failed"))
    }
    
    fn try_initialize_capture_device(&self, device: &Device) -> Result<()> {
        let mut audio_client = device.get_iaudioclient()
            .map_err(|_| anyhow::anyhow!("Failed to get audio client"))?;
        let mix_format = audio_client.get_mixformat()
            .map_err(|_| anyhow::anyhow!("Failed to get mix format"))?;
        
        let result = audio_client.initialize_client(
            &mix_format,
            10_000_000,
            &Direction::Capture,
            &ShareMode::Shared,
            false, // no loopback for capture devices
        );
        
        if result.is_ok() {
            if audio_client.get_audiocaptureclient().is_ok() {
                return Ok(());
            }
        }
        
        result.map_err(|_| anyhow::anyhow!("Failed to initialize capture device"))
    }
    
    pub fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>> {
        let devices = self.enumerate_loopback_devices()?;
        
        if devices.is_empty() {
            return Ok(None);
        }
        
        // Priority 1: Default render device with render loopback
        if let Some(default_render) = devices.iter().find(|d| 
            d.is_default && 
            matches!(d.device_type, DeviceType::Render) && 
            matches!(d.loopback_method, LoopbackMethod::RenderLoopback)
        ) {
            // println!("✓ Auto-selected: Default render device with loopback: {}", default_render.name); // Commented out: Audio loopback is working, reducing console noise for debugging focus
            return Ok(Some(default_render.clone()));
        }
        
        // Priority 2: Any render device with loopback
        if let Some(render_device) = devices.iter().find(|d| 
            matches!(d.device_type, DeviceType::Render) && 
            matches!(d.loopback_method, LoopbackMethod::RenderLoopback)
        ) {
            // println!("✓ Auto-selected: Render device with loopback: {}", render_device.name); // Commented out: Audio loopback is working, reducing console noise for debugging focus
            return Ok(Some(render_device.clone()));
        }
        
        // Priority 3: Stereo Mix device
        if let Some(stereo_mix) = devices.iter().find(|d| 
            matches!(d.loopback_method, LoopbackMethod::StereoMix)
        ) {
            // println!("✓ Auto-selected: Stereo Mix device: {}", stereo_mix.name); // Commented out: Audio loopback is working, reducing console noise for debugging focus
            return Ok(Some(stereo_mix.clone()));
        }
        
        // Priority 4: Any capture device that might work
        if let Some(capture_device) = devices.iter().find(|d| 
            matches!(d.device_type, DeviceType::Capture)
        ) {
            // println!("✓ Auto-selected: Capture device: {}", capture_device.name); // Commented out: Audio loopback is working, reducing console noise for debugging focus
            return Ok(Some(capture_device.clone()));
        }
        
        // Fallback: First available device
        // println!("⚠ No optimal device found, using first available: {}", devices[0].name); // Commented out: Audio loopback is working, reducing console noise for debugging focus
        Ok(Some(devices[0].clone()))
    }
    
    pub fn find_device_by_id(&self, device_id: &str) -> Result<Option<AudioLoopbackDevice>> {
        let devices = self.enumerate_loopback_devices()?;
        Ok(devices.into_iter().find(|d| d.id == device_id))
    }
}

// Tauri Commands
#[tauri::command]
pub async fn enumerate_loopback_devices() -> Result<Vec<AudioLoopbackDevice>, String> {
    match WASAPILoopbackEnumerator::new() {
        Ok(enumerator) => {
            match enumerator.enumerate_loopback_devices() {
                Ok(devices) => Ok(devices),
                Err(e) => Err(format!("Failed to enumerate audio devices: {}", e))
            }
        },
        Err(e) => Err(format!("Failed to initialize audio enumerator: {}", e))
    }
}

#[tauri::command]
pub async fn auto_select_best_device() -> Result<Option<AudioLoopbackDevice>, String> {
    match WASAPILoopbackEnumerator::new() {
        Ok(enumerator) => {
            match enumerator.auto_select_best_device() {
                Ok(device) => Ok(device),
                Err(e) => Err(format!("Failed to auto-select device: {}", e))
            }
        },
        Err(e) => Err(format!("Failed to initialize audio enumerator: {}", e))
    }
}

#[tauri::command]
pub async fn test_audio_device(device_id: String) -> Result<bool, String> {
    match WASAPILoopbackEnumerator::new() {
        Ok(enumerator) => {
            match enumerator.find_device_by_id(&device_id) {
                Ok(Some(device_info)) => {
                    let result = match device_info.device_type {
                        DeviceType::Render => {
                            if let Ok(device_collection) = DeviceCollection::new(&Direction::Render) {
                                let device_count = device_collection.get_nbr_devices().unwrap_or(0);
                                for i in 0..device_count {
                                    if let Ok(device) = device_collection.get_device_at_index(i) {
                                        if let Ok(id) = device.get_id() {
                                            if id == device_id {
                                                return Ok(enumerator.test_render_loopback_capability(&device));
                                            }
                                        }
                                    }
                                }
                            }
                            false
                        },
                        DeviceType::Capture => {
                            if let Ok(device_collection) = DeviceCollection::new(&Direction::Capture) {
                                let device_count = device_collection.get_nbr_devices().unwrap_or(0);
                                for i in 0..device_count {
                                    if let Ok(device) = device_collection.get_device_at_index(i) {
                                        if let Ok(id) = device.get_id() {
                                            if id == device_id {
                                                return Ok(enumerator.test_capture_device_capability(&device));
                                            }
                                        }
                                    }
                                }
                            }
                            false
                        }
                    };
                    
                    Ok(result)
                },
                Ok(None) => Ok(false),
                Err(e) => Err(format!("Failed to find device: {}", e))
            }
        },
        Err(e) => Err(format!("Failed to test audio device: {}", e))
    }
}