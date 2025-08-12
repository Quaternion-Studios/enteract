use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use std::collections::HashMap;
use std::sync::Arc;
// use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use tokio::sync::Semaphore;
// use tokio::time::timeout; 
use std::sync::Mutex;
use crate::system_prompts::{
    ENTERACT_AGENT_PROMPT, 
    VISION_ANALYSIS_PROMPT, 
    DEEP_RESEARCH_PROMPT, 
    CONVERSATIONAL_AI_PROMPT,
    CODING_AGENT_PROMPT
};
use crate::system_info::get_gpu_info;
// Shared HTTP client for better connection pooling and memory efficiency
lazy_static! {
    static ref HTTP_CLIENT: Arc<reqwest::Client> = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(16)  // More idle connections for faster reuse
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
            .timeout(std::time::Duration::from_secs(60))  // Shorter timeout to fail fast
            .build()
            .expect("Failed to create HTTP client")
    );
    
    // Semaphore to limit concurrent AI model requests (memory safety)
    static ref REQUEST_SEMAPHORE: Arc<Semaphore> = Arc::new(Semaphore::new(4)); // Slightly higher concurrency
    
    // Track active streaming sessions for cancellation
    static ref ACTIVE_SESSIONS: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelDetails {
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub parameter_size: String,
    pub quantization_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelsResponse {
    pub models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaStatus {
    pub status: String,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub name: String,
    pub insecure: Option<bool>,
    pub stream: Option<bool>,
}

// Chat context structures for frontend communication
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatContextMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    pub stream: Option<bool>,
    pub context: Option<Vec<i32>>,
    pub images: Option<Vec<String>>,
    pub system: Option<String>,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<i32>>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u64>,
}

const OLLAMA_BASE_URL: &str = "http://localhost:11434";



// Helper function to build prompt with chat context
fn build_prompt_with_context(current_prompt: String, context: Option<Vec<ChatContextMessage>>) -> String {
    match context {
        Some(messages) if !messages.is_empty() => {
            let mut full_prompt = String::new();
            full_prompt.push_str("## Conversation History:\n\n");
            
            for message in &messages {
                match message.role.as_str() {
                    "user" => full_prompt.push_str(&format!("**User:** {}\n\n", message.content)),
                    "assistant" => full_prompt.push_str(&format!("**Assistant:** {}\n\n", message.content)),
                    "system" => full_prompt.push_str(&format!("**System:** {}\n\n", message.content)),
                    _ => full_prompt.push_str(&format!("**{}:** {}\n\n", message.role, message.content)),
                }
            }
            
            full_prompt.push_str("## Current Request:\n\n");
            full_prompt.push_str(&current_prompt);
            
            println!("📊 Built prompt with {} context messages, total length: {} chars", messages.len(), full_prompt.len());
            full_prompt
        }
        _ => {
            println!("📊 No context provided, using prompt as-is");
            current_prompt
        }
    }
}

// Detect GPU and determine optimal layer count for GPU acceleration
fn detect_gpu_layers() -> i32 {
    // Try to get GPU info
    match get_gpu_info() {
        Ok(gpus) => {
            for gpu in gpus {
                // Check for NVIDIA GPUs (best Ollama support)
                if gpu.vendor == "NVIDIA" {
                    if let Some(memory_mb) = gpu.memory_mb {
                        println!("🎮 Detected NVIDIA GPU: {} with {}MB VRAM", gpu.name, memory_mb);
                        
                        // Calculate layers based on VRAM
                        // Conservative estimates to prevent OOM
                        let layers = if memory_mb >= 24000 {
                            99  // Full GPU offload for 24GB+ cards (RTX 4090, A5000)
                        } else if memory_mb >= 16000 {
                            80  // RTX 4080, A4000
                        } else if memory_mb >= 12000 {
                            60  // RTX 4070 Ti, RTX 3080 Ti
                        } else if memory_mb >= 10000 {
                            50  // RTX 3080, RTX 4070
                        } else if memory_mb >= 8000 {
                            40  // RTX 3070, RTX 4060 Ti
                        } else if memory_mb >= 6000 {
                            30  // RTX 3060, RTX 4060
                        } else if memory_mb >= 4000 {
                            20  // GTX 1650, older cards
                        } else {
                            0   // Too little VRAM, use CPU
                        };
                        
                        println!("🚀 GPU acceleration enabled with {} layers", layers);
                        return layers;
                    }
                }
                
                // AMD GPUs (experimental Ollama support)
                if gpu.vendor == "AMD" && gpu.name.contains("Radeon") {
                    if let Some(memory_mb) = gpu.memory_mb {
                        println!("🎮 Detected AMD GPU: {} with {}MB VRAM", gpu.name, memory_mb);
                        
                        // Conservative for AMD due to less mature support
                        let layers = if memory_mb >= 16000 {
                            40
                        } else if memory_mb >= 8000 {
                            20
                        } else {
                            0
                        };
                        
                        if layers > 0 {
                            println!("⚠️ AMD GPU support is experimental, using {} layers", layers);
                        }
                        return layers;
                    }
                }
            }
            
            println!("⚠️ No supported GPU found for acceleration, using CPU");
            0
        }
        Err(e) => {
            println!("⚠️ Could not detect GPU: {}, using CPU", e);
            0
        }
    }
}

// Get GPU acceleration status
#[tauri::command]
pub fn get_gpu_acceleration_status() -> serde_json::Value {
    let gpu_layers = detect_gpu_layers();
    let gpus = get_gpu_info().unwrap_or_else(|_| vec![]);
    
    serde_json::json!({
        "enabled": gpu_layers > 0,
        "layers": gpu_layers,
        "gpus": gpus.iter().map(|gpu| {
            serde_json::json!({
                "name": gpu.name,
                "vendor": gpu.vendor,
                "memory_mb": gpu.memory_mb,
                "driver_version": gpu.driver_version
            })
        }).collect::<Vec<_>>()
    })
}

// Cancel a streaming session
#[tauri::command]
pub fn cancel_ai_response(session_id: String) -> Result<(), String> {
    let mut sessions = ACTIVE_SESSIONS.lock().unwrap();
    sessions.insert(session_id.clone(), true);
    println!("🛑 Cancellation requested for session: {}", session_id);
    Ok(())
}

// Check if a session is cancelled
fn is_session_cancelled(session_id: &str) -> bool {
    let sessions = ACTIVE_SESSIONS.lock().unwrap();
    sessions.get(session_id).copied().unwrap_or(false)
}

// Clean up cancelled session
fn cleanup_session(session_id: &str) {
    let mut sessions = ACTIVE_SESSIONS.lock().unwrap();
    sessions.remove(session_id);
}

// Shared streaming logic
async fn stream_ollama_response(
    app_handle: AppHandle,
    url: String,
    request: GenerateRequest,
    session_id: String,
) -> Result<(), String> {
    // Register the session as active
    {
        let mut sessions = ACTIVE_SESSIONS.lock().unwrap();
        sessions.insert(session_id.clone(), false);
    }

    let client = Arc::clone(&HTTP_CLIENT);
    // Send with shorter connect timeout by spawning and imposing a small timeout for first bytes
    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let mut stream = response.bytes_stream();
                let mut buffer = Vec::new();
                // Emit a tiny nudge to UI so it can render quickly even before first chunk
                if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                    "type": "chunk",
                    "text": "",
                    "done": false
                })) {
                    eprintln!("Failed to emit priming chunk: {}", e);
                }

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            buffer.extend_from_slice(&chunk);

                            // Process complete lines from buffer
                            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                                // Check for cancellation
                                if is_session_cancelled(&session_id) {
                                    println!("🛑 Session cancelled: {}", session_id);

                                    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                        "type": "cancelled",
                                        "message": "Response cancelled by user"
                                    })) {
                                        eprintln!("Failed to emit cancellation event: {}", e);
                                    }

                                    cleanup_session(&session_id);
                                    return Ok(());
                                }

                                let line = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                                let line_str = String::from_utf8_lossy(&line[..line.len()-1]);

                                if line_str.trim().is_empty() {
                                    continue;
                                }

                                match serde_json::from_str::<GenerateResponse>(&line_str) {
                                    Ok(response_chunk) => {
                                        // Skip empty chunks to reduce UI overhead
                                        if response_chunk.response.is_empty() {
                                            continue;
                                        }
                                        if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                            "type": "chunk",
                                            "text": response_chunk.response,
                                            "done": response_chunk.done
                                        })) {
                                            eprintln!("Failed to emit chunk event: {}", e);
                                        }

                                        if response_chunk.done {
                                            println!("✅ Agent streaming completed for session: {}", session_id);
                                            cleanup_session(&session_id);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to parse streaming response: {} - Line: {}", e, line_str);
                                        continue;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Stream error: {}", e);
                            eprintln!("{}", error_msg);

                            if let Err(emit_err) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                "type": "error",
                                "error": error_msg
                            })) {
                                eprintln!("Failed to emit error event: {}", emit_err);
                            }

                            return Err(error_msg);
                        }
                    }
                }

                // Emit completion event
                if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                    "type": "complete"
                })) {
                    eprintln!("Failed to emit complete event: {}", e);
                }

                Ok(())
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                let error_msg = format!("Generation failed: {}", error_text);

                if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                    "type": "error",
                    "error": error_msg
                })) {
                    eprintln!("Failed to emit error event: {}", e);
                }

                Err(error_msg)
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to connect to Ollama: {}", e);

            if let Err(emit_err) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                "type": "error",
                "error": error_msg
            })) {
                eprintln!("Failed to emit error event: {}", emit_err);
            }

            Err(error_msg)
        }
    }
}




