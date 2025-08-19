// src-tauri/src/mcp/commands.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, State, Emitter};

use crate::mcp::types::*;
use crate::mcp::server::MCPSession;

// Global state for active MCP sessions
pub type MCPSessionManager = Arc<Mutex<HashMap<String, Arc<MCPSession>>>>;

#[tauri::command]
pub async fn start_mcp_session(
    config: Option<MCPSessionConfig>,
    app_handle: AppHandle,
    sessions: State<'_, MCPSessionManager>,
) -> Result<MCPSessionInfo, String> {
    let session_config = config.unwrap_or_default();
    let session = Arc::new(MCPSession::new(session_config, app_handle));
    
    // Initialize the session
    session.initialize().await?;
    
    let session_info = session.get_info().await;
    
    // Store session in global state
    {
        let mut sessions_guard = sessions.lock().await;
        sessions_guard.insert(session.id.clone(), session.clone());
    }
    
    // Log session creation
    session.log(
        LogLevel::Info,
        "MCP session started with computer use capabilities".to_string(),
        None,
    ).await;
    
    println!("✅ MCP Session created: {} with {} tools", session.id, session_info.tools_available.len());
    
    Ok(session_info)
}

#[tauri::command]
pub async fn end_mcp_session(
    session_id: String,
    sessions: State<'_, MCPSessionManager>,
) -> Result<(), String> {
    let session = {
        let mut sessions_guard = sessions.lock().await;
        sessions_guard.remove(&session_id)
    };
    
    if let Some(session) = session {
        session.cleanup().await?;
        println!("🔄 MCP Session ended: {}", session_id);
        Ok(())
    } else {
        Err(format!("Session not found: {}", session_id))
    }
}

#[tauri::command]
pub async fn get_mcp_session_info(
    session_id: String,
    sessions: State<'_, MCPSessionManager>,
) -> Result<MCPSessionInfo, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    Ok(session.get_info().await)
}

#[tauri::command]
pub async fn list_mcp_tools(
    session_id: String,
    sessions: State<'_, MCPSessionManager>,
) -> Result<Vec<ToolInfo>, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    Ok(session.get_available_tools().await)
}

#[tauri::command]
pub async fn execute_mcp_tool(
    session_id: String,
    tool_name: String,
    parameters: serde_json::Value,
    sessions: State<'_, MCPSessionManager>,
) -> Result<ToolExecutionResult, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    session.execute_tool(&tool_name, parameters).await
}

#[tauri::command]
pub async fn respond_to_mcp_approval_with_id(
    session_id: String,
    approval_id: String,
    approved: bool,
    reason: Option<String>,
    sessions: State<'_, MCPSessionManager>,
) -> Result<(), String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    // Create the response in the expected format
    let response = ToolApprovalResponse {
        session_id: session_id.clone(),
        approved,
        reason,
    };
    
    session.handle_approval_response(response).await
}

#[tauri::command]
pub async fn get_mcp_session_logs(
    session_id: String,
    sessions: State<'_, MCPSessionManager>,
) -> Result<Vec<MCPLogEntry>, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    let log_entries = session.log_entries.lock().await;
    Ok(log_entries.clone())
}

#[tauri::command]
pub async fn list_active_mcp_sessions(
    sessions: State<'_, MCPSessionManager>,
) -> Result<Vec<MCPSessionInfo>, String> {
    let sessions_guard = sessions.lock().await;
    let mut session_infos = Vec::new();
    
    for session in sessions_guard.values() {
        session_infos.push(session.get_info().await);
    }
    
    Ok(session_infos)
}

#[tauri::command]
pub async fn get_mcp_tool_schema(
    session_id: String,
    tool_name: String,
    sessions: State<'_, MCPSessionManager>,
) -> Result<serde_json::Value, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    let tools = session.get_available_tools().await;
    let tool = tools.iter().find(|t| t.name == tool_name)
        .ok_or(format!("Tool not found: {}", tool_name))?;
    
    Ok(tool.parameters_schema.clone())
}

#[tauri::command]
pub async fn get_mcp_session_status(
    session_id: String,
    sessions: State<'_, MCPSessionManager>,
) -> Result<SessionStatus, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    let status = session.status.lock().await;
    Ok(status.clone())
}

// New LLM-driven MCP commands


// Initialize the MCP session manager
#[tauri::command]
pub fn create_mcp_session_manager() -> MCPSessionManager {
    Arc::new(Mutex::new(HashMap::new()))
}


#[tauri::command]
pub async fn create_execution_plan_iterative(
    session_id: String,
    user_request: String,
    app_handle: AppHandle,
    sessions: State<'_, MCPSessionManager>,
) -> Result<ToolExecutionPlan, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    // Emit planning start event
    let _ = app_handle.emit("mcp_planning_progress", serde_json::json!({
        "session_id": session_id,
        "iteration": 1,
        "max_iterations": 3,
        "status": "Analyzing",
        "message": "Starting intelligent plan generation..."
    }));
    
    // Get available tools for planning
    let available_tools = session.get_available_tools().await;
    
    // Generate plan using iterative approach
    let plan = session.generate_execution_plan_iterative(&user_request, available_tools).await?;
    
    // Emit planning complete event
    let _ = app_handle.emit("mcp_planning_progress", serde_json::json!({
        "session_id": session_id,
        "iteration": 3,
        "max_iterations": 3,
        "status": "Complete",
        "message": format!("Generated plan with {} steps", plan.steps.len())
    }));
    
    Ok(plan)
}

#[tauri::command]
pub async fn execute_plan_interactive(
    plan: ToolExecutionPlan,
    sessions: State<'_, MCPSessionManager>,
) -> Result<Vec<ToolExecutionResult>, String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&plan.session_id)
        .ok_or(format!("Session not found: {}", plan.session_id))?;
    
    session.execute_plan_with_interaction(&plan).await
}

#[tauri::command]
pub async fn respond_to_mcp_approval(
    session_id: String,
    approval_id: String,
    approved: bool,
    reason: Option<String>,
    sessions: State<'_, MCPSessionManager>,
) -> Result<(), String> {
    let sessions_guard = sessions.lock().await;
    let session = sessions_guard.get(&session_id)
        .ok_or(format!("Session not found: {}", session_id))?;
    
    // Create the response in the expected format
    let response = ToolApprovalResponse {
        session_id: session_id.clone(),
        approved,
        reason,
    };
    
    session.handle_approval_response(response).await
}