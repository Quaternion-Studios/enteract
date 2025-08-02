use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub driver_version: Option<String>,
    pub memory_mb: Option<u64>,
    pub temperature_celsius: Option<f32>,
    pub utilization_percent: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub gpus: Vec<GpuInfo>,
    pub cpu_name: String,
    pub memory_gb: f64,
    pub os: String,
}

#[cfg(target_os = "windows")]
pub fn get_gpu_info() -> Result<Vec<GpuInfo>, String> {
    // For Windows, we'll use WMI as the primary method
    // DXGI requires additional dependencies that might not be available
    
    let mut gpus = get_gpu_info_wmi()?;
    
    // Try to get NVIDIA-specific info if available
    if let Ok(nvidia_info) = get_nvidia_info() {
        for gpu in &mut gpus {
            if gpu.vendor == "NVIDIA" {
                // Find matching NVIDIA GPU by name
                for nvidia in &nvidia_info {
                    if gpu.name.contains(&nvidia.name) || nvidia.name.contains(&gpu.name) {
                        gpu.driver_version = nvidia.driver_version.clone();
                        gpu.temperature_celsius = nvidia.temperature_celsius;
                        gpu.utilization_percent = nvidia.utilization_percent;
                        break;
                    }
                }
            }
        }
    }
    
    Ok(gpus)
}

#[cfg(target_os = "windows")]
fn get_gpu_info_wmi() -> Result<Vec<GpuInfo>, String> {
    let output = Command::new("wmic")
        .args(&["path", "win32_VideoController", "get", "name,AdapterRAM,DriverVersion", "/format:list"])
        .output()
        .map_err(|e| format!("Failed to execute wmic: {}", e))?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();
    let mut current_gpu = GpuInfo {
        name: String::new(),
        vendor: String::new(),
        driver_version: None,
        memory_mb: None,
        temperature_celsius: None,
        utilization_percent: None,
    };
    
    for line in output_str.lines() {
        if line.starts_with("Name=") {
            if !current_gpu.name.is_empty() {
                // Determine vendor from name
                current_gpu.vendor = if current_gpu.name.contains("NVIDIA") {
                    "NVIDIA".to_string()
                } else if current_gpu.name.contains("AMD") || current_gpu.name.contains("Radeon") {
                    "AMD".to_string()
                } else if current_gpu.name.contains("Intel") {
                    "Intel".to_string()
                } else {
                    "Unknown".to_string()
                };
                gpus.push(current_gpu.clone());
            }
            current_gpu.name = line.trim_start_matches("Name=").to_string();
        } else if line.starts_with("AdapterRAM=") {
            if let Ok(ram) = line.trim_start_matches("AdapterRAM=").parse::<u64>() {
                current_gpu.memory_mb = Some(ram / (1024 * 1024));
            }
        } else if line.starts_with("DriverVersion=") {
            current_gpu.driver_version = Some(line.trim_start_matches("DriverVersion=").to_string());
        }
    }
    
    // Don't forget the last GPU
    if !current_gpu.name.is_empty() {
        current_gpu.vendor = if current_gpu.name.contains("NVIDIA") {
            "NVIDIA".to_string()
        } else if current_gpu.name.contains("AMD") || current_gpu.name.contains("Radeon") {
            "AMD".to_string()
        } else if current_gpu.name.contains("Intel") {
            "Intel".to_string()
        } else {
            "Unknown".to_string()
        };
        gpus.push(current_gpu);
    }
    
    Ok(gpus)
}

#[cfg(not(target_os = "windows"))]
pub fn get_gpu_info() -> Result<Vec<GpuInfo>, String> {
    // For non-Windows platforms, try to use common tools
    let mut gpus = Vec::new();
    
    // Try nvidia-smi for NVIDIA GPUs
    if let Ok(nvidia_gpus) = get_nvidia_info() {
        gpus.extend(nvidia_gpus);
    }
    
    // Try other methods for AMD/Intel
    // This is a placeholder - would need platform-specific implementations
    
    if gpus.is_empty() {
        Err("GPU detection not implemented for this platform".to_string())
    } else {
        Ok(gpus)
    }
}

fn get_nvidia_info() -> Result<Vec<GpuInfo>, String> {
    let output = Command::new("nvidia-smi")
        .args(&["--query-gpu=name,driver_version,memory.total,temperature.gpu,utilization.gpu", "--format=csv,noheader,nounits"])
        .output()
        .map_err(|e| format!("nvidia-smi not found or failed: {}", e))?;
    
    if !output.status.success() {
        return Err("nvidia-smi command failed".to_string());
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();
    
    for line in output_str.lines() {
        let parts: Vec<&str> = line.split(", ").collect();
        if parts.len() >= 5 {
            gpus.push(GpuInfo {
                name: parts[0].to_string(),
                vendor: "NVIDIA".to_string(),
                driver_version: Some(parts[1].to_string()),
                memory_mb: parts[2].parse::<u64>().ok(),
                temperature_celsius: parts[3].parse::<f32>().ok(),
                utilization_percent: parts[4].parse::<u8>().ok(),
            });
        }
    }
    
    Ok(gpus)
}

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    let gpus = get_gpu_info().unwrap_or_else(|_| vec![]);
    
    // Get CPU info
    let cpu_name = if cfg!(target_os = "windows") {
        Command::new("wmic")
            .args(&["cpu", "get", "name", "/value"])
            .output()
            .ok()
            .and_then(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .find(|line| line.starts_with("Name="))
                    .map(|line| line.trim_start_matches("Name=").trim().to_string())
            })
            .unwrap_or_else(|| "Unknown CPU".to_string())
    } else {
        "Unknown CPU".to_string()
    };
    
    // Get memory info
    let memory_gb = if cfg!(target_os = "windows") {
        Command::new("wmic")
            .args(&["computersystem", "get", "TotalPhysicalMemory", "/value"])
            .output()
            .ok()
            .and_then(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .find(|line| line.starts_with("TotalPhysicalMemory="))
                    .and_then(|line| {
                        line.trim_start_matches("TotalPhysicalMemory=")
                            .trim()
                            .parse::<u64>()
                            .ok()
                            .map(|bytes| (bytes as f64) / (1024.0 * 1024.0 * 1024.0))
                    })
            })
            .unwrap_or(0.0)
    } else {
        0.0
    };
    
    let os = std::env::consts::OS.to_string();
    
    Ok(SystemInfo {
        gpus,
        cpu_name,
        memory_gb,
        os,
    })
}