#[tauri::command]
pub async fn get_ollama_models() -> Result<Vec<OllamaModel>, String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/tags", OLLAMA_BASE_URL);
    
    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<OllamaModelsResponse>().await {
                    Ok(models_response) => Ok(models_response.models),
                    Err(e) => Err(format!("Failed to parse models response: {}", e)),
                }
            } else {
                Err(format!("Ollama API error: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to connect to Ollama: {}. Make sure Ollama is running.", e)),
    }
}

#[tauri::command]
pub async fn get_ollama_status() -> Result<OllamaStatus, String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/version", OLLAMA_BASE_URL);
    
    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<HashMap<String, String>>().await {
                    Ok(version_info) => Ok(OllamaStatus {
                        status: "running".to_string(),
                        version: version_info.get("version").cloned(),
                    }),
                    Err(_) => Ok(OllamaStatus {
                        status: "running".to_string(),
                        version: None,
                    }),
                }
            } else {
                Err(format!("Ollama API error: {}", response.status()))
            }
        }
        Err(_) => Ok(OllamaStatus {
            status: "not_running".to_string(),
            version: None,
        }),
    }
}

#[tauri::command]
pub async fn pull_ollama_model(model_name: String) -> Result<String, String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/pull", OLLAMA_BASE_URL);
    
    let request = PullRequest {
        name: model_name.clone(),
        insecure: Some(false),
        stream: Some(false),
    };
    
    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                Ok(format!("Successfully started pulling model: {}", model_name))
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to pull model: {}", error_text))
            }
        }
        Err(e) => Err(format!("Failed to connect to Ollama: {}", e)),
    }
}

