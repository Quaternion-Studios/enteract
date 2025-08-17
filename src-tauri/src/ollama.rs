use serde::{Deserialize, Serialize};
use serde_json;
use reqwest;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use std::sync::Mutex;
use crate::system_prompts::{
    ENTERACT_AGENT_PROMPT, 
    VISION_ANALYSIS_PROMPT, 
    DEEP_RESEARCH_PROMPT, 
    CONVERSATIONAL_AI_PROMPT,
    CODING_AGENT_PROMPT
};
use crate::system_info::get_gpu_info;
use regex;

// Shared HTTP client for better connection pooling and memory efficiency
lazy_static! {
    static ref HTTP_CLIENT: Arc<reqwest::Client> = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(16)  // More idle connections for faster reuse
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .timeout(Duration::from_secs(60))  // Shorter timeout to fail fast
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

// Stream state tracking for timeouts and pattern detection
#[derive(Debug)]
struct StreamState {
    start_time: Instant,
    last_chunk_time: Instant,
    chunk_count: usize,
    last_chunk_text: String,
    repeat_count: usize,
    consecutive_empty_count: usize, // Changed: track consecutive empty chunks
    total_empty_count: usize,       // Added: track total for debugging
}

#[derive(Debug)]
enum ChunkResult {
    Continue,
    Exit(String), // Exit with message
}

impl StreamState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_chunk_time: now,
            chunk_count: 0,
            last_chunk_text: String::new(),
            repeat_count: 0,
            consecutive_empty_count: 0,
            total_empty_count: 0,
        }
    }

    fn update_chunk(&mut self, chunk_text: &str) -> ChunkResult {
        self.last_chunk_time = Instant::now();
        self.chunk_count += 1;

        // Track empty chunks
        if chunk_text.trim().is_empty() {
            self.consecutive_empty_count += 1;
            self.total_empty_count += 1;
            
            // Optional: Log excessive consecutive empty chunks for debugging
            if self.consecutive_empty_count > 10 {
                println!("⚠️ {} consecutive empty chunks received", self.consecutive_empty_count);
            }
            
            return ChunkResult::Continue;
        } else {
            // Reset consecutive empty count when we get content
            self.consecutive_empty_count = 0;
        }

        // Check for consecutive identical chunks (dynamic repetition)
        if !self.last_chunk_text.is_empty() && self.last_chunk_text == chunk_text {
            self.repeat_count += 1;
            println!("⚠️ Consecutive repeat detected: '{}' (count: {})", 
                     chunk_text.chars().take(50).collect::<String>(), self.repeat_count);
            
            // More aggressive detection for very short or UI-related chunks
            if chunk_text.trim().len() <= 20 && self.repeat_count > 3 {
                println!("🛑 Exiting stream due to short repeat (likely UI artifact)");
                return ChunkResult::Exit(format!("Stream terminated: Detected repetitive short chunk '{}' ({} times) - possible AI confusion", 
                    chunk_text.chars().take(20).collect::<String>(), self.repeat_count));
            }
            
            // Standard detection for longer content
            if chunk_text.trim().len() > 20 && self.repeat_count > 8 {
                println!("🛑 Exiting stream due to excessive content repeat");
                return ChunkResult::Exit(format!("Stream terminated: Excessive repetition of content '{}' ({} times)", 
                    chunk_text.chars().take(30).collect::<String>(), self.repeat_count));
            }
            
            // Continue but keep counting - don't update last_chunk_text since it's the same
            return ChunkResult::Continue;
        } else {
            // FIXED: Only reset counter and update text when we have DIFFERENT content
            if !chunk_text.trim().is_empty() {
                self.repeat_count = 0; // Reset counter for different text
                self.last_chunk_text = chunk_text.to_string(); // Store new text
            }
        }

        ChunkResult::Continue
    }

    fn should_timeout(&self, max_total_duration: Duration, max_chunk_gap: Duration) -> Option<String> {
        let now = Instant::now();
        
        // Check total stream timeout
        if now.duration_since(self.start_time) > max_total_duration {
            return Some(format!("Total stream timeout after {:?}", max_total_duration));
        }
        
        // Check gap between chunks timeout
        if now.duration_since(self.last_chunk_time) > max_chunk_gap {
            return Some(format!("Chunk gap timeout after {:?}", max_chunk_gap));
        }
        
        None
    }

    fn should_terminate_patterns(&self, max_repeats: usize, max_consecutive_empty_chunks: usize) -> Option<String> {
        // Only terminate on very excessive repetition (adjusted thresholds)
        if self.repeat_count >= max_repeats * 3 {  // Triple the threshold for termination
            return Some(format!("Too many consecutive repeats: {}", self.repeat_count));
        }
        
        // Check consecutive empty chunks instead of total
        if self.consecutive_empty_count > max_consecutive_empty_chunks {
            return Some(format!("Too many consecutive empty chunks: {} (total empty: {})", 
                self.consecutive_empty_count, self.total_empty_count));
        }
        
        None
    }
}

