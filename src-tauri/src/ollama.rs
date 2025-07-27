use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use tokio::sync::Semaphore;
use crate::system_prompts::{
    ENTERACT_AGENT_PROMPT, 
    VISION_ANALYSIS_PROMPT, 
    DEEP_RESEARCH_PROMPT, 
    CONVERSATIONAL_AI_PROMPT,
    CODING_AGENT_PROMPT
};

// Shared HTTP client for better connection pooling and memory efficiency
lazy_static! {
    static ref HTTP_CLIENT: Arc<reqwest::Client> = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(8)  // Maintain connection pool per host
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .timeout(std::time::Duration::from_secs(120))  // 2 minute timeout for large models
            .build()
            .expect("Failed to create HTTP client")
    );
    
    // Semaphore to limit concurrent AI model requests (memory safety)
    static ref REQUEST_SEMAPHORE: Arc<Semaphore> = Arc::new(Semaphore::new(3)); // Max 3 concurrent requests
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
    pub gpu_info: Option<GpuInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuInfo {
    pub gpu_available: bool,
    pub gpu_type: Option<String>,
    pub gpu_memory: Option<u64>,
    pub gpu_compute_capability: Option<String>,
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

// Shared streaming logic
async fn stream_ollama_response(
    app_handle: AppHandle,
    url: String,
    request: GenerateRequest,
    session_id: String,
) -> Result<(), String> {
    let client = Arc::clone(&HTTP_CLIENT);
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
                                let line_str = String::from_utf8_lossy(&line[..line.len()-1]);
                                
                                if line_str.trim().is_empty() {
                                    continue;
                                }
                                
                                match serde_json::from_str::<GenerateResponse>(&line_str) {
                                    Ok(response_chunk) => {
                                        if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                            "type": "chunk",
                                            "text": response_chunk.response,
                                            "done": response_chunk.done
                                        })) {
                                            eprintln!("Failed to emit chunk event: {}", e);
                                        }
                                        
                                        if response_chunk.done {
                                            println!("✅ Agent streaming completed for session: {}", session_id);
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
                        gpu_info: None,
                    }),
                    Err(_) => Ok(OllamaStatus {
                        status: "running".to_string(),
                        version: None,
                        gpu_info: None,
                    }),
                }
            } else {
                Err(format!("Ollama API error: {}", response.status()))
            }
        }
        Err(_) => Ok(OllamaStatus {
            status: "not_running".to_string(),
            version: None,
            gpu_info: None,
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
    
    let request = GenerateRequest {
        model,
        prompt,
        stream: Some(false),
        context: None,
        images: None,
        system: None,
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
    
    let request = GenerateRequest {
        model: model.clone(),
        prompt: prompt.clone(),
        stream: Some(true),
        context: None,
        images: None,
        system: None,
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
) -> Result<(), String> {
    let model = "gemma3:1b-it-qat".to_string(); // Using same model as enteract agent for consistency
    
    // Format the prompt to include the conversation context for live analysis
    let full_prompt = format!("LIVE CONVERSATION CONTEXT:\n{}\n\nAnalyze this ongoing conversation and suggest a thoughtful response or contribution that would add value to the discussion. Provide 1-2 concise response options that match the conversation's tone and advance the dialogue.", conversation_context);
    
    println!("💬 CONVERSATIONAL AI: Using model {} for live response assistance, session {}", model, session_id);
    generate_agent_response_stream(app_handle, model, full_prompt, CONVERSATIONAL_AI_PROMPT.to_string(), None, session_id, "conversational_ai".to_string()).await
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
    
    let request = GenerateRequest {
        model: model.clone(),
        prompt: full_prompt,
        stream: Some(true),
        context: None,
        images: None,
        system: Some(system_prompt),
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

#[tauri::command]
pub async fn get_ollama_gpu_info() -> Result<GpuInfo, String> {
    // Try to get GPU info from nvidia-smi first (for NVIDIA GPUs)
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(nvidia_output) = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=gpu_name,memory.total")
            .arg("--format=csv,noheader,nounits")
            .output()
        {
            if nvidia_output.status.success() {
                let output_str = String::from_utf8_lossy(&nvidia_output.stdout);
                let parts: Vec<&str> = output_str.trim().split(',').collect();
                if parts.len() >= 2 {
                    return Ok(GpuInfo {
                        gpu_available: true,
                        gpu_type: Some("CUDA".to_string()),
                        gpu_memory: parts[1].trim().parse::<u64>().ok().map(|mb| mb * 1024 * 1024), // Convert MB to bytes
                        gpu_compute_capability: Some(parts[0].trim().to_string()),
                    });
                }
            }
        }
    }
    
    // Check for Metal on macOS
    #[cfg(target_os = "macos")]
    {
        return Ok(GpuInfo {
            gpu_available: true,
            gpu_type: Some("Metal".to_string()),
            gpu_memory: None, // Metal doesn't expose memory in the same way
            gpu_compute_capability: Some("Apple Silicon".to_string()),
        });
    }
    
    // No GPU detected
    Ok(GpuInfo {
        gpu_available: false,
        gpu_type: None,
        gpu_memory: None,
        gpu_compute_capability: None,
    })
}

#[tauri::command]
pub async fn check_ollama_gpu_usage(model_name: String) -> Result<serde_json::Value, String> {
    // Get model info to see if it's loaded in GPU
    let model_info = get_ollama_model_info(model_name.clone()).await?;
    let gpu_info = get_ollama_gpu_info().await?;
    
    // Ollama automatically uses GPU when available for compatible models
    // We can infer GPU usage based on model size and available GPU
    let response = serde_json::json!({
        "model": model_name,
        "gpu_info": gpu_info,
        "gpu_layers": model_info.get("gpu_layers").cloned(),
        "note": "Ollama automatically uses GPU acceleration when available"
    });
    
    Ok(response)
}

#[tauri::command]
pub async fn preload_ollama_model(model: String) -> Result<String, String> {
    let client = Arc::clone(&HTTP_CLIENT);
    let url = format!("{}/api/generate", OLLAMA_BASE_URL);
    
    // Send a minimal prompt to load the model into memory
    let request = GenerateRequest {
        model: model.clone(),
        prompt: "Hello".to_string(),
        stream: Some(false),
        context: None,
        images: None,
        system: None,
    };
    
    println!("🔄 Pre-loading model {} into memory...", model);
    
    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if response.status().is_success() {
                // We don't care about the response, just that the model loaded
                println!("✅ Model {} successfully pre-loaded into memory", model);
                Ok(format!("Model {} pre-loaded successfully", model))
            } else {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(format!("Failed to pre-load model: {}", error_text))
            }
        }
        Err(e) => Err(format!("Failed to connect to Ollama: {}. Make sure Ollama is running.", e)),
    }
} 