#[tauri::command]
pub async fn delete_ollama_model(model_name: String) -> Result<String, String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/delete", OLLAMA_BASE_URL);
    
    let request = serde_json::json!({
        "name": model_name
    });
    
    match client.delete(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                Ok(format!("Successfully deleted model: {}", model_name))
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to delete model: {}", error_text))
            }
        }
        Err(e) => Err(format!("Failed to connect to Ollama: {}", e)),
    }
}

#[tauri::command]
pub async fn generate_ollama_response(model: String, prompt: String) -> Result<String, String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/generate", OLLAMA_BASE_URL);
    
    // Detect GPU and set acceleration options
    let gpu_layers = detect_gpu_layers();
    let options = if gpu_layers > 0 {
        Some(serde_json::json!({
            "num_gpu": gpu_layers,
            "num_thread": 4
        }))
    } else {
        None
    };
    
    let request = GenerateRequest {
        model,
        prompt,
        stream: Some(false),
        context: None,
        images: None,
        system: None,
        options,
    };
    
    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<GenerateResponse>().await {
                    Ok(generate_response) => Ok(generate_response.response),
                    Err(e) => Err(format!("Failed to parse response: {}", e)),
                }
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Generation failed: {}", error_text))
            }
        }
        Err(e) => Err(format!("Failed to connect to Ollama: {}", e)),
    }
}

