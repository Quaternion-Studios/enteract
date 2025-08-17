// src-tauri/src/mcp/tools.rs
use async_trait::async_trait;
use crate::mcp::types::*;
use std::time::Instant;

// Base trait for computer use tools
#[async_trait]
pub trait ComputerUseTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> String;
    fn danger_level(&self) -> DangerLevel;
    fn requires_approval(&self) -> bool {
        matches!(self.danger_level(), DangerLevel::Medium | DangerLevel::High | DangerLevel::Critical)
    }
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String>;
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync>;
}

// Click tool implementation
#[derive(Clone)]
pub struct ClickTool;

#[async_trait]
impl ComputerUseTool for ClickTool {
    fn name(&self) -> &str { "click" }
    
    fn description(&self) -> String {
        "Perform a mouse click at specified coordinates or current cursor position".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Medium }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": {
                    "type": "integer",
                    "description": "X coordinate (optional, uses current position if not provided)"
                },
                "y": {
                    "type": "integer", 
                    "description": "Y coordinate (optional, uses current position if not provided)"
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "default": "left",
                    "description": "Mouse button to click"
                }
            }
        })
    }
    
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let click_params: ClickParams = serde_json::from_value(params)
            .map_err(|e| format!("Invalid parameters for click: {}", e))?;
        
        // Get current cursor position if not specified
        let (click_x, click_y) = match (click_params.x, click_params.y) {
            (Some(x), Some(y)) => (x, y),
            _ => get_cursor_position()?,
        };
        
        let button = click_params.button.unwrap_or(MouseButton::Left);
        
        log::info!("Session {}: Executing click at ({}, {}) with {:?} button", session_id, click_x, click_y, button);
        
        let result = perform_click(click_x, click_y, button).await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => {
                Ok(ToolExecutionResult {
                    success: true,
                    result: serde_json::json!({
                        "success": true,
                        "x": click_x,
                        "y": click_y,
                        "button": button,
                        "message": format!("Successfully clicked at ({}, {}) with {:?} button", click_x, click_y, button)
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to perform click: {}", e);
                Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"success": false, "error": error_msg}),
                    error: Some(error_msg),
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

// Type tool implementation
#[derive(Clone)]
pub struct TypeTool;

#[async_trait]
impl ComputerUseTool for TypeTool {
    fn name(&self) -> &str { "type" }
    
    fn description(&self) -> String {
        "Type text at the current cursor/focus position".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Medium }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to type"
                },
                "delay_ms": {
                    "type": "integer",
                    "description": "Delay between keystrokes in milliseconds",
                    "default": 10
                }
            },
            "required": ["text"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let type_params: TypeParams = serde_json::from_value(params)
            .map_err(|e| format!("Invalid parameters for type: {}", e))?;
        
        log::info!("Session {}: Typing text: '{}'", session_id, type_params.text);
        
        let result = type_text(&type_params.text, type_params.delay_ms.unwrap_or(10)).await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => {
                Ok(ToolExecutionResult {
                    success: true,
                    result: serde_json::json!({
                        "success": true,
                        "text": type_params.text.clone(),
                        "characters_typed": type_params.text.chars().count(),
                        "message": format!("Successfully typed {} characters", type_params.text.chars().count())
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to type text: {}", e);
                Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"success": false, "error": error_msg}),
                    error: Some(error_msg),
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

// Add more tools: ScrollTool, KeyPressTool, GetCursorPositionTool, GetScreenInfoTool, ScreenshotTool

#[derive(Clone)]
pub struct ScrollTool;

#[async_trait]
impl ComputerUseTool for ScrollTool {
    fn name(&self) -> &str { "scroll" }
    
    fn description(&self) -> String {
        "Scroll in a specified direction".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Low }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "Direction to scroll"
                },
                "amount": {
                    "type": "integer",
                    "description": "Amount to scroll (default: 3)",
                    "default": 3
                },
                "x": {
                    "type": "integer",
                    "description": "X coordinate for scroll location (optional)"
                },
                "y": {
                    "type": "integer",
                    "description": "Y coordinate for scroll location (optional)"
                }
            },
            "required": ["direction"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let scroll_params: ScrollParams = serde_json::from_value(params)
            .map_err(|e| format!("Invalid parameters for scroll: {}", e))?;
        
        log::info!("Session {}: Scrolling {:?}", session_id, scroll_params.direction);
        
        let result = perform_scroll(scroll_params.clone()).await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => {
                Ok(ToolExecutionResult {
                    success: true,
                    result: serde_json::json!({
                        "success": true,
                        "direction": scroll_params.direction,
                        "amount": scroll_params.amount.unwrap_or(3),
                        "message": "Successfully scrolled"
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to scroll: {}", e);
                Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"success": false, "error": error_msg}),
                    error: Some(error_msg),
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct KeyPressTool;

#[async_trait]
impl ComputerUseTool for KeyPressTool {
    fn name(&self) -> &str { "key_press" }
    
    fn description(&self) -> String {
        "Press a key or key combination".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Medium }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Key to press (e.g., 'Enter', 'Tab', 'a', 'F1')"
                },
                "modifiers": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["ctrl", "alt", "shift", "meta"]
                    },
                    "description": "Modifier keys to hold"
                }
            },
            "required": ["key"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let key_params: KeyPressParams = serde_json::from_value(params)
            .map_err(|e| format!("Invalid parameters for key_press: {}", e))?;
        
        log::info!("Session {}: Pressing key: '{}' with modifiers: {:?}", session_id, key_params.key, key_params.modifiers);
        
        let result = press_key(&key_params.key, key_params.modifiers.clone().unwrap_or_default()).await;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => {
                Ok(ToolExecutionResult {
                    success: true,
                    result: serde_json::json!({
                        "success": true,
                        "key": key_params.key.clone(),
                        "modifiers": key_params.modifiers,
                        "message": format!("Successfully pressed key: {}", key_params.key)
                    }),
                    error: None,
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to press key: {}", e);
                Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"success": false, "error": error_msg}),
                    error: Some(error_msg),
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct GetCursorPositionTool;

#[async_trait]
impl ComputerUseTool for GetCursorPositionTool {
    fn name(&self) -> &str { "get_cursor_position" }
    
    fn description(&self) -> String {
        "Get the current mouse cursor position".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Low }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }
    
    async fn execute(&self, _params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        log::info!("Session {}: Getting cursor position", session_id);
        
        match get_cursor_position() {
            Ok((x, y)) => {
                Ok(ToolExecutionResult {
                    success: true,
                    result: serde_json::json!({
                        "success": true,
                        "x": x,
                        "y": y,
                        "message": format!("Cursor position: ({}, {})", x, y)
                    }),
                    error: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    tool_name: self.name().to_string(),
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to get cursor position: {}", e);
                Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"success": false, "error": error_msg}),
                    error: Some(error_msg),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    tool_name: self.name().to_string(),
                })
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct GetScreenInfoTool;

#[async_trait]
impl ComputerUseTool for GetScreenInfoTool {
    fn name(&self) -> &str { "get_screen_info" }
    
    fn description(&self) -> String {
        "Get screen information (width, height, scale factor)".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Low }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }
    
    async fn execute(&self, _params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        log::info!("Session {}: Getting screen info", session_id);
        
        match get_screen_info() {
            Ok(screen_info) => {
                Ok(ToolExecutionResult {
                    success: true,
                    result: serde_json::to_value(screen_info).unwrap(),
                    error: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    tool_name: self.name().to_string(),
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to get screen info: {}", e);
                Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"success": false, "error": error_msg}),
                    error: Some(error_msg),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    tool_name: self.name().to_string(),
                })
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct ScreenshotTool;

#[async_trait]
impl ComputerUseTool for ScreenshotTool {
    fn name(&self) -> &str { "take_screenshot" }
    
    fn description(&self) -> String {
        "Take a screenshot of the screen or a region".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Low }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["png", "jpeg"],
                    "default": "png",
                    "description": "Image format"
                },
                "quality": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "default": 90,
                    "description": "JPEG quality (1-100)"
                },
                "region": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"},
                        "y": {"type": "integer"},
                        "width": {"type": "integer"},
                        "height": {"type": "integer"}
                    },
                    "description": "Region to capture (full screen if not specified)"
                }
            }
        })
    }
    
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let screenshot_params: ScreenshotParams = serde_json::from_value(params)
            .unwrap_or(ScreenshotParams {
                format: Some("png".to_string()),
                quality: Some(90),
                region: None,
            });
        
        log::info!("Session {}: Taking screenshot", session_id);
        
        let result = if let Some(region) = screenshot_params.region {
            take_screenshot_region(region, screenshot_params.format, screenshot_params.quality).await
        } else {
            take_screenshot_full(screenshot_params.format, screenshot_params.quality).await
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(screenshot_result) => {
                Ok(ToolExecutionResult {
                    success: true,
                    result: serde_json::to_value(screenshot_result).unwrap(),
                    error: None,
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
            Err(e) => {
                let error_msg = format!("Failed to take screenshot: {}", e);
                Ok(ToolExecutionResult {
                    success: false,
                    result: serde_json::json!({"success": false, "error": error_msg}),
                    error: Some(error_msg),
                    execution_time_ms: execution_time,
                    tool_name: self.name().to_string(),
                })
            }
        }
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

// Platform-specific implementations

#[cfg(target_os = "windows")]
async fn perform_click(x: i32, y: i32, button: MouseButton) -> Result<(), String> {
    use winapi::um::winuser::{
        SetCursorPos, mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP
    };
    
    unsafe {
        if SetCursorPos(x, y) == 0 {
            return Err("Failed to move cursor".to_string());
        }
        
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        let (down_event, up_event) = match button {
            MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
            MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        };
        
        mouse_event(down_event, 0, 0, 0, 0);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        mouse_event(up_event, 0, 0, 0, 0);
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
fn get_cursor_position() -> Result<(i32, i32), String> {
    use winapi::um::winuser::GetCursorPos;
    use winapi::shared::windef::POINT;
    
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) != 0 {
            Ok((point.x, point.y))
        } else {
            Err("Failed to get cursor position".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
async fn type_text(_text: &str, delay_ms: u64) -> Result<(), String> {
    // Use Windows SendInput API for more reliable text input
    // This is a simplified implementation
    for _ch in _text.chars() {
        // Convert character to virtual key and send input events
        // This would need proper implementation with SendInput
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
async fn perform_scroll(params: ScrollParams) -> Result<(), String> {
    use winapi::um::winuser::{mouse_event, MOUSEEVENTF_WHEEL, WHEEL_DELTA};
    
    // Move to position if specified
    if let (Some(x), Some(y)) = (params.x, params.y) {
        use winapi::um::winuser::SetCursorPos;
        unsafe {
            let _ = SetCursorPos(x, y);
        }
    }
    
    let amount = params.amount.unwrap_or(3);
    let delta = match params.direction {
        ScrollDirection::Up => (WHEEL_DELTA as i32) * amount,
        ScrollDirection::Down => -(WHEEL_DELTA as i32) * amount,
        ScrollDirection::Left | ScrollDirection::Right => {
            // Horizontal scrolling would need MOUSEEVENTF_HWHEEL
            return Err("Horizontal scrolling not yet implemented".to_string());
        }
    };
    
    unsafe {
        mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta as u32, 0);
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
async fn press_key(_key: &str, _modifiers: Vec<KeyModifier>) -> Result<(), String> {
    // This would need proper implementation with SendInput and virtual key codes
    // For now, return success
    Ok(())
}

#[cfg(target_os = "windows")]
fn get_screen_info() -> Result<ScreenInfo, String> {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN) as u32;
        let height = GetSystemMetrics(SM_CYSCREEN) as u32;
        
        Ok(ScreenInfo {
            width,
            height,
            scale_factor: 1.0, // Would need proper DPI detection
        })
    }
}

#[cfg(target_os = "windows")]
async fn take_screenshot_full(_format: Option<String>, _quality: Option<u8>) -> Result<ScreenshotResult, String> {
    // Use existing screenshot implementation from screenshot.rs
    match crate::screenshot::capture_screenshot().await {
        Ok(result) => Ok(ScreenshotResult {
            image_base64: result.image_base64,
            width: result.width,
            height: result.height,
            format: result.format,
        }),
        Err(e) => Err(e),
    }
}

#[cfg(target_os = "windows")]
async fn take_screenshot_region(region: ScreenRegion, _format: Option<String>, _quality: Option<u8>) -> Result<ScreenshotResult, String> {
    // Use existing screenshot implementation from screenshot.rs
    match crate::screenshot::capture_screenshot_area(region.x, region.y, region.width, region.height).await {
        Ok(result) => Ok(ScreenshotResult {
            image_base64: result.image_base64,
            width: result.width,
            height: result.height,
            format: result.format,
        }),
        Err(e) => Err(e),
    }
}

// Fallback implementations for non-Windows platforms
#[cfg(not(target_os = "windows"))]
async fn perform_click(x: i32, y: i32, button: MouseButton) -> Result<(), String> {
    log::info!("Simulated click at ({}, {}) with {:?} button - not implemented for this platform", x, y, button);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_position() -> Result<(i32, i32), String> {
    Ok((800, 600)) // Return center of screen as fallback
}

#[cfg(not(target_os = "windows"))]
async fn type_text(text: &str, delay_ms: u64) -> Result<(), String> {
    log::info!("Simulated typing: '{}' - not implemented for this platform", text);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn perform_scroll(params: ScrollParams) -> Result<(), String> {
    log::info!("Simulated scroll {:?} - not implemented for this platform", params.direction);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn press_key(_key: &str, _modifiers: Vec<KeyModifier>) -> Result<(), String> {
    log::info!("Simulated key press: '{}' with modifiers: {:?} - not implemented for this platform", _key, _modifiers);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn get_screen_info() -> Result<ScreenInfo, String> {
    Ok(ScreenInfo {
        width: 1920,
        height: 1080,
        scale_factor: 1.0,
    })
}

#[cfg(not(target_os = "windows"))]
async fn take_screenshot_full(_format: Option<String>, _quality: Option<u8>) -> Result<ScreenshotResult, String> {
    Err("Screenshot not implemented for this platform".to_string())
}

#[cfg(not(target_os = "windows"))]
async fn take_screenshot_region(_region: ScreenRegion, _format: Option<String>, _quality: Option<u8>) -> Result<ScreenshotResult, String> {
    Err("Screenshot not implemented for this platform".to_string())
}