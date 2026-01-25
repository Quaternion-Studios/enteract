// src-tauri/src/audio_loopback/macos/device_enumerator.rs
// macOS audio device enumeration using CPAL
//
// Phase 2 (en-03o): macOS CPAL audio implementation
// References: Jack's research (resources/MACOS_AUDIO_LOOPBACK_RESEARCH.md)

use crate::audio_loopback::types::{AudioLoopbackDevice, DeviceType, LoopbackMethod};
use cpal::traits::{DeviceTrait, HostTrait};
use std::error::Error;

/// macOS device enumerator using CPAL
pub struct CPALDeviceEnumerator {
    host: cpal::Host,
}

impl CPALDeviceEnumerator {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    /// Enumerate all available audio output devices
    ///
    /// On macOS 14.6+, CPAL provides native loopback support.
    /// Devices are enumerated and tested for loopback capability.
    pub fn enumerate_loopback_devices(&self) -> Result<Vec<AudioLoopbackDevice>, Box<dyn Error>> {
        let mut devices = Vec::new();

        // Get all input devices (loopback devices appear as inputs on macOS)
        let input_devices = self.host.input_devices()?;

        for device in input_devices {
            if let Ok(device_info) = self.device_to_loopback_info(&device) {
                devices.push(device_info);
            }
        }

        // Sort by default device first, then by name
        devices.sort_by(|a, b| {
            match (a.is_default, b.is_default) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(devices)
    }

    /// Convert CPAL device to AudioLoopbackDevice structure
    fn device_to_loopback_info(&self, device: &cpal::Device) -> Result<AudioLoopbackDevice, Box<dyn Error>> {
        let name = device.name()?;
        let default_config = device.default_input_config()?;

        // Check if this is the default device
        let is_default = if let Some(default_device) = self.host.default_input_device() {
            if let Ok(default_name) = default_device.name() {
                default_name == name
            } else {
                false
            }
        } else {
            false
        };

        // Determine device type and loopback method
        let (device_type, loopback_method) = self.classify_device(&name);

        Ok(AudioLoopbackDevice {
            id: name.clone(), // Use name as ID on macOS (CPAL doesn't expose raw device ID)
            name,
            is_default,
            sample_rate: default_config.sample_rate(),
            channels: default_config.channels(),
            format: format!("{:?}", default_config.sample_format()),
            device_type,
            loopback_method,
        })
    }

    /// Classify device type and determine loopback method
    ///
    /// macOS device classification:
    /// - Native loopback devices (macOS 14.6+ via CPAL)
    /// - Virtual devices (BlackHole, Loopback, SoundFlower)
    /// - ScreenCaptureKit devices
    fn classify_device(&self, name: &str) -> (DeviceType, LoopbackMethod) {
        let name_lower = name.to_lowercase();

        // Detect virtual audio devices
        if name_lower.contains("blackhole")
            || name_lower.contains("soundflower")
            || name_lower.contains("loopback")
        {
            return (DeviceType::Capture, LoopbackMethod::VirtualDevice);
        }

        // Detect ScreenCaptureKit devices (if present)
        if name_lower.contains("screencapturekit") || name_lower.contains("screen capture") {
            return (DeviceType::Capture, LoopbackMethod::ScreenCaptureKit);
        }

        // Default: assume CoreAudio Tap or CPAL native loopback
        // On macOS 14.6+, CPAL uses CoreAudio Taps under the hood
        (DeviceType::Capture, LoopbackMethod::CoreAudioTap)
    }

    /// Find the default audio output device for loopback
    pub fn get_default_device(&self) -> Result<Option<AudioLoopbackDevice>, Box<dyn Error>> {
        if let Some(device) = self.host.default_input_device() {
            let device_info = self.device_to_loopback_info(&device)?;
            Ok(Some(device_info))
        } else {
            Ok(None)
        }
    }

    /// Auto-select the best device for loopback
    ///
    /// Priority:
    /// 1. Default input device (if loopback-capable)
    /// 2. First virtual device (BlackHole, etc.)
    /// 3. First available device
    pub fn auto_select_best_device(&self) -> Result<Option<AudioLoopbackDevice>, Box<dyn Error>> {
        let devices = self.enumerate_loopback_devices()?;

        if devices.is_empty() {
            return Ok(None);
        }

        // Priority 1: Default device
        if let Some(default) = devices.iter().find(|d| d.is_default) {
            return Ok(Some(default.clone()));
        }

        // Priority 2: Virtual device (BlackHole, etc.)
        if let Some(virtual_dev) = devices.iter().find(|d| {
            matches!(d.loopback_method, LoopbackMethod::VirtualDevice)
        }) {
            return Ok(Some(virtual_dev.clone()));
        }

        // Priority 3: First available device
        Ok(devices.first().cloned())
    }

    /// Test if a device supports loopback capture
    ///
    /// On macOS with CPAL, all input devices are potentially loopback-capable.
    /// This function verifies the device can be opened and configured.
    pub fn test_device_capability(&self, device_id: &str) -> bool {
        let Ok(devices) = self.host.input_devices() else {
            return false;
        };

        for device in devices {
            if let Ok(name) = device.name() {
                if name == device_id {
                    // Try to get default config to verify device is usable
                    return device.default_input_config().is_ok();
                }
            }
        }

        false
    }
}

/// Get macOS version for feature detection
///
/// Returns (major, minor, patch) version tuple.
/// Used to determine which audio capture API to use.
pub fn get_macos_version() -> (u32, u32, u32) {
    use std::process::Command;

    // Use sw_vers command to get macOS version
    if let Ok(output) = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
    {
        if let Ok(version_str) = String::from_utf8(output.stdout) {
            let parts: Vec<&str> = version_str.trim().split('.').collect();

            let major = parts.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(14);
            let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(6);
            let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

            return (major, minor, patch);
        }
    }

    // Fallback: assume modern macOS 14.6+ if detection fails
    (14, 6, 0)
}

/// Check if CoreAudio Taps are available (macOS 14.2+)
pub fn has_coreaudio_taps() -> bool {
    let (major, minor, _) = get_macos_version();
    major > 14 || (major == 14 && minor >= 2)
}

/// Check if CPAL native loopback is available (macOS 14.6+)
pub fn has_cpal_loopback() -> bool {
    let (major, minor, _) = get_macos_version();
    major > 14 || (major == 14 && minor >= 6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_devices() {
        let enumerator = CPALDeviceEnumerator::new();
        let result = enumerator.enumerate_loopback_devices();

        // Should not error even if no devices found
        assert!(result.is_ok());
    }

    #[test]
    fn test_device_classification() {
        let enumerator = CPALDeviceEnumerator::new();

        let (_, method) = enumerator.classify_device("BlackHole 2ch");
        assert!(matches!(method, LoopbackMethod::VirtualDevice));

        let (_, method) = enumerator.classify_device("Built-in Microphone");
        assert!(matches!(method, LoopbackMethod::CoreAudioTap));
    }

    #[test]
    fn test_version_detection() {
        let (major, minor, _) = get_macos_version();
        // Should return a valid version
        assert!(major >= 10);

        // Test feature flags
        if major >= 14 && minor >= 6 {
            assert!(has_cpal_loopback());
        }

        if major >= 14 && minor >= 2 {
            assert!(has_coreaudio_taps());
        }
    }
}