// Base streaming configuration
pub struct StreamConfig {
    max_total_duration: Duration,
    max_chunk_gap: Duration,
    chunk_timeout: Duration,
    max_consecutive_repeats: usize,
    max_consecutive_empty_chunks: usize, 
}


impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_total_duration: Duration::from_secs(300), // 5 minutes total
            max_chunk_gap: Duration::from_secs(30),       // 30 seconds between chunks
            chunk_timeout: Duration::from_secs(10),       // 10 seconds per chunk read
            max_consecutive_repeats: 5,                   // Max 5 consecutive identical chunks
            max_consecutive_empty_chunks: 25,              // Max 25 consecutive empty chunks (increased)
        }
    }
}

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

// Enhanced streaming logic with timeout and pattern detection
async fn stream_ollama_response_enhanced(
    app_handle: AppHandle,
    url: String,
    request: GenerateRequest,
    session_id: String,
    config: StreamConfig,
) -> Result<(), String> {
    // Register the session as active
    {
        let mut sessions = ACTIVE_SESSIONS.lock().unwrap();
        sessions.insert(session_id.clone(), false);
    }

    let client = Arc::clone(&HTTP_CLIENT);
    
    // Make request with timeout
    let response = timeout(Duration::from_secs(30), client.post(&url).json(&request).send())
        .await
        .map_err(|_| "Request timeout".to_string())?
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        let error_msg = format!("Generation failed: {}", error_text);
        
        emit_error(&app_handle, &session_id, &error_msg).await;
        cleanup_session(&session_id);
        return Err(error_msg);
    }

    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    let mut state = StreamState::new();

    // Emit a tiny nudge to UI so it can render quickly even before first chunk
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "chunk",
        "text": "",
        "done": false
    })) {
        eprintln!("Failed to emit priming chunk: {}", e);
    }

    loop {
        // Check for cancellation first
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

        // Check timeouts
        if let Some(timeout_reason) = state.should_timeout(config.max_total_duration, config.max_chunk_gap) {
            println!("⏰ Stream timeout: {}", timeout_reason);
            emit_timeout(&app_handle, &session_id, &timeout_reason).await;
            emit_complete(&app_handle, &session_id).await;
            cleanup_session(&session_id);
            return Err(timeout_reason);
        }

        // Check problematic patterns
        if let Some(pattern_reason) = state.should_terminate_patterns(config.max_consecutive_repeats, config.max_consecutive_empty_chunks) {
            println!("🔁 Pattern termination: {}", pattern_reason);
            emit_error(&app_handle, &session_id, &pattern_reason).await;
            emit_complete(&app_handle, &session_id).await;
            cleanup_session(&session_id);
            return Err(pattern_reason);
        }

        // Read next chunk with timeout
        let chunk_result = timeout(config.chunk_timeout, stream.next()).await;
        
        let chunk_result = match chunk_result {
            Ok(Some(chunk_result)) => chunk_result,
            Ok(None) => {
                // Stream ended naturally
                println!("✅ Stream completed naturally for session: {}", session_id);
                emit_complete(&app_handle, &session_id).await;
                cleanup_session(&session_id);
                return Ok(());
            }
            Err(_) => {
                let error_msg = format!("Chunk read timeout after {:?}", config.chunk_timeout);
                println!("⏰ {}", error_msg);
                emit_timeout(&app_handle, &session_id, &error_msg).await;
                emit_complete(&app_handle, &session_id).await;
                cleanup_session(&session_id);
                return Err(error_msg);
            }
        };

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
                            // Check patterns and update state
                            match state.update_chunk(&response_chunk.response) {
                                ChunkResult::Continue => { 
                                    // Process chunk normally
                                }
                                ChunkResult::Exit(reason) => {
                                // 1. Send termination event with details
                                emit_termination(&app_handle, &session_id, &reason, state.chunk_count, state.repeat_count).await;
                                
                                // 2. Send completion event to reset UI state  
                                emit_complete(&app_handle, &session_id).await;
                                
                                // 3. Clean up session
                                cleanup_session(&session_id);
                                
                                return Ok(());
                                }
                            }

                            // Skip empty chunks to reduce UI overhead but still emit important ones
                            if response_chunk.response.is_empty() && !response_chunk.done {
                                continue;
                            }

                            if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                "type": "chunk",
                                "text": response_chunk.response,
                                "done": response_chunk.done,
                                "chunk_count": state.chunk_count,
                                "repeat_count": state.repeat_count
                            })) {
                                eprintln!("Failed to emit chunk event: {}", e);
                            }

                            if response_chunk.done {
                                println!("✅ Agent streaming completed for session: {} (chunks: {}, repeats: {})", 
                                         session_id, state.chunk_count, state.repeat_count);
                                emit_complete(&app_handle, &session_id).await;
                                cleanup_session(&session_id);
                                return Ok(());
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

                emit_error(&app_handle, &session_id, &error_msg).await;
                cleanup_session(&session_id);
                return Err(error_msg);
            }
        }
    }
}

