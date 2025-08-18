// src-tauri/src/mcp/server.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;
use tauri::{AppHandle, Emitter};
use chrono::Utc;

use crate::mcp::types::*;
use crate::mcp::tools::ComputerUseTool;

use log;

pub struct MCPSession {
    pub id: String,
    pub config: MCPSessionConfig,
    pub created_at: String,
    pub app_handle: AppHandle,
    pub pending_approvals: Arc<Mutex<HashMap<String, PendingApproval>>>,
    pub log_entries: Arc<Mutex<Vec<MCPLogEntry>>>,
    pub status: Arc<Mutex<SessionStatus>>,
    pub tools: Arc<Mutex<HashMap<String, Box<dyn ComputerUseTool + Send + Sync>>>>,
}

impl MCPSession {
    pub fn new(config: MCPSessionConfig, app_handle: AppHandle) -> Self {
        let session_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        
        log::info!("🚀 Creating new MCP session: {}", session_id);
        
        let mut tools: HashMap<String, Box<dyn ComputerUseTool + Send + Sync>> = HashMap::new();
        
        // Register computer use tools
        tools.insert("click".to_string(), Box::new(crate::mcp::tools::ClickTool));
        tools.insert("type".to_string(), Box::new(crate::mcp::tools::TypeTool));
        tools.insert("scroll".to_string(), Box::new(crate::mcp::tools::ScrollTool));
        tools.insert("key_press".to_string(), Box::new(crate::mcp::tools::KeyPressTool));
        tools.insert("get_cursor_position".to_string(), Box::new(crate::mcp::tools::GetCursorPositionTool));
        tools.insert("get_screen_info".to_string(), Box::new(crate::mcp::tools::GetScreenInfoTool));
        tools.insert("take_screenshot".to_string(), Box::new(crate::mcp::tools::ScreenshotTool));
        
        // Register new atomic OCR tools
        tools.insert("find_text".to_string(), Box::new(crate::mcp::tools::FindTextTool));
        tools.insert("click_at".to_string(), Box::new(crate::mcp::tools::ClickAtTool));
        tools.insert("debug_ocr".to_string(), Box::new(crate::mcp::tools::DebugOcrTool));
        
        // Register compound tools (require approval)
        tools.insert("click_on_text".to_string(), Box::new(crate::mcp::tools::ClickOnTextTool));
        tools.insert("click_and_type".to_string(), Box::new(crate::mcp::tools::ClickAndTypeTool));
        Self {
            id: session_id,
            config,
            created_at,
            app_handle,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            log_entries: Arc::new(Mutex::new(Vec::new())),
            status: Arc::new(Mutex::new(SessionStatus::Initializing)),
            tools: Arc::new(Mutex::new(tools)),
        }
    }
    
    pub async fn initialize(&self) -> Result<(), String> {
        {
            let mut status = self.status.lock().await;
            *status = SessionStatus::Active;
        }
        
        self.log(
            LogLevel::Info,
            "MCP session initialized".to_string(),
            None,
        ).await;
        
        Ok(())
    }
    
    pub async fn get_info(&self) -> MCPSessionInfo {
        let status = {
            let status_guard = self.status.lock().await;
            status_guard.clone()
        };
        
        let tools_available = {
            let tools_guard = self.tools.lock().await;
            let mut tool_infos = Vec::new();
            
            for (name, tool) in tools_guard.iter() {
                tool_infos.push(ToolInfo {
                    name: name.clone(),
                    description: tool.description(),
                    danger_level: tool.danger_level(),
                    requires_approval: tool.requires_approval(),
                    parameters_schema: tool.parameters_schema(),
                });
            }
            
            tool_infos
        };
        
        let approvals_pending = {
            let pending = self.pending_approvals.lock().await;
            pending.len()
        };
        
        MCPSessionInfo {
            id: self.id.clone(),
            created_at: self.created_at.clone(),
            config: self.config.clone(),
            tools_available,
            status,
            approvals_pending,
        }
    }
    
    pub async fn log(&self, level: LogLevel, message: String, tool_name: Option<String>) {
        if !self.config.enable_logging {
            return;
        }
        
        let entry = MCPLogEntry {
            session_id: self.id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            level: level.clone(),
            message: message.clone(),
            tool_name: tool_name.clone(),
            execution_result: None,
        };
        
        // Log to console
        match level {
            LogLevel::Info => log::info!("MCP[{}]: {}", self.id, message),
            LogLevel::Warning => log::warn!("MCP[{}]: {}", self.id, message),
            LogLevel::Error => log::error!("MCP[{}]: {}", self.id, message),
            LogLevel::Debug => log::debug!("MCP[{}]: {}", self.id, message),
        }
        
        // Store in session log
        let mut log_entries = self.log_entries.lock().await;
        log_entries.push(entry);
        
        // Emit to frontend if it's an important event
        if matches!(level, LogLevel::Info | LogLevel::Error) {
            let _ = self.app_handle.emit("mcp_log", serde_json::json!({
                "session_id": self.id,
                "level": level,
                "message": message,
                "tool_name": tool_name,
                "timestamp": Utc::now().to_rfc3339()
            }));
        }
    }
    
