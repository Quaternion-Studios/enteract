// src-tauri/src/mcp/types.rs
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSessionConfig {
    pub require_approval: bool,
    pub session_timeout_seconds: u64,
    pub enable_logging: bool,
    pub server_name: String,
    pub server_version: String,
}

impl Default for MCPSessionConfig {
    fn default() -> Self {
        Self {
            require_approval: true,
            session_timeout_seconds: 300, // 5 minutes
            enable_logging: true,
            server_name: "enteract-mcp-server".to_string(),
            server_version: "1.0.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalRequest {
    pub session_id: String,
    pub tool_name: String,
    pub tool_description: String,
    pub parameters: serde_json::Value,
    pub timestamp: String,
    pub danger_level: DangerLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalResponse {
    pub session_id: String,
    pub approved: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSessionInfo {
    pub id: String,
    pub created_at: String,
    pub config: MCPSessionConfig,
    pub tools_available: Vec<ToolInfo>,
    pub status: SessionStatus,
    pub approvals_pending: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Initializing,
    Active,
    WaitingForApproval,
    Completed,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub tool_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub danger_level: DangerLevel,
    pub requires_approval: bool,
    pub parameters_schema: serde_json::Value,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum DangerLevel {
    Low,      // Reading data, getting cursor position
    Medium,   // Clicking, typing, scrolling
    High,     // File operations, system commands
    Critical, // Destructive operations
}

// New types for LLM-driven MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionPlan {
    pub session_id: String,
    pub plan_id: String,
    pub user_request: String,
    pub steps: Vec<ToolStep>,
    pub overall_risk: DangerLevel,
    pub requires_approval: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStep {
    pub step_id: String,
    pub tool_name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub depends_on: Option<String>, // Previous step ID
    pub danger_level: DangerLevel,
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlanApproval {
    pub plan_id: String,
    pub approved: bool,
    pub approved_steps: Vec<String>, // Step IDs that are approved
    pub reason: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextData {
    pub step_id: String,
    pub tool_name: String,
    pub result: serde_json::Value,
    pub success: bool,
}

// Internal types for approval workflow
pub struct PendingApproval {
    pub request: ToolApprovalRequest,
    pub response_sender: oneshot::Sender<ToolApprovalResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPLogEntry {
    pub session_id: String,
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
    pub tool_name: Option<String>,
    pub execution_result: Option<ToolExecutionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

// Computer use specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickParams {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub button: Option<MouseButton>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeParams {
    pub text: String,
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollParams {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub direction: ScrollDirection,
    pub amount: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPressParams {
    pub key: String,
    pub modifiers: Option<Vec<KeyModifier>>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum KeyModifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotParams {
    pub format: Option<String>, // "png", "jpeg"
    pub quality: Option<u8>,    // 1-100 for jpeg
    pub region: Option<ScreenRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub image_base64: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}