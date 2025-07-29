use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub driver_version: Option<String>,
    pub memory_total: Option<u64>, // in MB
    pub memory_used: Option<u64>, // in MB
    pub temperature: Option<f32>, // in Celsius
    pub utilization: Option<f32>, // percentage
    pub vendor: GpuVendor,
    pub pci_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuStats {
    pub gpus: Vec<GpuInfo>,
    pub last_updated: u64, // timestamp
}

impl Default for GpuStats {
    fn default() -> Self {
        Self {
            gpus: Vec::new(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[command]
pub async fn get_gpu_info() -> Result<GpuStats, String> {
    detect_gpus().await
}

#[command]
pub async fn get_gpu_utilization() -> Result<Vec<GpuInfo>, String> {
    let stats = detect_gpus().await?;
    Ok(stats.gpus)
}

async fn detect_gpus() -> Result<GpuStats, String> {
    let mut gpus = Vec::new();
    
    // Try different detection methods based on platform
    #[cfg(target_os = "windows")]
    {
        gpus.extend(detect_windows_gpus().await?);
    }
    
    #[cfg(target_os = "macos")]
    {
        gpus.extend(detect_macos_gpus().await?);
    }
    
    #[cfg(target_os = "linux")]
    {
        gpus.extend(detect_linux_gpus().await?);
    }
    
    // If no platform-specific detection worked, try generic methods
    if gpus.is_empty() {
        gpus.extend(detect_generic_gpus().await?);
    }
    
    Ok(GpuStats {
        gpus,
        last_updated: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    })
}

#[cfg(target_os = "windows")]
async fn detect_windows_gpus() -> Result<Vec<GpuInfo>, String> {
    let mut gpus = Vec::new();
    
    // Try nvidia-smi first
    if let Ok(nvidia_gpus) = detect_nvidia_gpus().await {
        gpus.extend(nvidia_gpus);
    }
    
    // Try PowerShell for general GPU info
    let powershell_output = Command::new("powershell")
        .args(&[
            "-Command",
            "Get-WmiObject Win32_VideoController | Select-Object Name, DriverVersion, AdapterRAM, PNPDeviceID | ConvertTo-Json"
        ])
        .output();
    
    if let Ok(output) = powershell_output {
        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(wmi_data) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(array) = wmi_data.as_array() {
                    for gpu_data in array {
                        if let Some(gpu) = parse_windows_gpu_data(gpu_data) {
                            // Avoid duplicates from nvidia-smi
                            if !gpus.iter().any(|existing| existing.name == gpu.name) {
                                gpus.push(gpu);
                            }
                        }
                    }
                } else if let Some(gpu) = parse_windows_gpu_data(&wmi_data) {
                    if !gpus.iter().any(|existing| existing.name == gpu.name) {
                        gpus.push(gpu);
                    }
                }
            }
        }
    }
    
    Ok(gpus)
}

#[cfg(target_os = "macos")]
async fn detect_macos_gpus() -> Result<Vec<GpuInfo>, String> {
    let mut gpus = Vec::new();
    
    // Try system_profiler for macOS
    let output = Command::new("system_profiler")
        .args(&["SPDisplaysDataType", "-json"])
        .output()
        .map_err(|e| format!("Failed to run system_profiler: {}", e))?;
    
    if output.status.success() {
        let json_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let Some(displays) = data["SPDisplaysDataType"].as_array() {
                for display in displays {
                    if let Some(gpu) = parse_macos_gpu_data(display) {
                        gpus.push(gpu);
                    }
                }
            }
        }
    }
    
    Ok(gpus)
}

#[cfg(target_os = "linux")]
async fn detect_linux_gpus() -> Result<Vec<GpuInfo>, String> {
    let mut gpus = Vec::new();
    
    // Try nvidia-smi first
    if let Ok(nvidia_gpus) = detect_nvidia_gpus().await {
        gpus.extend(nvidia_gpus);
    }
    
    // Try lspci for general GPU detection
    let lspci_output = Command::new("lspci")
        .args(&["-nn", "-k"])
        .output();
    
    if let Ok(output) = lspci_output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            gpus.extend(parse_lspci_output(&output_str));
        }
    }
    
    Ok(gpus)
}

// Cross-platform NVIDIA detection using nvidia-smi
async fn detect_nvidia_gpus() -> Result<Vec<GpuInfo>, String> {
    let mut gpus = Vec::new();
    
    let output = Command::new("nvidia-smi")
        .args(&[
            "--query-gpu=name,driver_version,memory.total,memory.used,temperature.gpu,utilization.gpu,pci.bus_id",
            "--format=csv,noheader,nounits"
        ])
        .output();
    
    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if let Some(gpu) = parse_nvidia_smi_line(line) {
                    gpus.push(gpu);
                }
            }
        }
    }
    
    Ok(gpus)
}