    async fn request_approval(
        &self,
        tool_name: &str,
        tool_description: &str,
        parameters: &serde_json::Value,
        danger_level: DangerLevel,
    ) -> Result<bool, String> {
        if !self.config.require_approval {
            return Ok(true);
        }
        
        // Check if tool requires approval based on danger level
        let requires_approval = matches!(danger_level, DangerLevel::Medium | DangerLevel::High | DangerLevel::Critical);
        if !requires_approval {
            return Ok(true);
        }
        
        // Update session status
        {
            let mut status = self.status.lock().await;
            *status = SessionStatus::WaitingForApproval;
        }
        
        let approval_id = Uuid::new_v4().to_string();
        let (response_sender, response_receiver) = oneshot::channel();
        
        let request = ToolApprovalRequest {
            session_id: self.id.clone(),
            tool_name: tool_name.to_string(),
            tool_description: tool_description.to_string(),
            parameters: parameters.clone(),
            timestamp: Utc::now().to_rfc3339(),
            danger_level,
        };
        
        // Store pending approval
        {
            let mut pending = self.pending_approvals.lock().await;
            pending.insert(approval_id.clone(), PendingApproval {
                request: request.clone(),
                response_sender,
            });
        }
        
        // Emit approval request to frontend
        self.app_handle.emit("mcp_approval_request", &request)
            .map_err(|e| format!("Failed to emit approval request: {}", e))?;
        
        self.log(
            LogLevel::Info,
            format!("Requesting approval for tool: {} ({})", tool_name, match request.danger_level {
                DangerLevel::Low => "low risk",
                DangerLevel::Medium => "medium risk",
                DangerLevel::High => "high risk",
                DangerLevel::Critical => "critical risk",
            }),
            Some(tool_name.to_string()),
        ).await;
        
        // Wait for response with timeout
        let timeout_duration = std::time::Duration::from_secs(self.config.session_timeout_seconds);
        
        match tokio::time::timeout(timeout_duration, response_receiver).await {
            Ok(Ok(response)) => {
                // Clean up pending approval
                {
                    let mut pending = self.pending_approvals.lock().await;
                    pending.remove(&approval_id);
                }
                
                // Update session status
                {
                    let mut status = self.status.lock().await;
                    *status = SessionStatus::Active;
                }
                
                self.log(
                    LogLevel::Info,
                    format!("Tool approval response: {}", if response.approved { "APPROVED" } else { "DENIED" }),
                    Some(tool_name.to_string()),
                ).await;
                
                Ok(response.approved)
            }
            Ok(Err(_)) => {
                self.log(
                    LogLevel::Error,
                    "Approval response channel closed".to_string(),
                    Some(tool_name.to_string()),
                ).await;
                Err("Approval response channel closed".to_string())
            }
            Err(_) => {
                // Timeout
                {
                    let mut pending = self.pending_approvals.lock().await;
                    pending.remove(&approval_id);
                }
                
                self.log(
                    LogLevel::Warning,
                    "Tool approval timed out".to_string(),
                    Some(tool_name.to_string()),
                ).await;
                
                Err("Approval request timed out".to_string())
            }
        }
    }
    