#[tauri::command]
pub async fn generate_ollama_response_stream(
    app_handle: AppHandle,
    model: String,
    prompt: String,
    session_id: String,
) -> Result<(), String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/generate", OLLAMA_BASE_URL);
    
    // Detect GPU and set acceleration options
    let gpu_layers = detect_gpu_layers();
    let options = if gpu_layers > 0 {
        Some(serde_json::json!({
            "num_gpu": gpu_layers,
            "num_thread": 4
        }))
    } else {
        None
    };
    
    let request = GenerateRequest {
        model: model.clone(),
        prompt: prompt.clone(),
        stream: Some(true),
        context: None,
        images: None,
        system: None,
        options,
    };
    
    println!("🚀 Starting streaming generation for session: {}", session_id);
    
    // Emit start event
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "start",
        "model": model,
        "prompt": prompt
    })) {
        return Err(format!("Failed to emit start event: {}", e));
    }
    
    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let mut stream = response.bytes_stream();
                let mut buffer = Vec::new();
                
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            buffer.extend_from_slice(&chunk);
                            
                            // Process complete lines from buffer
                            while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                                let line = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                                let line_str = String::from_utf8_lossy(&line[..line.len()-1]); // Remove newline
                                
                                if line_str.trim().is_empty() {
                                    continue;
                                }
                                
                                // Parse JSON response
                                match serde_json::from_str::<GenerateResponse>(&line_str) {
                                    Ok(response_chunk) => {
                                        // Emit chunk event
                                        if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                            "type": "chunk",
                                            "text": response_chunk.response,
                                            "done": response_chunk.done
                                        })) {
                                            eprintln!("Failed to emit chunk event: {}", e);
                                        }
                                        
                                        // If done, break the loop
                                        if response_chunk.done {
                                            println!("✅ Streaming completed for session: {}", session_id);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to parse streaming response: {} - Line: {}", e, line_str);
                                        continue;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Stream error: {}", e);
                            eprintln!("{}", error_msg);
                            
                            // Emit error event
                            if let Err(emit_err) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                "type": "error",
                                "error": error_msg
                            })) {
                                eprintln!("Failed to emit error event: {}", emit_err);
                            }
                            
                            return Err(error_msg);
                        }
                    }
                }
                
                // Emit completion event
                if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                    "type": "complete"
                })) {
                    eprintln!("Failed to emit complete event: {}", e);
                }
                
                Ok(())
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                let error_msg = format!("Generation failed: {}", error_text);
                
                // Emit error event
                if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                    "type": "error",
                    "error": error_msg
                })) {
                    eprintln!("Failed to emit error event: {}", e);
                }
                
                Err(error_msg)
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to connect to Ollama: {}", e);
            
            // Emit error event
            if let Err(emit_err) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                "type": "error",
                "error": error_msg
            })) {
                eprintln!("Failed to emit error event: {}", emit_err);
            }
            
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn generate_enteract_agent_response(
    app_handle: AppHandle,
    prompt: String,
    context: Option<Vec<ChatContextMessage>>,
    session_id: String,
) -> Result<(), String> {
    let model = "gemma3:1b-it-qat".to_string();
    generate_agent_response_stream(app_handle, model, prompt, ENTERACT_AGENT_PROMPT.to_string(), context, session_id, "enteract".to_string()).await
}