// Shared streaming logic (backwards compatibility)
async fn stream_ollama_response(
    app_handle: AppHandle,
    url: String,
    request: GenerateRequest,
    session_id: String,
) -> Result<(), String> {
    stream_ollama_response_enhanced(app_handle, url, request, session_id, StreamConfig::default()).await
}

// Use enhanced streaming with default config - remove any old stream_ollama_response calls
// All streaming now goes through stream_ollama_response_enhanced

// Helper emit functions
async fn emit_error(app_handle: &AppHandle, session_id: &str, error: &str) {
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "error",
        "error": error
    })) {
        eprintln!("Failed to emit error: {}", e);
    }
}

async fn emit_timeout(app_handle: &AppHandle, session_id: &str, reason: &str) {
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "timeout",
        "reason": reason
    })) {
        eprintln!("Failed to emit timeout: {}", e);
    }
}

async fn emit_complete(app_handle: &AppHandle, session_id: &str) {
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "complete"
    })) {
        eprintln!("Failed to emit complete: {}", e);
    }
}

async fn emit_termination(app_handle: &AppHandle, session_id: &str, reason: &str, chunk_count: usize, repeat_count: usize) {
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "terminated",
        "reason": reason,
        "chunk_count": chunk_count,
        "repeat_count": repeat_count
    })) {
        eprintln!("Failed to emit termination: {}", e);
    }
}

// All your existing Tauri commands remain the same...

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
    
    // Use enhanced streaming with default config
    stream_ollama_response_enhanced(app_handle, url, request, session_id, StreamConfig::default()).await
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
    _custom_system_prompt: Option<String>, // Prefixed with underscore to indicate intentionally unused
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
        println!("AI agent type: {}", agent_type);
        // Balanced for comprehensive but focused conversation coaching
        let mut opts = serde_json::json!({
            "num_predict": 2048,
            "temperature": 0.7,
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
            "num_predict": 1024,
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
            "num_predict": 1024,
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
    
    // Use enhanced streaming with tighter config for agents
    let agent_config = StreamConfig {
        max_total_duration: Duration::from_secs(180), // 3 minutes for agents
        max_chunk_gap: Duration::from_secs(20),       // 20 seconds between chunks
        chunk_timeout: Duration::from_secs(8),        // 8 seconds per chunk
        max_consecutive_repeats: 3,                   // Max 3 consecutive repeats for agents
        max_consecutive_empty_chunks: 30,               // Max 30 consecutive empty chunks (increased)
    };

    
    let result = stream_ollama_response_enhanced(app_handle, url, request, session_id.clone(), agent_config).await;
    
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
                "num_predict": 1024,
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
    
    // Use enhanced streaming with vision-specific config
    let vision_config = StreamConfig {
        max_total_duration: Duration::from_secs(120), // 2 minutes for vision
        max_chunk_gap: Duration::from_secs(25),       // 25 seconds between chunks
        chunk_timeout: Duration::from_secs(10),       // 10 seconds per chunk
        max_consecutive_repeats: 4,                   // Max 4 consecutive repeats for vision
        max_consecutive_empty_chunks: 25,              // Max 25 consecutive empty chunks (increased)
    };

    let result = stream_ollama_response_enhanced(app_handle, url, request, session_id.clone(), vision_config).await;
    
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

// Additional helper function for custom timeout streaming (for specific use cases)
#[tauri::command]
pub async fn generate_with_custom_timeouts(
    app_handle: AppHandle,
    model: String,
    prompt: String,
    session_id: String,
    total_timeout_secs: u64,
    chunk_gap_secs: u64,
    max_repeats: usize,
) -> Result<(), String> {
    let url = format!("{}/api/generate", OLLAMA_BASE_URL);
    
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
        prompt,
        stream: Some(true),
        context: None,
        images: None,
        system: None,
        options,
    };
    
    println!("🚀 Starting custom timeout streaming for session: {} (total: {}s, gap: {}s, repeats: {})", 
             session_id, total_timeout_secs, chunk_gap_secs, max_repeats);
    
    // Emit start event
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "start",
        "model": model
    })) {
        return Err(format!("Failed to emit start event: {}", e));
    }
    
    let custom_config = StreamConfig {
        max_total_duration: Duration::from_secs(total_timeout_secs),
        max_chunk_gap: Duration::from_secs(chunk_gap_secs),
        chunk_timeout: Duration::from_secs(10),
        max_consecutive_repeats: max_repeats,
        max_consecutive_empty_chunks: 25,
    };
    
    stream_ollama_response_enhanced(app_handle, url, request, session_id, custom_config).await
}





