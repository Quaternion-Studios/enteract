// Cross-platform GPU detection and acceleration status
use serde::{Deserialize, Serialize};
use std::process::Command;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: GpuVendor,
    pub memory_gb: Option<f32>,
    pub compute_capability: Option<String>,
    pub supports_metal: bool,
    pub supports_cuda: bool,
    pub supports_opencl: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemGpuStatus {
    pub platform: String,
    pub gpus: Vec<GpuInfo>,
    pub recommended_backend: AccelerationBackend,
    pub ollama_gpu_status: OllamaGpuStatus,
    pub acceleration_available: bool,
    pub performance_notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AccelerationBackend {
    Metal,      // macOS Apple Silicon
    Cuda,       // NVIDIA GPUs
    Rocm,       // AMD GPUs
    OpenCl,     // Fallback for older hardware
    Cpu,        // No GPU acceleration
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaGpuStatus {
    pub detected_backend: String,
    pub gpu_layers: Option<u32>,
    pub memory_usage: Option<String>,
    pub supports_acceleration: bool,
}

#[tauri::command]
pub async fn detect_gpu_capabilities() -> Result<SystemGpuStatus, String> {
    println!("🔍 Detecting GPU capabilities...");
    
    let platform = std::env::consts::OS.to_string();
    let gpus = detect_platform_gpus().await?;
    let recommended_backend = determine_best_backend(&gpus);
    let ollama_status = check_ollama_gpu_support().await;
    let acceleration_available = !matches!(recommended_backend, AccelerationBackend::Cpu);
    let performance_notes = generate_performance_notes(&gpus, &recommended_backend);
    
    Ok(SystemGpuStatus {
        platform,
        gpus,
        recommended_backend,
        ollama_gpu_status: ollama_status,
        acceleration_available,
        performance_notes,
    })
}

async fn detect_platform_gpus() -> Result<Vec<GpuInfo>, String> {
    #[cfg(target_os = "macos")]
    {
        detect_macos_gpus().await
    }
    
    #[cfg(target_os = "windows")]
    {
        detect_windows_gpus().await
    }
    
    #[cfg(target_os = "linux")]
    {
        detect_linux_gpus().await
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Ok(vec![])
    }
}

#[cfg(target_os = "macos")]
async fn detect_macos_gpus() -> Result<Vec<GpuInfo>, String> {
    println!("🍎 Detecting macOS GPUs...");
    
    let mut gpus = Vec::new();
    
    // Check for Apple Silicon
    if let Ok(output) = Command::new("sysctl")
        .arg("-n")
        .arg("machdep.cpu.brand_string")
        .output()
    {
        let cpu_info = String::from_utf8_lossy(&output.stdout);
        if cpu_info.contains("Apple") {
            println!("📱 Detected Apple Silicon");
            
            // Get GPU info from system_profiler for Apple Silicon
            if let Ok(gpu_output) = Command::new("system_profiler")
                .arg("SPDisplaysDataType")
                .arg("-json")
                .output()
            {
                let gpu_data = String::from_utf8_lossy(&gpu_output.stdout);
                
                // Parse basic GPU info
                let gpu_name = if gpu_data.contains("Apple M1") {
                    "Apple M1 GPU"
                } else if gpu_data.contains("Apple M2") {
                    "Apple M2 GPU" 
                } else if gpu_data.contains("Apple M3") {
                    "Apple M3 GPU"
                } else if gpu_data.contains("Apple M4") {
                    "Apple M4 GPU"
                } else {
                    "Apple Silicon GPU"
                };
                
                // Estimate GPU memory (unified memory on Apple Silicon)
                let memory_gb = if let Ok(mem_output) = Command::new("sysctl")
                    .arg("-n")
                    .arg("hw.memsize")
                    .output()
                {
                    let mem_bytes = String::from_utf8_lossy(&mem_output.stdout)
                        .trim()
                        .parse::<u64>()
                        .unwrap_or(8_000_000_000);
                    Some((mem_bytes as f32) / 1_073_741_824.0) // Convert to GB
                } else {
                    None
                };
                
                gpus.push(GpuInfo {
                    name: gpu_name.to_string(),
                    vendor: GpuVendor::Apple,
                    memory_gb,
                    compute_capability: Some("Metal".to_string()),
                    supports_metal: true,
                    supports_cuda: false,
                    supports_opencl: true,
                });
            }
        }
    }
    
    // Check for discrete GPUs (AMD/Intel on older Macs)
    if let Ok(output) = Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .arg("-xml")
        .output()
    {
        let gpu_data = String::from_utf8_lossy(&output.stdout);
        
        if gpu_data.contains("AMD") || gpu_data.contains("Radeon") {
            gpus.push(GpuInfo {
                name: "AMD Radeon (macOS)".to_string(),
                vendor: GpuVendor::Amd,
                memory_gb: None,
                compute_capability: Some("OpenCL".to_string()),
                supports_metal: true,
                supports_cuda: false,
                supports_opencl: true,
            });
        }
        
        if gpu_data.contains("Intel") && !gpu_data.contains("Apple") {
            gpus.push(GpuInfo {
                name: "Intel Graphics (macOS)".to_string(),
                vendor: GpuVendor::Intel,
                memory_gb: None,
                compute_capability: Some("OpenCL".to_string()),
                supports_metal: true,
                supports_cuda: false,
                supports_opencl: true,
            });
        }
    }
    
    if gpus.is_empty() {
        gpus.push(GpuInfo {
            name: "Unknown macOS GPU".to_string(),
            vendor: GpuVendor::Unknown("macOS".to_string()),
            memory_gb: None,
            compute_capability: None,
            supports_metal: false,
            supports_cuda: false,
            supports_opencl: false,
        });
    }
    
    Ok(gpus)
}

#[cfg(target_os = "windows")]
async fn detect_windows_gpus() -> Result<Vec<GpuInfo>, String> {
    println!("🪟 Detecting Windows GPUs...");
    
    let mut gpus = Vec::new();
    
    // Use WMIC to get GPU information
    if let Ok(output) = Command::new("wmic")
        .args(&["path", "win32_VideoController", "get", "Name,AdapterRAM", "/format:csv"])
        .output()
    {
        let gpu_data = String::from_utf8_lossy(&output.stdout);
        
        for line in gpu_data.lines().skip(1) { // Skip header
            if line.trim().is_empty() { continue; }
            
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 3 {
                let gpu_name = parts[1].trim();
                let memory_bytes = parts[2].trim().parse::<u64>().unwrap_or(0);
                let memory_gb = if memory_bytes > 0 { 
                    Some(memory_bytes as f32 / 1_073_741_824.0) 
                } else { 
                    None 
                };
                
                if !gpu_name.is_empty() && gpu_name != "Name" {
                    let vendor = if gpu_name.to_lowercase().contains("nvidia") {
                        GpuVendor::Nvidia
                    } else if gpu_name.to_lowercase().contains("amd") || gpu_name.to_lowercase().contains("radeon") {
                        GpuVendor::Amd
                    } else if gpu_name.to_lowercase().contains("intel") {
                        GpuVendor::Intel
                    } else {
                        GpuVendor::Unknown(gpu_name.to_string())
                    };
                    
                    let supports_cuda = matches!(vendor, GpuVendor::Nvidia);
                    let compute_capability = match vendor {
                        GpuVendor::Nvidia => Some("CUDA".to_string()),
                        GpuVendor::Amd => Some("ROCm/OpenCL".to_string()),
                        GpuVendor::Intel => Some("OpenCL".to_string()),
                        _ => None,
                    };
                    
                    gpus.push(GpuInfo {
                        name: gpu_name.to_string(),
                        vendor,
                        memory_gb,
                        compute_capability,
                        supports_metal: false,
                        supports_cuda,
                        supports_opencl: true,
                    });
                }
            }
        }
    }
    
    // Check for NVIDIA-SMI for more detailed CUDA info
    if let Ok(output) = Command::new("nvidia-smi")
        .arg("--query-gpu=name,memory.total,compute_cap")
        .arg("--format=csv,noheader,nounits")
        .output()
    {
        let nvidia_data = String::from_utf8_lossy(&output.stdout);
        println!("🔥 NVIDIA-SMI detected");
        
        // Update existing NVIDIA GPUs with more detailed info
        for (i, line) in nvidia_data.lines().enumerate() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 3 {
                let gpu_name = parts[0].trim();
                let memory_mb: f32 = parts[1].trim().parse().unwrap_or(0.0);
                let compute_cap = parts[2].trim();
                
                // Find existing NVIDIA GPU or add new one
                if let Some(gpu) = gpus.iter_mut().find(|g| 
                    matches!(g.vendor, GpuVendor::Nvidia) && g.name.contains("NVIDIA")
                ) {
                    gpu.name = gpu_name.to_string();
                    gpu.memory_gb = Some(memory_mb / 1024.0);
                    gpu.compute_capability = Some(format!("CUDA {}", compute_cap));
                } else {
                    gpus.push(GpuInfo {
                        name: gpu_name.to_string(),
                        vendor: GpuVendor::Nvidia,
                        memory_gb: Some(memory_mb / 1024.0),
                        compute_capability: Some(format!("CUDA {}", compute_cap)),
                        supports_metal: false,
                        supports_cuda: true,
                        supports_opencl: true,
                    });
                }
            }
        }
    }
    
    if gpus.is_empty() {
        gpus.push(GpuInfo {
            name: "Unknown Windows GPU".to_string(),
            vendor: GpuVendor::Unknown("Windows".to_string()),
            memory_gb: None,
            compute_capability: None,
            supports_metal: false,
            supports_cuda: false,
            supports_opencl: false,
        });
    }
    
    Ok(gpus)
}

#[cfg(target_os = "linux")]
async fn detect_linux_gpus() -> Result<Vec<GpuInfo>, String> {
    println!("🐧 Detecting Linux GPUs...");
    
    let mut gpus = Vec::new();
    
    // Check lspci for GPU info
    if let Ok(output) = Command::new("lspci")
        .arg("-nn")
        .output()
    {
        let gpu_data = String::from_utf8_lossy(&output.stdout);
        
        for line in gpu_data.lines() {
            if line.to_lowercase().contains("vga") || line.to_lowercase().contains("3d") {
                let gpu_name = line.split(':').nth(2).unwrap_or("Unknown GPU").trim();
                
                let vendor = if gpu_name.to_lowercase().contains("nvidia") {
                    GpuVendor::Nvidia
                } else if gpu_name.to_lowercase().contains("amd") || gpu_name.to_lowercase().contains("radeon") {
                    GpuVendor::Amd
                } else if gpu_name.to_lowercase().contains("intel") {
                    GpuVendor::Intel
                } else {
                    GpuVendor::Unknown(gpu_name.to_string())
                };
                
                gpus.push(GpuInfo {
                    name: gpu_name.to_string(),
                    vendor,
                    memory_gb: None, // Would need additional queries
                    compute_capability: match vendor {
                        GpuVendor::Nvidia => Some("CUDA".to_string()),
                        GpuVendor::Amd => Some("ROCm".to_string()),
                        _ => Some("OpenCL".to_string()),
                    },
                    supports_metal: false,
                    supports_cuda: matches!(vendor, GpuVendor::Nvidia),
                    supports_opencl: true,
                });
            }
        }
    }
    
    if gpus.is_empty() {
        gpus.push(GpuInfo {
            name: "Unknown Linux GPU".to_string(),
            vendor: GpuVendor::Unknown("Linux".to_string()),
            memory_gb: None,
            compute_capability: None,
            supports_metal: false,
            supports_cuda: false,
            supports_opencl: false,
        });
    }
    
    Ok(gpus)
}

fn determine_best_backend(gpus: &[GpuInfo]) -> AccelerationBackend {
    // Priority order: Metal (Apple Silicon) > CUDA (NVIDIA) > ROCm (AMD) > OpenCL > CPU
    
    for gpu in gpus {
        if gpu.supports_metal && matches!(gpu.vendor, GpuVendor::Apple) {
            return AccelerationBackend::Metal;
        }
    }
    
    for gpu in gpus {
        if gpu.supports_cuda && matches!(gpu.vendor, GpuVendor::Nvidia) {
            return AccelerationBackend::Cuda;
        }
    }
    
    for gpu in gpus {
        if matches!(gpu.vendor, GpuVendor::Amd) {
            return AccelerationBackend::Rocm;
        }
    }
    
    for gpu in gpus {
        if gpu.supports_opencl {
            return AccelerationBackend::OpenCl;
        }
    }
    
    AccelerationBackend::Cpu
}

async fn check_ollama_gpu_support() -> OllamaGpuStatus {
    // Try to get Ollama's GPU status
    if let Ok(output) = Command::new("ollama")
        .arg("ps")
        .output()
    {
        let status_text = String::from_utf8_lossy(&output.stdout);
        
        // Basic parsing - in a real implementation, you might parse JSON output
        let supports_acceleration = status_text.contains("GPU") || 
                                   status_text.contains("Metal") || 
                                   status_text.contains("CUDA");
        
        let detected_backend = if status_text.contains("Metal") {
            "Metal"
        } else if status_text.contains("CUDA") {
            "CUDA"
        } else if status_text.contains("GPU") {
            "GPU"
        } else {
            "CPU"
        }.to_string();
        
        OllamaGpuStatus {
            detected_backend,
            gpu_layers: None, // Would need specific query
            memory_usage: None, // Would need specific query
            supports_acceleration,
        }
    } else {
        OllamaGpuStatus {
            detected_backend: "Unknown (Ollama not accessible)".to_string(),
            gpu_layers: None,
            memory_usage: None,
            supports_acceleration: false,
        }
    }
}

fn generate_performance_notes(gpus: &[GpuInfo], backend: &AccelerationBackend) -> Vec<String> {
    let mut notes = Vec::new();
    
    match backend {
        AccelerationBackend::Metal => {
            notes.push("🚀 Excellent: Apple Silicon GPU acceleration available via Metal".to_string());
            notes.push("💡 Tip: Unified memory architecture provides efficient GPU-CPU data sharing".to_string());
        }
        AccelerationBackend::Cuda => {
            if let Some(nvidia_gpu) = gpus.iter().find(|g| matches!(g.vendor, GpuVendor::Nvidia)) {
                if let Some(memory) = nvidia_gpu.memory_gb {
                    if memory >= 8.0 {
                        notes.push("🚀 Excellent: High-end NVIDIA GPU with sufficient VRAM".to_string());
                    } else if memory >= 4.0 {
                        notes.push("⚡ Good: NVIDIA GPU available, but limited VRAM may affect large models".to_string());
                    } else {
                        notes.push("⚠️ Limited: Low VRAM may require smaller models or CPU fallback".to_string());
                    }
                }
                notes.push("💡 Tip: Ensure CUDA drivers are installed and up to date".to_string());
            }
        }
        AccelerationBackend::Rocm => {
            notes.push("⚡ Good: AMD GPU detected - ROCm support available".to_string());
            notes.push("💡 Tip: ROCm support varies by GPU generation - check compatibility".to_string());
        }
        AccelerationBackend::OpenCl => {
            notes.push("🔧 Basic: OpenCL acceleration available but may be slower than native backends".to_string());
        }
        AccelerationBackend::Cpu => {
            notes.push("🐌 CPU-only: No GPU acceleration detected - performance may be limited".to_string());
            notes.push("💡 Tip: Consider upgrading hardware or installing GPU drivers for better performance".to_string());
        }
    }
    
    notes
}