#[tauri::command]
pub async fn generate_vision_analysis(
    app_handle: AppHandle,
    prompt: String,
    image_base64: String,
    session_id: String,
) -> Result<(), String> {
    let model = "qwen2.5vl:3b".to_string();
    let full_prompt = format!("Screenshot Analysis Request:\n\n{}", prompt);
    
    generate_agent_response_stream_with_image(
        app_handle, 
        model, 
        full_prompt, 
        VISION_ANALYSIS_PROMPT.to_string(),
        image_base64,
        None, // Vision analysis doesn't use chat context
        session_id,
        "vision".to_string()
    ).await
}

#[tauri::command]
pub async fn generate_coding_agent_response(
    app_handle: AppHandle,
    prompt: String,
    context: Option<Vec<ChatContextMessage>>,
    session_id: String,
) -> Result<(), String> {
    let model = "qwen2.5-coder:1.5b".to_string();
    let full_prompt = format!("Coding Request:\n\n{}", prompt);
    
    println!("💻 CODING AGENT: Using model {} for session {}", model, session_id);
    generate_agent_response_stream(app_handle, model, full_prompt, CODING_AGENT_PROMPT.to_string(), context, session_id, "coding".to_string()).await
}

#[tauri::command]
pub async fn generate_deep_research(
    app_handle: AppHandle,
    prompt: String,
    context: Option<Vec<ChatContextMessage>>,
    session_id: String,
) -> Result<(), String> {
    let model = "deepseek-r1:1.5b".to_string();
    let full_prompt = format!("Deep Research Query:\n\n{}", prompt);
    
    println!("🧠 DEEP RESEARCH: Using model {} for session {}", model, session_id);
    generate_agent_response_stream(app_handle, model, full_prompt, DEEP_RESEARCH_PROMPT.to_string(), context, session_id, "research".to_string()).await
}

#[tauri::command]
pub async fn generate_conversational_ai(
    app_handle: AppHandle,
    conversation_context: String,
    session_id: String,
    custom_system_prompt: Option<String>,
) -> Result<(), String> {
    // Fast 1B model for instant responses (quantized)
    let model = "gemma3:1b-it-qat".to_string();
    
    // Simplified prompt - just provide the conversation context
    let full_prompt = format!("Conversation:\n{}\n\nProvide a brief summary and helpful next steps.", conversation_context);
    
    // Always use the simplified system prompt
    let system_prompt = CONVERSATIONAL_AI_PROMPT.to_string();
    
    println!("💬 CONVERSATIONAL AI: Using model {} for insights, session {}", model, session_id);
    
    generate_agent_response_stream(app_handle, model, full_prompt, system_prompt, None, session_id, "conversational_ai".to_string()).await
}


