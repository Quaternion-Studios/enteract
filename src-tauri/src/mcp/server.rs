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


    pub async fn generate_execution_plan_iterative(
        &self,
        user_request: &str,
        available_tools: Vec<ToolInfo>,
    ) -> Result<ToolExecutionPlan, String> {
        let mut conversation_history = Vec::new();
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 3;
        
        // Build context about current environment
        let context = self.build_execution_context().await;
        
        // Initial planning prompt
        let initial_prompt = format!(r#"
You are a computer automation assistant. Create a step-by-step execution plan.

USER REQUEST: "{}"

CURRENT CONTEXT:
- Available tools: {} tools
- Session ID: {}

AVAILABLE TOOLS:
{}

INSTRUCTIONS:
1. If you need more information, respond with: "QUESTION: What is..."
2. If you can create a plan, respond with valid JSON only
3. Be specific about parameters and tool usage
4. Consider tool dependencies and order

JSON FORMAT (if creating plan):
{{
  "steps": [
    {{
      "step_id": "step_1",
      "tool_name": "exact_tool_name_from_list",
      "description": "clear description of what this step does",
      "parameters": {{"param_name": "param_value"}},
      "requires_permission": true
    }}
  ],
  "reasoning": "why this plan will accomplish the goal"
}}

Respond with either a QUESTION or valid JSON plan:
"#, 
            user_request,
            available_tools.len(),
            self.id,
            self.format_tools_for_llm(&available_tools)
        );

        conversation_history.push(format!("System: {}", initial_prompt));
        
        // Iterative planning loop
        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                return Err("Could not generate a viable plan after multiple attempts".to_string());
            }
            
            self.log(LogLevel::Info, format!("Planning iteration {}/{}", iteration, MAX_ITERATIONS), None).await;
            
            // Generate LLM response using existing Ollama integration
            let llm_response = crate::ollama::generate_ollama_response(
                "llama3.2:3b".to_string(),
                conversation_history.join("\n\n")
            ).await.map_err(|e| format!("LLM planning failed: {}", e))?;
            
            conversation_history.push(format!("Assistant: {}", llm_response));
            
            // Handle LLM questions
            if llm_response.contains("QUESTION:") {
                let clarification = self.handle_planning_question(&llm_response, &available_tools, &context).await?;
                conversation_history.push(format!("System: {}", clarification));
                continue;
            }
            
            // Try to parse as execution plan
            match self.parse_llm_execution_plan(&llm_response, user_request) {
                Ok(plan) => {
                    // Validate plan feasibility
                    match self.validate_execution_plan(&plan, &available_tools).await {
                        Ok(true) => {
                            self.log(LogLevel::Info, format!("Generated valid plan with {} steps", plan.steps.len()), None).await;
                            return Ok(plan);
                        },
                        Ok(false) => {
                            let feedback = self.get_plan_validation_feedback(&plan, &available_tools).await;
                            conversation_history.push(format!(
                                "System: Plan validation failed. Issues: {}. Please create a revised plan.", 
                                feedback
                            ));
                        },
                        Err(e) => {
                            conversation_history.push(format!("System: Validation error: {}. Please revise.", e));
                        }
                    }
                },
                Err(parse_error) => {
                    conversation_history.push(format!(
                        "System: Could not parse your response as a plan. Error: {}. Please respond with either 'QUESTION: ...' or valid JSON.", 
                        parse_error
                    ));
                }
            }
        }
    }
    
    // Handle when LLM asks questions during planning
    async fn handle_planning_question(
        &self,
        question_response: &str,
        available_tools: &[ToolInfo],
        _context: &ExecutionContext, // Add underscore to fix unused variable
    ) -> Result<String, String> {
        let question = question_response
            .split("QUESTION:")
            .nth(1)
            .unwrap_or(question_response)
            .trim();
        
        let question_lower = question.to_lowercase();
        
        // Answer common questions programmatically
        if question_lower.contains("screen") || question_lower.contains("resolution") || question_lower.contains("display") {
            if available_tools.iter().any(|t| t.name == "get_screen_info") {
                return Ok("You can get screen information using the 'get_screen_info' tool. Consider starting with that.".to_string());
            }
        }
        
        if question_lower.contains("cursor") || question_lower.contains("mouse") || question_lower.contains("position") {
            if available_tools.iter().any(|t| t.name == "get_cursor_position") {
                return Ok("Current cursor position can be obtained using 'get_cursor_position' tool.".to_string());
            }
        }
        
        if question_lower.contains("see") || question_lower.contains("visible") || question_lower.contains("on screen") {
            if available_tools.iter().any(|t| t.name == "take_screenshot") { // Fix tool name
                return Ok("You can take a screenshot using the 'take_screenshot' tool to see what's currently on screen.".to_string());
            }
        }
        
        if question_lower.contains("text") || question_lower.contains("find") || question_lower.contains("locate") {
            if available_tools.iter().any(|t| t.name == "find_text") {
                return Ok("You can search for text on screen using the 'find_text' tool with specific text to search for.".to_string());
            }
        }
        
        // For complex questions, provide general guidance
        Ok(format!(
            "For your question '{}', proceed with your best approach using available tools. Start with information gathering tools like take_screenshot or get_screen_info if needed.",
            question
        ))
    }
    
    // Parse LLM response into structured execution plan
    fn parse_llm_execution_plan(&self, llm_response: &str, user_request: &str) -> Result<ToolExecutionPlan, String> {
        // Find JSON in response
        let json_start = llm_response.find('{').ok_or("No JSON object found in response")?;
        let json_end = llm_response.rfind('}').ok_or("No closing brace found in response")?;
        let json_str = &llm_response[json_start..=json_end];
        
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid JSON format: {}", e))?;
        
        let steps_array = parsed["steps"].as_array()
            .ok_or("Missing 'steps' array in plan")?;
        
        if steps_array.is_empty() {
            return Err("Plan contains no steps".to_string());
        }
        
        let mut tool_steps = Vec::new();
        for (i, step) in steps_array.iter().enumerate() {
            let step_id_str = format!("step_{}", i + 1); // Create owned string
            let step_id = step["step_id"].as_str()
                .unwrap_or(&step_id_str); // Now step_id_str lives long enough
            let tool_name = step["tool_name"].as_str()
                .ok_or(format!("Missing tool_name in step {}", i + 1))?;
            let description = step["description"].as_str()
                .unwrap_or("No description provided");
            let parameters = step["parameters"].clone();
            let _requires_permission = step["requires_permission"].as_bool().unwrap_or(true); // Add underscore
            
            tool_steps.push(ToolStep {
                step_id: step_id.to_string(),
                tool_name: tool_name.to_string(),
                description: description.to_string(),
                parameters,
                depends_on: if i > 0 { Some(format!("step_{}", i)) } else { None },
                danger_level: DangerLevel::Medium,
                estimated_duration_ms: Some(2000),
            });
        }
        
        let _reasoning = parsed["reasoning"].as_str().unwrap_or("No reasoning provided"); // Add underscore
        
        Ok(ToolExecutionPlan {
            session_id: self.id.clone(),
            plan_id: uuid::Uuid::new_v4().to_string(),
            user_request: user_request.to_string(),
            steps: tool_steps,
            overall_risk: DangerLevel::Medium,
            requires_approval: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    // Validate execution plan feasibility
    async fn validate_execution_plan(&self, plan: &ToolExecutionPlan, available_tools: &[ToolInfo]) -> Result<bool, String> {
        for step in &plan.steps {
            // Check if tool exists
            let tool_exists = available_tools.iter().any(|t| t.name == step.tool_name);
            if !tool_exists {
                return Ok(false);
            }
            
            // Basic parameter validation
            if step.parameters.is_null() && self.tool_requires_parameters(&step.tool_name, available_tools) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    
    // Get validation feedback for failed plans
    async fn get_plan_validation_feedback(&self, plan: &ToolExecutionPlan, available_tools: &[ToolInfo]) -> String {
        let mut issues = Vec::new();
        
        for step in &plan.steps {
            if !available_tools.iter().any(|t| t.name == step.tool_name) {
                issues.push(format!("Tool '{}' does not exist", step.tool_name));
            }
            
            if step.parameters.is_null() && self.tool_requires_parameters(&step.tool_name, available_tools) {
                issues.push(format!("Tool '{}' requires parameters", step.tool_name));
            }
        }
        
        if issues.is_empty() {
            "Unknown validation issues".to_string()
        } else {
            issues.join(", ")
        }
    }
    
    // Execute approved plan with user interaction
    // Fix the danger level comparison by implementing PartialEq
    // This should be done in types.rs, but here's the fix for the server.rs usage:
    // Replace the != comparison with matches!
    pub async fn execute_plan_with_interaction(&self, plan: &ToolExecutionPlan) -> Result<Vec<ToolExecutionResult>, String> {
        let mut results = Vec::new();
        let mut execution_context = ExecutionContext::new();
        
        for (i, step) in plan.steps.iter().enumerate() {
            self.log(LogLevel::Info, format!("Executing step {}: {}", i + 1, step.description), Some(step.tool_name.clone())).await;
            
            // Emit progress update
            let _ = self.app_handle.emit("mcp_execution_progress", serde_json::json!({
                "session_id": self.id,
                "step_number": i + 1,
                "total_steps": plan.steps.len(),
                "step_description": step.description,
                "tool_name": step.tool_name
            }));
            
            // Request approval if needed - fix the comparison
            if !matches!(step.danger_level, DangerLevel::Low) { // Use matches! instead of !=
                let approved = self.request_approval(
                    &step.tool_name,
                    &step.description,
                    &step.parameters,
                    step.danger_level
                ).await?;
                
                if !approved {
                    let error_result = ToolExecutionResult {
                        success: false,
                        result: serde_json::json!({"error": "User denied approval"}),
                        error: Some("Execution cancelled by user".to_string()),
                        execution_time_ms: 0,
                        tool_name: step.tool_name.clone(),
                    };
                    results.push(error_result);
                    break;
                }
            }
            
            // Execute the tool
            match self.execute_tool(&step.tool_name, step.parameters.clone()).await {
                Ok(result) => {
                    execution_context.update_from_result(&step.step_id, &result);
                    results.push(result);
                },
                Err(e) => {
                    let error_result = ToolExecutionResult {
                        success: false,
                        result: serde_json::json!({"error": e}),
                        error: Some(e.clone()),
                        execution_time_ms: 0,
                        tool_name: step.tool_name.clone(),
                    };
                    results.push(error_result);
                    
                    // For now, stop on first error (could be enhanced to continue/retry)
                    break;
                }
            }
        }
        
        Ok(results)
    }
    
    // Helper methods
    fn format_tools_for_llm(&self, tools: &[ToolInfo]) -> String {
        tools.iter()
            .map(|tool| {
                let key_params = self.extract_key_parameters(&tool.parameters_schema);
                format!("- **{}**: {} (Parameters: {})", 
                    tool.name, 
                    tool.description, 
                    key_params
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn extract_key_parameters(&self, schema: &serde_json::Value) -> String {
        if let Some(properties) = schema.get("properties") {
            if let Some(obj) = properties.as_object() {
                let required = schema.get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();
                
                return obj.keys()
                    .map(|key| if required.contains(&key.as_str()) { 
                        format!("{}*", key) 
                    } else { 
                        key.clone() 
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
            }
        }
        "none".to_string()
    }
    
    fn tool_requires_parameters(&self, tool_name: &str, available_tools: &[ToolInfo]) -> bool {
        if let Some(tool) = available_tools.iter().find(|t| t.name == tool_name) {
            if let Some(required) = tool.parameters_schema.get("required") {
                if let Some(required_array) = required.as_array() {
                    return !required_array.is_empty();
                }
            }
        }
        false
    }
    
    async fn build_execution_context(&self) -> ExecutionContext {
        ExecutionContext {
            session_id: self.id.clone(),
            screen_width: 1920, // Could be enhanced to get actual screen info
            screen_height: 1080,
            cursor_x: 0,
            cursor_y: 0,
            previous_actions: Vec::new(),
        }
    }
}