// ADDED MCP FUNCTIONALITY


// Add these imports to the top of your ollama.rs file
use crate::mcp::commands::MCPSessionManager;
use crate::mcp::types::MCPSessionConfig;

// Add this new command for MCP-enabled AI responses
#[tauri::command]
pub async fn generate_mcp_enabled_response(
    app_handle: AppHandle,
    model: String,
    prompt: String,
    context: Option<Vec<ChatContextMessage>>,
    session_id: String,
    mcp_session_id: Option<String>,
    mcp_sessions: tauri::State<'_, MCPSessionManager>,
) -> Result<(), String> {
    let url = format!("{}/api/generate", OLLAMA_BASE_URL);
    
    // Build the enhanced system prompt that includes MCP capabilities
    let system_prompt = build_mcp_system_prompt(mcp_session_id.clone(), &mcp_sessions).await?;
    
    // Build full prompt with context and MCP capabilities
    let full_prompt = if let Some(mcp_id) = &mcp_session_id {
        format!("{}{}Context: You have access to computer control tools through MCP session {}. You can click, type, scroll, take screenshots, and more. Use tools when helpful to assist the user.\n\nUser Request: {}", 
                system_prompt, 
                if let Some(ctx) = context { build_context_string(ctx) } else { String::new() },
                mcp_id,
                prompt)
    } else {
        build_prompt_with_context(prompt, context)
    };
    
    // Detect GPU and set acceleration options
    let gpu_layers = detect_gpu_layers();
    let options = if gpu_layers > 0 {
        Some(serde_json::json!({
            "num_gpu": gpu_layers,
            "num_thread": 4,
            "temperature": 0.7,
            "top_p": 0.9,
            "repeat_penalty": 1.1
        }))
    } else {
        Some(serde_json::json!({
            "temperature": 0.7,
            "top_p": 0.9,
            "repeat_penalty": 1.1
        }))
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
    
    println!("🤖 Starting MCP-enabled streaming for session: {} (MCP: {:?})", session_id, mcp_session_id);
    
    // Emit start event
    if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
        "type": "start",
        "model": model,
        "mcp_enabled": mcp_session_id.is_some(),
        "mcp_session_id": mcp_session_id
    })) {
        return Err(format!("Failed to emit start event: {}", e));
    }
    
    // Use enhanced streaming with MCP tool execution
    stream_ollama_response_with_mcp(app_handle, url, request, session_id, mcp_session_id, mcp_sessions).await
}