// Helper function for streaming with system prompt
async fn generate_agent_response_stream(
    app_handle: AppHandle,
    model: String,
    prompt: String,
    system_prompt: String,
    context: Option<Vec<ChatContextMessage>>,
    session_id: String,
    agent_type: String,
) -> Result<(), String> {
    // Acquire semaphore permit for memory safety (limits concurrent model loads)
    let _permit = REQUEST_SEMAPHORE.acquire().await.map_err(|e| format!("Failed to acquire semaphore: {}", e))?;
    
    println!("🔒 Acquired request semaphore for {} agent (session: {})", agent_type, session_id);
    
    let url = format!("{}/api/generate", OLLAMA_BASE_URL);
    
    // Build full prompt with context
    let full_prompt = build_prompt_with_context(prompt, context);
    
    // Detect GPU and set acceleration options
    let gpu_layers = detect_gpu_layers();
    
    let options = if agent_type == "conversational_ai" {
        // Keep live suggestions extremely fast and short
        let mut opts = serde_json::json!({
            "num_predict": 64,
            "temperature": 0.6,
            "top_p": 0.9,
            "repeat_penalty": 1.05
        });
        if gpu_layers > 0 {
            opts["num_gpu"] = serde_json::json!(gpu_layers);
            opts["num_thread"] = serde_json::json!(4); // Reduce CPU threads when using GPU
        }
        Some(opts)
    } else if agent_type == "coding" {
        let mut opts = serde_json::json!({
            "num_predict": 512,
            "temperature": 0.2,
            "top_p": 0.9,
            "repeat_penalty": 1.1
        });
        if gpu_layers > 0 {
            opts["num_gpu"] = serde_json::json!(gpu_layers);
            opts["num_thread"] = serde_json::json!(4);
        }
        Some(opts)
    } else {
        let mut opts = serde_json::json!({
            "num_predict": 256,
            "temperature": 0.7,
            "top_p": 0.9,
            "repeat_penalty": 1.1
        });
        if gpu_layers > 0 {
            opts["num_gpu"] = serde_json::json!(gpu_layers);
            opts["num_thread"] = serde_json::json!(4);
        }
        Some(opts)
    };

    let request = GenerateRequest {
        model: model.clone(),
        prompt: full_prompt,
        stream: Some(true),
        context: None,
        images: None,
        system: Some(system_prompt),
        options,
    };
    
    println!("🤖 Starting {} agent ({}) streaming for session: {}", agent_type, model, session_id);
    
    // Emit start event with correct agent type
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "start",
        "model": model,
        "agent_type": agent_type
    })) {
        return Err(format!("Failed to emit start event: {}", e));
    }
    
    let result = stream_ollama_response(app_handle, url, request, session_id.clone()).await;
    
    // Semaphore is automatically released when _permit goes out of scope
    println!("🔓 Released request semaphore for {} agent (session: {})", agent_type, session_id);
    
    result
}

// Helper function for streaming with image
async fn generate_agent_response_stream_with_image(
    app_handle: AppHandle,
    model: String,
    prompt: String,
    system_prompt: String,
    image_base64: String,
    context: Option<Vec<ChatContextMessage>>,
    session_id: String,
    agent_type: String,
) -> Result<(), String> {
    // Acquire semaphore permit for memory safety (limits concurrent model loads)
    let _permit = REQUEST_SEMAPHORE.acquire().await.map_err(|e| format!("Failed to acquire semaphore: {}", e))?;
    
    println!("🔒 Acquired request semaphore for {} agent with image (session: {})", agent_type, session_id);
    
    let url = format!("{}/api/generate", OLLAMA_BASE_URL);
    
    // Build full prompt with context (if provided)
    let full_prompt = build_prompt_with_context(prompt, context);
    
    let request = GenerateRequest {
        model: model.clone(),
        prompt: full_prompt,
        stream: Some(true),
        context: None,
        images: Some(vec![image_base64]),
        system: Some(system_prompt),
        options: {
            let gpu_layers = detect_gpu_layers();
            let mut opts = serde_json::json!({
                "num_predict": 256,
                "temperature": 0.5,
                "top_p": 0.9
            });
            if gpu_layers > 0 {
                opts["num_gpu"] = serde_json::json!(gpu_layers);
                opts["num_thread"] = serde_json::json!(4);
            }
            Some(opts)
        },
    };
    
    println!("👁️ Starting {} vision analysis ({}) for session: {}", agent_type, model, session_id);
    
    // Emit start event with correct agent type
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "start",
        "model": model,
        "agent_type": agent_type
    })) {
        return Err(format!("Failed to emit start event: {}", e));
    }
    
    let result = stream_ollama_response(app_handle, url, request, session_id.clone()).await;
    
    // Semaphore is automatically released when _permit goes out of scope
    println!("🔓 Released request semaphore for {} agent (session: {})", agent_type, session_id);
    
    result
}

#[tauri::command]
pub async fn get_ollama_model_info(model_name: String) -> Result<serde_json::Value, String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/show", OLLAMA_BASE_URL);
    
    let request = serde_json::json!({
        "name": model_name
    });
    
    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(model_info) => Ok(model_info),
                    Err(e) => Err(format!("Failed to parse model info response: {}", e)),
                }
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to get model info: {}", error_text))
            }
        }
        Err(e) => Err(format!("Failed to connect to Ollama: {}", e)),
    }
} 