    pub async fn handle_approval_response(&self, response: ToolApprovalResponse) -> Result<(), String> {
        let mut pending = self.pending_approvals.lock().await;
        
        // Find the pending approval (we only expect one at a time for this demo)
        let approval_id = pending.keys().next().cloned();
        
        if let Some(id) = approval_id {
            if let Some(pending_approval) = pending.remove(&id) {
                let _ = pending_approval.response_sender.send(response);
                Ok(())
            } else {
                Err("No pending approval found".to_string())
            }
        } else {
            Err("No pending approvals".to_string())
        }
    }
    
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<ToolExecutionResult, String> {
        self.log(
            LogLevel::Info,
            format!("Executing tool: {} with params: {}", tool_name, parameters),
            Some(tool_name.to_string()),
        ).await;
        
        let tool = {
            let tools_guard = self.tools.lock().await;
            tools_guard.get(tool_name).map(|t| t.clone_box())
        };
        
        if let Some(tool) = tool {
            // Request approval if required
            let approved = self.request_approval(
                tool_name,
                &tool.description(),
                &parameters,
                tool.danger_level(),
            ).await?;
            
            if !approved {
                return Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"error": "User denied approval"}),
                    error: Some("User denied approval".to_string()),
                    execution_time_ms: 0,
                    tool_name: tool_name.to_string(),
                });
            }
            
            // Execute tool
            let result = tool.execute(parameters, &self.id).await;
            
            // Log the result
            if let Ok(ref exec_result) = result {
                let log_entry = MCPLogEntry {
                    session_id: self.id.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                    level: if exec_result.success { LogLevel::Info } else { LogLevel::Error },
                    message: format!("Tool execution completed: {}", tool_name),
                    tool_name: Some(tool_name.to_string()),
                    execution_result: Some(exec_result.clone()),
                };
                
                let mut log_entries = self.log_entries.lock().await;
                log_entries.push(log_entry);
            }
            
            result
        } else {
            let error_msg = format!("Unknown tool: {}", tool_name);
            self.log(LogLevel::Error, error_msg.clone(), Some(tool_name.to_string())).await;
            Err(error_msg)
        }
    }
    
    pub async fn get_available_tools(&self) -> Vec<ToolInfo> {
        let tools_guard = self.tools.lock().await;
        let mut tool_infos = Vec::new();
        
        for (name, tool) in tools_guard.iter() {
            tool_infos.push(ToolInfo {
                name: name.clone(),
                description: tool.description(),
                danger_level: tool.danger_level(),
                requires_approval: tool.requires_approval(),
                parameters_schema: tool.parameters_schema(),
            });
        }
        
        tool_infos
    }
    
    pub async fn generate_execution_plan(
        &self,
        user_request: &str,
        available_tools: Vec<ToolInfo>,
    ) -> Result<ToolExecutionPlan, String> {
        use uuid::Uuid;
        
        // For now, create a simple demo plan
        // TODO: Replace with actual LLM call to generate intelligent plan
        let plan_id = Uuid::new_v4().to_string();
        
        // Basic keyword-based planning (will be replaced with LLM)
        let mut steps = Vec::new();
        let request_lower = user_request.to_lowercase();
        
        if request_lower.contains("find") && request_lower.contains("text") {
            if let Some(_) = available_tools.iter().find(|t| t.name == "find_text") {
                let text_to_find = self.extract_quoted_text(user_request).unwrap_or_else(|| "Submit".to_string());
                
                steps.push(ToolStep {
                    step_id: Uuid::new_v4().to_string(),
                    tool_name: "find_text".to_string(),
                    description: format!("Find text '{}' on screen", text_to_find),
                    parameters: serde_json::json!({ "text": text_to_find }),
                    depends_on: None,
                    danger_level: DangerLevel::Low,
                    estimated_duration_ms: Some(2000),
                });
            }
        }
        
        if request_lower.contains("click") {
            if let Some(_) = available_tools.iter().find(|t| t.name == "click") {
                steps.push(ToolStep {
                    step_id: Uuid::new_v4().to_string(),
                    tool_name: "click".to_string(),
                    description: "Click on the found text location".to_string(),
                    parameters: serde_json::json!({}),
                    depends_on: steps.last().map(|s| s.step_id.clone()),
                    danger_level: DangerLevel::Medium,
                    estimated_duration_ms: Some(500),
                });
            }
        }
        
        if request_lower.contains("screenshot") {
            if let Some(_) = available_tools.iter().find(|t| t.name == "take_screenshot") {
                steps.push(ToolStep {
                    step_id: Uuid::new_v4().to_string(),
                    tool_name: "take_screenshot".to_string(),
                    description: "Take a screenshot".to_string(),
                    parameters: serde_json::json!({}),
                    depends_on: None,
                    danger_level: DangerLevel::Low,
                    estimated_duration_ms: Some(1000),
                });
            }
        }
        
        let overall_risk = steps.iter()
            .map(|s| s.danger_level)
            .max_by_key(|&level| match level {
                DangerLevel::Low => 1,
                DangerLevel::Medium => 2,
                DangerLevel::High => 3,
                DangerLevel::Critical => 4,
            })
            .unwrap_or(DangerLevel::Low);
        
        let requires_approval = steps.iter().any(|s| matches!(s.danger_level, DangerLevel::Medium | DangerLevel::High | DangerLevel::Critical));
        
        let plan = ToolExecutionPlan {
            session_id: self.id.clone(),
            plan_id,
            user_request: user_request.to_string(),
            steps,
            overall_risk,
            requires_approval,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        
        self.log(
            LogLevel::Info,
            format!("Generated execution plan with {} steps", plan.steps.len()),
            None,
        ).await;
        
        Ok(plan)
    }
    
    fn extract_quoted_text(&self, text: &str) -> Option<String> {
        // Extract text from quotes like "Submit" or 'Submit'
        if let Some(start) = text.find('"') {
            if let Some(end) = text[start + 1..].find('"') {
                return Some(text[start + 1..start + 1 + end].to_string());
            }
        }
        if let Some(start) = text.find('\'') {
            if let Some(end) = text[start + 1..].find('\'') {
                return Some(text[start + 1..start + 1 + end].to_string());
            }
        }
        None
    }
    pub async fn cleanup(&self) -> Result<(), String> {
        self.log(LogLevel::Info, "Cleaning up session".to_string(), None).await;
        
        // Cancel any pending approvals
        {
            let mut pending = self.pending_approvals.lock().await;
            pending.clear();
        }
        
        // Update status
        {
            let mut status = self.status.lock().await;
            *status = SessionStatus::Completed;
        }
        
        Ok(())
    }
}