// Helper function to build MCP-aware system prompt
async fn build_mcp_system_prompt(
    mcp_session_id: Option<String>,
    mcp_sessions: &tauri::State<'_, MCPSessionManager>,
) -> Result<String, String> {
    if let Some(session_id) = mcp_session_id {
        let sessions_guard = mcp_sessions.lock().await;
        if let Some(session) = sessions_guard.get(&session_id) {
            let tools = session.get_available_tools().await;
            
            let mut tool_descriptions = String::new();
            tool_descriptions.push_str("Available computer control tools:\n");
            
            for tool in &tools {
                tool_descriptions.push_str(&format!(
                    "- {}: {} (Risk: {:?})\n",
                    tool.name,
                    tool.description,
                    tool.danger_level
                ));
            }
            
            return Ok(format!(
                "You are an AI assistant with computer control capabilities. {}

When you need to use computer control tools, format your requests as:
TOOL_CALL: tool_name {{\"param1\": \"value1\", \"param2\": \"value2\"}}

Available tool calls:
- TOOL_CALL: click {{\"x\": 100, \"y\": 200}} - Click at coordinates
- TOOL_CALL: type {{\"text\": \"hello world\"}} - Type text
- TOOL_CALL: scroll {{\"direction\": \"up\", \"amount\": 3}} - Scroll
- TOOL_CALL: key_press {{\"key\": \"Enter\"}} - Press a key
- TOOL_CALL: take_screenshot {{}} - Take a screenshot
- TOOL_CALL: get_cursor_position {{}} - Get cursor position
- TOOL_CALL: get_screen_info {{}} - Get screen information

Always explain what you're doing and ask for permission for risky actions.",
                tool_descriptions
            ));
        }
    }
    
    Ok("You are a helpful AI assistant.".to_string())
}

// Enhanced streaming function that can execute MCP tools
async fn stream_ollama_response_with_mcp(
    app_handle: AppHandle,
    url: String,
    request: GenerateRequest,
    session_id: String,
    mcp_session_id: Option<String>,
    mcp_sessions: tauri::State<'_, MCPSessionManager>,
) -> Result<(), String> {
    // Register the session as active
    {
        let mut sessions = ACTIVE_SESSIONS.lock().unwrap();
        sessions.insert(session_id.clone(), false);
    }

    let client = Arc::clone(&HTTP_CLIENT);
    
    // Make request with timeout
    let response = timeout(Duration::from_secs(30), client.post(&url).json(&request).send())
        .await
        .map_err(|_| "Request timeout".to_string())?
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        let error_msg = format!("Generation failed: {}", error_text);
        
        emit_error(&app_handle, &session_id, &error_msg).await;
        cleanup_session(&session_id);
        return Err(error_msg);
    }

    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    let mut state = StreamState::new();
    let mut accumulated_response = String::new();

    loop {
        // Check for cancellation
        if is_session_cancelled(&session_id) {
            println!("🛑 Session cancelled: {}", session_id);
            cleanup_session(&session_id);
            return Ok(());
        }

        // Check timeouts and patterns
        if let Some(timeout_reason) = state.should_timeout(Duration::from_secs(300), Duration::from_secs(30)) {
            emit_timeout(&app_handle, &session_id, &timeout_reason).await;
            emit_complete(&app_handle, &session_id).await;
            cleanup_session(&session_id);
            return Err(timeout_reason);
        }

        // Read next chunk
        let chunk_result = timeout(Duration::from_secs(10), stream.next()).await;
        
        let chunk_result = match chunk_result {
            Ok(Some(chunk_result)) => chunk_result,
            Ok(None) => {
                // Process any remaining accumulated response for tool calls
                if !accumulated_response.is_empty() && mcp_session_id.is_some() {
                    process_tool_calls(&accumulated_response, &mcp_session_id.unwrap(), &mcp_sessions, &app_handle, &session_id).await;
                }
                
                emit_complete(&app_handle, &session_id).await;
                cleanup_session(&session_id);
                return Ok(());
            }
            Err(_) => {
                emit_timeout(&app_handle, &session_id, "Chunk read timeout").await;
                emit_complete(&app_handle, &session_id).await;
                cleanup_session(&session_id);
                return Err("Chunk read timeout".to_string());
            }
        };

        match chunk_result {
            Ok(chunk) => {
                buffer.extend_from_slice(&chunk);

                while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                    let line = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
                    let line_str = String::from_utf8_lossy(&line[..line.len()-1]);

                    if line_str.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<GenerateResponse>(&line_str) {
                        Ok(response_chunk) => {
                            match state.update_chunk(&response_chunk.response) {
                                ChunkResult::Continue => {},
                                ChunkResult::Exit(reason) => {
                                    emit_termination(&app_handle, &session_id, &reason, state.chunk_count, state.repeat_count).await;
                                    emit_complete(&app_handle, &session_id).await;
                                    cleanup_session(&session_id);
                                    return Ok(());
                                }
                            }

                            // Accumulate response for tool call detection
                            accumulated_response.push_str(&response_chunk.response);

                            // Check for tool calls in the accumulated response
                            if mcp_session_id.is_some() && accumulated_response.contains("TOOL_CALL:") {
                                // Process tool calls and get updated response
                                let processed_response = process_tool_calls(&accumulated_response, &mcp_session_id.as_ref().unwrap(), &mcp_sessions, &app_handle, &session_id).await;
                                if let Some(updated_response) = processed_response {
                                    accumulated_response = updated_response;
                                }
                            }

                            if !response_chunk.response.is_empty() || response_chunk.done {
                                if let Err(e) = app_handle.emit(&format!("ollama-stream-{}", session_id), serde_json::json!({
                                    "type": "chunk",
                                    "text": response_chunk.response,
                                    "done": response_chunk.done,
                                    "mcp_enabled": mcp_session_id.is_some()
                                })) {
                                    eprintln!("Failed to emit chunk event: {}", e);
                                }
                            }

                            if response_chunk.done {
                                emit_complete(&app_handle, &session_id).await;
                                cleanup_session(&session_id);
                                return Ok(());
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
                emit_error(&app_handle, &session_id, &error_msg).await;
                cleanup_session(&session_id);
                return Err(error_msg);
            }
        }
    }
}

// Process tool calls found in AI response
async fn process_tool_calls(
    response_text: &str,
    mcp_session_id: &str,
    mcp_sessions: &tauri::State<'_, MCPSessionManager>,
    app_handle: &AppHandle,
    session_id: &str,
) -> Option<String> {
    let tool_call_pattern = regex::Regex::new(r"TOOL_CALL:\s*(\w+)\s*(\{[^}]*\})").ok()?;
    
    if let Some(captures) = tool_call_pattern.find(response_text) {
        let tool_call_text = captures.as_str();
        println!("🔧 Detected tool call: {}", tool_call_text);
        
        // Parse tool name and parameters
        if let Some(caps) = tool_call_pattern.captures(tool_call_text) {
            let tool_name = caps.get(1)?.as_str();
            let params_str = caps.get(2)?.as_str();
            
            if let Ok(parameters) = serde_json::from_str::<serde_json::Value>(params_str) {
                // Execute the tool via MCP
                let sessions_guard = mcp_sessions.lock().await;
                let session = sessions_guard.get(mcp_session_id)?;
                
                match session.execute_tool(tool_name, parameters).await {
                    Ok(result) => {
                        let result_text = if result.success {
                            format!("✅ Tool executed successfully: {}", 
                                    result.result.get("message").and_then(|m| m.as_str()).unwrap_or("Success"))
                        } else {
                            format!("❌ Tool execution failed: {}", 
                                    result.error.as_deref().unwrap_or("Unknown error"))
                        };
                        
                        // Emit tool execution result to frontend
                        let _ = app_handle.emit(&format!("mcp-tool-result-{}", session_id), serde_json::json!({
                            "tool_name": tool_name,
                            "result": &result,
                            "session_id": session_id
                        }));
                        
                        // Replace the tool call with the result in the response
                        let updated_response = response_text.replace(tool_call_text, &result_text);
                        return Some(updated_response);
                    }
                    Err(e) => {
                        let error_text = format!("❌ Tool execution error: {}", e);
                        let updated_response = response_text.replace(tool_call_text, &error_text);
                        return Some(updated_response);
                    }
                }
            }
        }
    }
    
    None
}

// Helper function to build context string
fn build_context_string(context: Vec<ChatContextMessage>) -> String {
    let mut context_str = String::new();
    for message in &context {
        context_str.push_str(&format!("{}**: {}\n\n", 
            message.role.chars().next().unwrap().to_uppercase().collect::<String>() + &message.role[1..],
            message.content));
    }
    context_str
}

// Add MCP session management commands for the frontend
#[tauri::command]
pub async fn create_mcp_session_for_ai(
    app_handle: AppHandle,
    mcp_sessions: tauri::State<'_, MCPSessionManager>,
) -> Result<String, String> {
    let config = MCPSessionConfig {
        require_approval: true,
        session_timeout_seconds: 300,
        enable_logging: true,
        server_name: "enteract-ai-mcp".to_string(),
        server_version: "1.0.0".to_string(),
    };
    
    let session_info = crate::mcp::commands::start_mcp_session(
        Some(config),
        app_handle,
        mcp_sessions,
    ).await?;
    
    Ok(session_info.id)
}

#[tauri::command]
pub async fn get_mcp_session_for_ai(
    mcp_session_id: String,
    mcp_sessions: tauri::State<'_, MCPSessionManager>,
) -> Result<crate::mcp::types::MCPSessionInfo, String> {
    crate::mcp::commands::get_mcp_session_info(mcp_session_id, mcp_sessions).await
}