async fn detect_generic_gpus() -> Result<Vec<GpuInfo>, String> {
    // Fallback detection for when platform-specific methods fail
    Ok(vec![GpuInfo {
        name: "Generic Graphics Device".to_string(),
        driver_version: None,
        memory_total: None,
        memory_used: None,
        temperature: None,
        utilization: None,
        vendor: GpuVendor::Unknown,
        pci_id: None,
    }])
}

fn parse_nvidia_smi_line(line: &str) -> Option<GpuInfo> {
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() >= 7 {
        Some(GpuInfo {
            name: parts[0].to_string(),
            driver_version: Some(parts[1].to_string()),
            memory_total: parts[2].parse().ok(),
            memory_used: parts[3].parse().ok(),
            temperature: parts[4].parse().ok(),
            utilization: parts[5].parse().ok(),
            vendor: GpuVendor::Nvidia,
            pci_id: Some(parts[6].to_string()),
        })
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn parse_windows_gpu_data(data: &serde_json::Value) -> Option<GpuInfo> {
    let name = data["Name"].as_str()?.to_string();
    let driver_version = data["DriverVersion"].as_str().map(|s| s.to_string());
    let memory_total = data["AdapterRAM"]
        .as_u64()
        .map(|bytes| bytes / (1024 * 1024)); // Convert to MB
    let pci_id = data["PNPDeviceID"].as_str().map(|s| s.to_string());
    
    let vendor = if name.to_lowercase().contains("nvidia") {
        GpuVendor::Nvidia
    } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
        GpuVendor::Amd
    } else if name.to_lowercase().contains("intel") {
        GpuVendor::Intel
    } else {
        GpuVendor::Unknown
    };
    
    Some(GpuInfo {
        name,
        driver_version,
        memory_total,
        memory_used: None,
        temperature: None,
        utilization: None,
        vendor,
        pci_id,
    })
}

#[cfg(target_os = "macos")]
fn parse_macos_gpu_data(data: &serde_json::Value) -> Option<GpuInfo> {
    let name = data["sppci_model"].as_str()?.to_string();
    let memory_str = data["spdisplays_vram"].as_str().unwrap_or("0 MB");
    let memory_total = extract_memory_from_string(memory_str);
    
    let vendor = if name.to_lowercase().contains("nvidia") {
        GpuVendor::Nvidia
    } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
        GpuVendor::Amd
    } else if name.to_lowercase().contains("intel") {
        GpuVendor::Intel
    } else {
        GpuVendor::Unknown
    };
    
    Some(GpuInfo {
        name,
        driver_version: None,
        memory_total,
        memory_used: None,
        temperature: None,
        utilization: None,
        vendor,
        pci_id: None,
    })
}

#[cfg(target_os = "linux")]
fn parse_lspci_output(output: &str) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    
    for line in output.lines() {
        if line.to_lowercase().contains("vga compatible controller") 
            || line.to_lowercase().contains("3d controller") 
            || line.to_lowercase().contains("display controller") {
            
            if let Some(gpu) = parse_lspci_line(line) {
                gpus.push(gpu);
            }
        }
    }
    
    gpus
}

#[cfg(target_os = "linux")]
fn parse_lspci_line(line: &str) -> Option<GpuInfo> {
    // Parse lspci line format: "00:02.0 VGA compatible controller: Intel Corporation ..."
    let parts: Vec<&str> = line.splitn(2, ": ").collect();
    if parts.len() < 2 {
        return None;
    }
    
    let name = parts[1].to_string();
    let vendor = if name.to_lowercase().contains("nvidia") {
        GpuVendor::Nvidia
    } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
        GpuVendor::Amd
    } else if name.to_lowercase().contains("intel") {
        GpuVendor::Intel
    } else {
        GpuVendor::Unknown
    };
    
    Some(GpuInfo {
        name,
        driver_version: None,
        memory_total: None,
        memory_used: None,
        temperature: None,
        utilization: None,
        vendor,
        pci_id: Some(parts[0].split_whitespace().next()?.to_string()),
    })
}

fn extract_memory_from_string(memory_str: &str) -> Option<u64> {
    let parts: Vec<&str> = memory_str.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(value) = parts[0].parse::<u64>() {
            match parts[1].to_lowercase().as_str() {
                "mb" => Some(value),
                "gb" => Some(value * 1024),
                "kb" => Some(value / 1024),
                _ => Some(value),
            }
        } else {
            None
        }
    } else {
        None
    }
}