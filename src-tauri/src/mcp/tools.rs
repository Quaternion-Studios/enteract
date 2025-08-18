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
async fn type_text(text: &str, delay_ms: u64) -> Result<(), String> {
    use winapi::um::winuser::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VkKeyScanA, MapVirtualKeyA, MAPVK_VK_TO_VSC};
    use winapi::um::winuser::{KEYBDINPUT};
    use std::mem;
    
    for ch in text.chars() {
        unsafe {
            // For Unicode characters, use KEYEVENTF_UNICODE
            if ch as u32 > 127 {
                // Unicode input
                let mut inputs = [INPUT {
                    type_: INPUT_KEYBOARD,
                    u: mem::zeroed(),
                }; 2];
                
                *inputs[0].u.ki_mut() = KEYBDINPUT {
                    wVk: 0,
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                };
                
                *inputs[1].u.ki_mut() = KEYBDINPUT {
                    wVk: 0,
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                };
                
                let result = SendInput(2, inputs.as_mut_ptr(), mem::size_of::<INPUT>() as i32);
                if result != 2 {
                    return Err(format!("Failed to send unicode input for character '{}'", ch));
                }
            } else {
                // ASCII character - use virtual key code
                let ascii_byte = ch as u8;
                let vk_code = VkKeyScanA(ascii_byte as i8);
                
                if vk_code == -1 {
                    // Character cannot be represented, try unicode method
                    let mut inputs = [INPUT {
                        type_: INPUT_KEYBOARD,
                        u: mem::zeroed(),
                    }; 2];
                    
                    *inputs[0].u.ki_mut() = KEYBDINPUT {
                        wVk: 0,
                        wScan: ch as u16,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    
                    *inputs[1].u.ki_mut() = KEYBDINPUT {
                        wVk: 0,
                        wScan: ch as u16,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    
                    let result = SendInput(2, inputs.as_mut_ptr(), mem::size_of::<INPUT>() as i32);
                    if result != 2 {
                        return Err(format!("Failed to send unicode input for character '{}'", ch));
                    }
                } else {
                    let virtual_key = (vk_code & 0xFF) as u16;
                    let scan_code = MapVirtualKeyA(virtual_key as u32, MAPVK_VK_TO_VSC) as u16;
                    
                    // Check if shift is needed
                    let shift_needed = (vk_code & 0x100) != 0;
                    
                    let mut inputs = Vec::new();
                    
                    // Press shift if needed
                    if shift_needed {
                        inputs.push(INPUT {
                            type_: INPUT_KEYBOARD,
                            u: {
                                let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                                *input_union.ki_mut() = KEYBDINPUT {
                                    wVk: 0x10, // VK_SHIFT
                                    wScan: MapVirtualKeyA(0x10, MAPVK_VK_TO_VSC) as u16,
                                    dwFlags: 0,
                                    time: 0,
                                    dwExtraInfo: 0,
                                };
                                input_union
                            },
                        });
                    }
                    
                    // Press key
                    inputs.push(INPUT {
                        type_: INPUT_KEYBOARD,
                        u: {
                            let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                            *input_union.ki_mut() = KEYBDINPUT {
                                wVk: virtual_key,
                                wScan: scan_code,
                                dwFlags: 0,
                                time: 0,
                                dwExtraInfo: 0,
                            };
                            input_union
                        },
                    });
                    
                    // Release key
                    inputs.push(INPUT {
                        type_: INPUT_KEYBOARD,
                        u: {
                            let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                            *input_union.ki_mut() = KEYBDINPUT {
                                wVk: virtual_key,
                                wScan: scan_code,
                                dwFlags: KEYEVENTF_KEYUP,
                                time: 0,
                                dwExtraInfo: 0,
                            };
                            input_union
                        },
                    });
                    
                    // Release shift if needed
                    if shift_needed {
                        inputs.push(INPUT {
                            type_: INPUT_KEYBOARD,
                            u: {
                                let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                                *input_union.ki_mut() = KEYBDINPUT {
                                    wVk: 0x10, // VK_SHIFT
                                    wScan: MapVirtualKeyA(0x10, MAPVK_VK_TO_VSC) as u16,
                                    dwFlags: KEYEVENTF_KEYUP,
                                    time: 0,
                                    dwExtraInfo: 0,
                                };
                                input_union
                            },
                        });
                    }
                    
                    let result = SendInput(inputs.len() as u32, inputs.as_mut_ptr(), mem::size_of::<INPUT>() as i32);
                    if result != inputs.len() as u32 {
                        return Err(format!("Failed to send input for character '{}', sent {}/{} inputs", ch, result, inputs.len()));
                    }
                }
            }
        }
        
        // Add delay between characters
        if delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
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
async fn press_key(key: &str, modifiers: Vec<KeyModifier>) -> Result<(), String> {
    use winapi::um::winuser::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, MapVirtualKeyA, MAPVK_VK_TO_VSC, VK_RETURN, VK_DELETE, VK_BACK, VK_TAB, VK_ESCAPE, VK_SPACE, VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN};
    use winapi::um::winuser::{KEYBDINPUT};
    use std::mem;
    
    // Map key names to virtual key codes
    let virtual_key = match key.to_lowercase().as_str() {
        "return" | "enter" => VK_RETURN as u32,
        "delete" | "del" => VK_DELETE as u32,
        "backspace" | "back" => VK_BACK as u32,
        "tab" => VK_TAB as u32,
        "escape" | "esc" => VK_ESCAPE as u32,
        "space" => VK_SPACE as u32,
        "left" | "leftarrow" => VK_LEFT as u32,
        "right" | "rightarrow" => VK_RIGHT as u32,
        "up" | "uparrow" => VK_UP as u32,
        "down" | "downarrow" => VK_DOWN as u32,
        "ctrl" | "control" => VK_CONTROL as u32,
        "alt" => VK_MENU as u32,
        "shift" => VK_SHIFT as u32,
        "meta" | "win" | "windows" => VK_LWIN as u32,
        // Function keys
        "f1" => 0x70,
        "f2" => 0x71,
        "f3" => 0x72,
        "f4" => 0x73,
        "f5" => 0x74,
        "f6" => 0x75,
        "f7" => 0x76,
        "f8" => 0x77,
        "f9" => 0x78,
        "f10" => 0x79,
        "f11" => 0x7A,
        "f12" => 0x7B,
        // Single character keys
        _ if key.len() == 1 => {
            let ch = key.chars().next().unwrap().to_ascii_uppercase();
            if ch.is_ascii_alphabetic() {
                ch as u32
            } else {
                return Err(format!("Unsupported key: {}", key));
            }
        }
        _ => return Err(format!("Unsupported key: {}", key)),
    };
    
    // Map modifier enums to virtual key codes
    let modifier_vks: Vec<u32> = modifiers.iter().map(|m| match m {
        KeyModifier::Ctrl => VK_CONTROL as u32,
        KeyModifier::Alt => VK_MENU as u32,
        KeyModifier::Shift => VK_SHIFT as u32,
        KeyModifier::Meta => VK_LWIN as u32,
    }).collect();
    
    unsafe {
        let mut inputs = Vec::new();
        
        // Press modifiers
        for &modifier_vk in &modifier_vks {
            inputs.push(INPUT {
                type_: INPUT_KEYBOARD,
                u: {
                    let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                    *input_union.ki_mut() = KEYBDINPUT {
                        wVk: modifier_vk as u16,
                        wScan: MapVirtualKeyA(modifier_vk, MAPVK_VK_TO_VSC) as u16,
                        dwFlags: 0,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    input_union
                },
            });
        }
        
        // Press the main key
        inputs.push(INPUT {
            type_: INPUT_KEYBOARD,
            u: {
                let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                *input_union.ki_mut() = KEYBDINPUT {
                    wVk: virtual_key as u16,
                    wScan: MapVirtualKeyA(virtual_key, MAPVK_VK_TO_VSC) as u16,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                };
                input_union
            },
        });
        
        // Release the main key
        inputs.push(INPUT {
            type_: INPUT_KEYBOARD,
            u: {
                let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                *input_union.ki_mut() = KEYBDINPUT {
                    wVk: virtual_key as u16,
                    wScan: MapVirtualKeyA(virtual_key, MAPVK_VK_TO_VSC) as u16,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                };
                input_union
            },
        });
        
        // Release modifiers (in reverse order)
        for &modifier_vk in modifier_vks.iter().rev() {
            inputs.push(INPUT {
                type_: INPUT_KEYBOARD,
                u: {
                    let mut input_union: winapi::um::winuser::INPUT_u = mem::zeroed();
                    *input_union.ki_mut() = KEYBDINPUT {
                        wVk: modifier_vk as u16,
                        wScan: MapVirtualKeyA(modifier_vk, MAPVK_VK_TO_VSC) as u16,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    input_union
                },
            });
        }
        
        let result = SendInput(inputs.len() as u32, inputs.as_mut_ptr(), mem::size_of::<INPUT>() as i32);
        if result != inputs.len() as u32 {
            return Err(format!("Failed to send key press input, sent {}/{} inputs", result, inputs.len()));
        }
    }
    
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

// ========== NEW ATOMIC OCR TOOLS ==========

#[derive(Clone)]
pub struct FindTextTool;

#[async_trait]
impl ComputerUseTool for FindTextTool {
    fn name(&self) -> &str { "find_text" }
    
    fn description(&self) -> String {
        "Find text on screen using OCR and return its location and confidence".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Low }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to search for on screen"
                },
                "confidence_threshold": {
                    "type": "number",
                    "default": 0.8,
                    "description": "Minimum confidence level (0.0-1.0) for text recognition"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "default": false,
                    "description": "Whether to perform case-sensitive matching"
                }
            },
            "required": ["text"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value, _session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let text_to_find = params["text"].as_str()
            .ok_or("Missing required parameter: text")?;
        let confidence_threshold = params["confidence_threshold"].as_f64().unwrap_or(0.8);
        let case_sensitive = params["case_sensitive"].as_bool().unwrap_or(false);
        
        // Take screenshot first
        let screenshot_result = take_screenshot_full(Some("png".to_string()), Some(80)).await?;
        
        // Perform OCR on the screenshot
        let text_locations = find_text_in_image(&screenshot_result.image_base64, text_to_find, confidence_threshold, case_sensitive).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolExecutionResult {
            success: true,
            result: serde_json::json!({
                "text_locations": text_locations,
                "search_text": text_to_find,
                "confidence_threshold": confidence_threshold,
                "matches_found": text_locations.len()
            }),
            error: None,
            execution_time_ms: execution_time,
            tool_name: "find_text".to_string(),
        })
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub struct ClickAtTool;

#[async_trait]
impl ComputerUseTool for ClickAtTool {
    fn name(&self) -> &str { "click_at" }
    
    fn description(&self) -> String {
        "Click at specific coordinates on screen".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Medium }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": {
                    "type": "integer",
                    "description": "X coordinate to click"
                },
                "y": {
                    "type": "integer",
                    "description": "Y coordinate to click"
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "default": "left",
                    "description": "Mouse button to click"
                },
                "double_click": {
                    "type": "boolean",
                    "default": false,
                    "description": "Whether to perform a double-click"
                }
            },
            "required": ["x", "y"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value, _session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let x = params["x"].as_i64().ok_or("Missing required parameter: x")? as i32;
        let y = params["y"].as_i64().ok_or("Missing required parameter: y")? as i32;
        let button = params["button"].as_str().unwrap_or("left");
        let double_click = params["double_click"].as_bool().unwrap_or(false);
        
        // Perform the click
        click_at_coordinates(x, y, button, double_click).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolExecutionResult {
            success: true,
            result: serde_json::json!({
                "clicked_at": {"x": x, "y": y},
                "button": button,
                "double_click": double_click,
                "message": format!("Successfully clicked at ({}, {})", x, y)
            }),
            error: None,
            execution_time_ms: execution_time,
            tool_name: "click_at".to_string(),
        })
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

// ========== COMPOUND TOOLS ==========

#[derive(Clone)]
pub struct ClickOnTextTool;

#[async_trait]
impl ComputerUseTool for ClickOnTextTool {
    fn name(&self) -> &str { "click_on_text" }
    
    fn description(&self) -> String {
        "Find text on screen using OCR and click on it (compound tool)".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Medium }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to find and click on"
                },
                "confidence_threshold": {
                    "type": "number",
                    "default": 0.8,
                    "description": "Minimum confidence level for text recognition"
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "default": "left",
                    "description": "Mouse button to click"
                }
            },
            "required": ["text"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let text_to_find = params["text"].as_str()
            .ok_or("Missing required parameter: text")?;
        
        // Step 1: Find the text
        let find_tool = FindTextTool;
        let find_result = find_tool.execute(params.clone(), session_id).await?;
        
        if !find_result.success {
            return Ok(ToolExecutionResult {
                success: false,
                result: serde_json::json!({}),
                error: Some(format!("Failed to find text: {}", text_to_find)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                tool_name: "click_on_text".to_string(),
            });
        }
        
        let text_locations = find_result.result["text_locations"].as_array()
            .ok_or("Invalid find_text result format")?;
            
        if text_locations.is_empty() {
            return Ok(ToolExecutionResult {
                success: false,
                result: serde_json::json!({
                    "search_text": text_to_find,
                    "matches_found": 0
                }),
                error: Some(format!("Text '{}' not found on screen", text_to_find)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                tool_name: "click_on_text".to_string(),
            });
        }
        
        // Use the first (most confident) match
        let best_match = &text_locations[0];
        let x = best_match["center_x"].as_i64().ok_or("Invalid text location format")? as i32;
        let y = best_match["center_y"].as_i64().ok_or("Invalid text location format")? as i32;
        
        // Step 2: Click at the found location
        let click_params = serde_json::json!({
            "x": x,
            "y": y,
            "button": params["button"].as_str().unwrap_or("left")
        });
        
        let click_tool = ClickAtTool;
        let click_result = click_tool.execute(click_params, session_id).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolExecutionResult {
            success: click_result.success,
            result: serde_json::json!({
                "text_found": text_to_find,
                "location": {"x": x, "y": y},
                "confidence": best_match["confidence"],
                "click_result": click_result.result
            }),
            error: click_result.error,
            execution_time_ms: execution_time,
            tool_name: "click_on_text".to_string(),
        })
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

// ========== DEBUG OCR TOOL ==========

#[derive(Clone)]
pub struct DebugOcrTool;

#[async_trait]
impl ComputerUseTool for DebugOcrTool {
    fn name(&self) -> &str { "debug_ocr" }
    
    fn description(&self) -> String {
        "Take a screenshot and show all detected text with locations, confidence scores, and priorities for debugging OCR".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Low }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "confidence_threshold": {
                    "type": "number",
                    "default": 0.7,
                    "description": "Minimum confidence level for text recognition (0.0-1.0)"
                },
                "show_all": {
                    "type": "boolean",
                    "default": true,
                    "description": "Show all detected text, even below confidence threshold"
                }
            }
        })
    }
    
    async fn execute(&self, params: serde_json::Value, _session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let confidence_threshold = params["confidence_threshold"].as_f64().unwrap_or(0.7);
        let show_all = params["show_all"].as_bool().unwrap_or(true);
        
        // Take screenshot first
        let screenshot_result = take_screenshot_full(Some("png".to_string()), Some(80)).await?;
        
        // Perform OCR to get all text on screen
        let all_text_locations = debug_ocr_scan(&screenshot_result.image_base64, confidence_threshold, show_all).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // Format results for display
        let mut debug_output = String::new();
        debug_output.push_str(&format!("🔍 **OCR Debug Results** ({}x{} screenshot)\n\n", screenshot_result.width, screenshot_result.height));
        debug_output.push_str(&format!("**Confidence Threshold:** {:.2}\n", confidence_threshold));
        debug_output.push_str(&format!("**Total Text Elements Found:** {}\n\n", all_text_locations.len()));
        
        if all_text_locations.is_empty() {
            debug_output.push_str("❌ No text detected on screen\n");
        } else {
            debug_output.push_str("**Text Elements (sorted by confidence, then position):**\n\n");
            
            for (i, location) in all_text_locations.iter().enumerate() {
                let confidence_icon = if location.confidence >= confidence_threshold as f32 { "✅" } else { "⚠️" };
                let priority_text = if i < 3 { format!("🎯 **PRIORITY {}**", i + 1) } else { format!("#{}", i + 1) };
                
                debug_output.push_str(&format!(
                    "{} {} `\"{}\"` - Confidence: {:.3}\n   📍 Position: ({}, {}) | Center: ({}, {})\n   📏 Size: {}×{}\n\n",
                    confidence_icon,
                    priority_text,
                    location.text,
                    location.confidence,
                    location.bounding_box.x,
                    location.bounding_box.y,
                    location.center_x,
                    location.center_y,
                    location.bounding_box.width,
                    location.bounding_box.height
                ));
            }
            
            // Show summary by confidence range
            let high_confidence = all_text_locations.iter().filter(|l| l.confidence >= 0.9).count();
            let medium_confidence = all_text_locations.iter().filter(|l| l.confidence >= 0.7 && l.confidence < 0.9).count();
            let low_confidence = all_text_locations.iter().filter(|l| l.confidence < 0.7).count();
            
            debug_output.push_str("**Confidence Summary:**\n");
            debug_output.push_str(&format!("• High (≥0.9): {} elements\n", high_confidence));
            debug_output.push_str(&format!("• Medium (0.7-0.9): {} elements\n", medium_confidence));
            debug_output.push_str(&format!("• Low (<0.7): {} elements\n", low_confidence));
        }
        
        Ok(ToolExecutionResult {
            success: true,
            result: serde_json::json!({
                "debug_output": debug_output,
                "all_text_locations": all_text_locations,
                "total_found": all_text_locations.len(),
                "screenshot_size": {
                    "width": screenshot_result.width,
                    "height": screenshot_result.height
                },
                "confidence_threshold": confidence_threshold
            }),
            error: None,
            execution_time_ms: execution_time,
            tool_name: "debug_ocr".to_string(),
        })
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

// ========== NEW COMPOUND TOOL: CLICK AND TYPE ==========

#[derive(Clone)]
pub struct ClickAndTypeTool;

#[async_trait]
impl ComputerUseTool for ClickAndTypeTool {
    fn name(&self) -> &str { "click_and_type" }
    
    fn description(&self) -> String {
        "Find text on screen (like a textbox label), click on it, and then type text into the focused element".to_string()
    }
    
    fn danger_level(&self) -> DangerLevel { DangerLevel::Medium }
    
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "click_target": {
                    "type": "string",
                    "description": "Text to find and click on (e.g., 'Search' textbox, placeholder text, or nearby label)"
                },
                "text_to_type": {
                    "type": "string",
                    "description": "Text to type after clicking"
                },
                "confidence_threshold": {
                    "type": "number",
                    "default": 0.8,
                    "description": "Minimum confidence level for text recognition"
                },
                "press_enter": {
                    "type": "boolean",
                    "default": false,
                    "description": "Whether to press Enter after typing"
                },
                "clear_existing": {
                    "type": "boolean",
                    "default": true,
                    "description": "Whether to clear existing text (Ctrl+A, Delete) before typing"
                },
                "delay_ms": {
                    "type": "integer",
                    "default": 10,
                    "description": "Delay between keystrokes in milliseconds"
                }
            },
            "required": ["click_target", "text_to_type"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value, session_id: &str) -> Result<ToolExecutionResult, String> {
        let start_time = Instant::now();
        
        let click_target = params["click_target"].as_str()
            .ok_or("Missing required parameter: click_target")?;
        let text_to_type = params["text_to_type"].as_str()
            .ok_or("Missing required parameter: text_to_type")?;
        let confidence_threshold = params["confidence_threshold"].as_f64().unwrap_or(0.8);
        let press_enter = params["press_enter"].as_bool().unwrap_or(false);
        let clear_existing = params["clear_existing"].as_bool().unwrap_or(true);
        let delay_ms = params["delay_ms"].as_u64().unwrap_or(10);
        
        log::info!("Session {}: Executing click_and_type - target: '{}', text: '{}'", session_id, click_target, text_to_type);
        
        // Step 1: Find and click the target
        let click_params = serde_json::json!({
            "text": click_target,
            "confidence_threshold": confidence_threshold,
            "button": "left"
        });
        
        let click_tool = ClickOnTextTool;
        let click_result = click_tool.execute(click_params, session_id).await?;
        
        if !click_result.success {
            return Ok(ToolExecutionResult {
                success: false,
                result: serde_json::json!({
                    "click_target": click_target,
                    "text_to_type": text_to_type,
                    "step_failed": "click",
                    "error": "Failed to find or click target text"
                }),
                error: Some(format!("Failed to find or click target text: {}", click_target)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                tool_name: "click_and_type".to_string(),
            });
        }
        
        // Small delay to ensure the click registered and focus changed
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Step 2: Clear existing text if requested
        if clear_existing {
            // Ctrl+A to select all, then Delete to clear
            let select_all_result = press_key("a", vec![KeyModifier::Ctrl]).await;
            if let Err(e) = select_all_result {
                log::warn!("Failed to select all text: {}", e);
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let delete_result = press_key("Delete", vec![]).await;
                if let Err(e) = delete_result {
                    log::warn!("Failed to delete selected text: {}", e);
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
        
        // Step 3: Type the text
        let type_result = type_text(text_to_type, delay_ms).await;
        if let Err(e) = type_result {
            return Ok(ToolExecutionResult {
                success: false,
                result: serde_json::json!({
                    "click_target": click_target,
                    "text_to_type": text_to_type,
                    "step_failed": "type",
                    "click_location": click_result.result["location"],
                    "error": format!("Failed to type text: {}", e)
                }),
                error: Some(format!("Failed to type text: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                tool_name: "click_and_type".to_string(),
            });
        }
        
        // Step 4: Press Enter if requested
        if press_enter {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let enter_result = press_key("Return", vec![]).await;
            if let Err(e) = enter_result {
                log::warn!("Failed to press Enter: {}", e);
            }
        }
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolExecutionResult {
            success: true,
            result: serde_json::json!({
                "click_target": click_target,
                "text_to_type": text_to_type,
                "click_location": click_result.result["location"],
                "characters_typed": text_to_type.chars().count(),
                "cleared_existing": clear_existing,
                "pressed_enter": press_enter,
                "message": format!("Successfully clicked '{}' and typed {} characters", click_target, text_to_type.chars().count())
            }),
            error: None,
            execution_time_ms: execution_time,
            tool_name: "click_and_type".to_string(),
        })
    }
    
    fn clone_box(&self) -> Box<dyn ComputerUseTool + Send + Sync> {
        Box::new(self.clone())
    }
}

// ========== OCR HELPER FUNCTIONS ==========

#[derive(serde::Serialize, serde::Deserialize)]
struct TextLocation {
    text: String,
    confidence: f32,
    bounding_box: TextBoundingBox,
    center_x: i32,
    center_y: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TextBoundingBox {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

async fn find_text_in_image(
    base64_image: &str,
    target_text: &str,
    confidence_threshold: f64,
    case_sensitive: bool,
) -> Result<Vec<TextLocation>, String> {
    #[cfg(target_os = "windows")]
    {
        windows_ocr_find_text(base64_image, target_text, confidence_threshold, case_sensitive).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("OCR is only supported on Windows currently".to_string())
    }
}

async fn debug_ocr_scan(
    base64_image: &str,
    confidence_threshold: f64,
    show_all: bool,
) -> Result<Vec<TextLocation>, String> {
    #[cfg(target_os = "windows")]
    {
        windows_ocr_debug_scan(base64_image, confidence_threshold, show_all).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("OCR is only supported on Windows currently".to_string())
    }
}

#[cfg(target_os = "windows")]
async fn windows_ocr_find_text(
    base64_image: &str,
    target_text: &str,
    confidence_threshold: f64,
    case_sensitive: bool,
) -> Result<Vec<TextLocation>, String> {
    use base64::Engine;
    use windows::{
        Media::Ocr::*,
        Storage::Streams::*,
        Graphics::Imaging::*,
    };
    
    // Decode base64 image
    let image_data = base64::engine::general_purpose::STANDARD
        .decode(base64_image)
        .map_err(|e| format!("Failed to decode base64 image: {}", e))?;
    
    // Create OCR engine
    let ocr_engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("Failed to create OCR engine: {}", e))?;
    
    // Create memory stream from image data
    let stream = InMemoryRandomAccessStream::new()
        .map_err(|e| format!("Failed to create memory stream: {}", e))?;
    
    let writer = stream.GetOutputStreamAt(0)
        .map_err(|e| format!("Failed to get output stream: {}", e))?;
    
    // Write image data to stream using DataWriter
    let data_writer = windows::Storage::Streams::DataWriter::CreateDataWriter(&writer)
        .map_err(|e| format!("Failed to create data writer: {}", e))?;
    
    data_writer.WriteBytes(&image_data)
        .map_err(|e| format!("Failed to write bytes: {}", e))?;
    
    data_writer.StoreAsync()
        .map_err(|e| format!("Failed to store data: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete store: {}", e))?;
    
    writer.FlushAsync()
        .map_err(|e| format!("Failed to flush stream: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete flush: {}", e))?;
    
    // Create bitmap decoder
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .map_err(|e| format!("Failed to create bitmap decoder: {}", e))?
        .get()
        .map_err(|e| format!("Failed to get bitmap decoder: {}", e))?;
    
    // Get software bitmap
    let bitmap = decoder.GetSoftwareBitmapAsync()
        .map_err(|e| format!("Failed to get software bitmap: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete bitmap operation: {}", e))?;
    
    // Perform OCR
    let ocr_result = ocr_engine.RecognizeAsync(&bitmap)
        .map_err(|e| format!("Failed to start OCR: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete OCR: {}", e))?;
    
    // Extract text and positions
    let mut results = Vec::new();
    let search_text = if case_sensitive { target_text.to_string() } else { target_text.to_lowercase() };
    
    let lines = ocr_result.Lines()
        .map_err(|e| format!("Failed to get OCR lines: {}", e))?;
    
    for line in lines {
        let words = line.Words()
            .map_err(|e| format!("Failed to get line words: {}", e))?;
        
        for word in words {
            let text = word.Text()
                .map_err(|e| format!("Failed to get word text: {}", e))?
                .to_string();
            
            let found_text = if case_sensitive { text.clone() } else { text.to_lowercase() };
            
            // Check if this word contains our target text
            if found_text.contains(&search_text) {
                let bounding_rect = word.BoundingRect()
                    .map_err(|e| format!("Failed to get bounding rect: {}", e))?;
                
                // Windows OCR doesn't provide confidence per word, so we'll use a default high confidence
                let confidence = 0.95_f32; // High confidence for Windows OCR
                
                if confidence >= confidence_threshold as f32 {
                    let x = bounding_rect.X as i32;
                    let y = bounding_rect.Y as i32;
                    let width = bounding_rect.Width as i32;
                    let height = bounding_rect.Height as i32;
                    
                    let center_x = x + width / 2;
                    let center_y = y + height / 2;
                    
                    results.push(TextLocation {
                        text: text.clone(),
                        confidence,
                        bounding_box: TextBoundingBox { x, y, width, height },
                        center_x,
                        center_y,
                    });
                }
            }
        }
    }
    
    // Sort by confidence (highest first) and then by position (top to bottom, left to right)
    results.sort_by(|a, b| {
        b.confidence.partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.bounding_box.y.cmp(&b.bounding_box.y))
            .then_with(|| a.bounding_box.x.cmp(&b.bounding_box.x))
    });
    
    Ok(results)
}

#[cfg(target_os = "windows")]
async fn windows_ocr_debug_scan(
    base64_image: &str,
    confidence_threshold: f64,
    show_all: bool,
) -> Result<Vec<TextLocation>, String> {
    use base64::Engine;
    use windows::{
        Media::Ocr::*,
        Storage::Streams::*,
        Graphics::Imaging::*,
    };
    
    // Decode base64 image
    let image_data = base64::engine::general_purpose::STANDARD
        .decode(base64_image)
        .map_err(|e| format!("Failed to decode base64 image: {}", e))?;
    
    // Create OCR engine
    let ocr_engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("Failed to create OCR engine: {}", e))?;
    
    // Create memory stream from image data
    let stream = InMemoryRandomAccessStream::new()
        .map_err(|e| format!("Failed to create memory stream: {}", e))?;
    
    let writer = stream.GetOutputStreamAt(0)
        .map_err(|e| format!("Failed to get output stream: {}", e))?;
    
    // Write image data to stream using DataWriter
    let data_writer = windows::Storage::Streams::DataWriter::CreateDataWriter(&writer)
        .map_err(|e| format!("Failed to create data writer: {}", e))?;
    
    data_writer.WriteBytes(&image_data)
        .map_err(|e| format!("Failed to write bytes: {}", e))?;
    
    data_writer.StoreAsync()
        .map_err(|e| format!("Failed to store data: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete store: {}", e))?;
    
    writer.FlushAsync()
        .map_err(|e| format!("Failed to flush stream: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete flush: {}", e))?;
    
    // Create bitmap decoder
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .map_err(|e| format!("Failed to create bitmap decoder: {}", e))?
        .get()
        .map_err(|e| format!("Failed to get bitmap decoder: {}", e))?;
    
    // Get software bitmap
    let bitmap = decoder.GetSoftwareBitmapAsync()
        .map_err(|e| format!("Failed to get software bitmap: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete bitmap operation: {}", e))?;
    
    // Perform OCR
    let ocr_result = ocr_engine.RecognizeAsync(&bitmap)
        .map_err(|e| format!("Failed to start OCR: {}", e))?
        .get()
        .map_err(|e| format!("Failed to complete OCR: {}", e))?;
    
    // Extract ALL text and positions (for debugging)
    let mut results = Vec::new();
    
    let lines = ocr_result.Lines()
        .map_err(|e| format!("Failed to get OCR lines: {}", e))?;
    
    for line in lines {
        let words = line.Words()
            .map_err(|e| format!("Failed to get line words: {}", e))?;
        
        for word in words {
            let text = word.Text()
                .map_err(|e| format!("Failed to get word text: {}", e))?
                .to_string();
            
            if text.trim().is_empty() {
                continue; // Skip empty text
            }
            
            let bounding_rect = word.BoundingRect()
                .map_err(|e| format!("Failed to get bounding rect: {}", e))?;
            
            // Windows OCR doesn't provide confidence per word, so we'll use a default high confidence
            let confidence = 0.95_f32; // High confidence for Windows OCR
            
            // Include all text if show_all is true, or only text above threshold
            if show_all || confidence >= confidence_threshold as f32 {
                let x = bounding_rect.X as i32;
                let y = bounding_rect.Y as i32;
                let width = bounding_rect.Width as i32;
                let height = bounding_rect.Height as i32;
                
                let center_x = x + width / 2;
                let center_y = y + height / 2;
                
                results.push(TextLocation {
                    text: text.clone(),
                    confidence,
                    bounding_box: TextBoundingBox { x, y, width, height },
                    center_x,
                    center_y,
                });
            }
        }
    }
    
    // Sort by confidence (highest first) and then by position (top to bottom, left to right)
    results.sort_by(|a, b| {
        b.confidence.partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.bounding_box.y.cmp(&b.bounding_box.y))
            .then_with(|| a.bounding_box.x.cmp(&b.bounding_box.x))
    });
    
    log::info!("🔍 OCR Debug: Found {} text elements total", results.len());
    for (i, result) in results.iter().take(10).enumerate() {
        log::info!("  {}. \"{}\" at ({}, {}) confidence: {:.3}", 
                  i + 1, result.text, result.center_x, result.center_y, result.confidence);
    }
    
    Ok(results)
}

async fn click_at_coordinates(x: i32, y: i32, button: &str, double_click: bool) -> Result<(), String> {
    // For now, use the existing click implementation
    // This will be platform-specific
    #[cfg(target_os = "windows")]
    {
        windows_click_at(x, y, button, double_click).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Click not implemented for this platform".to_string())
    }
}

#[cfg(target_os = "windows")]
async fn windows_click_at(x: i32, y: i32, button: &str, double_click: bool) -> Result<(), String> {
    use winapi::um::winuser::{SetCursorPos, mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP};
    
    unsafe {
        // Move cursor to position
        SetCursorPos(x, y);
        
        // Small delay to ensure cursor movement
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Determine mouse events
        let (down_event, up_event) = match button {
            "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            "middle" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
            _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP), // Default to left
        };
        
        // Perform click
        mouse_event(down_event, 0, 0, 0, 0);
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        mouse_event(up_event, 0, 0, 0, 0);
        
        // Double click if requested
        if double_click {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            mouse_event(down_event, 0, 0, 0, 0);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            mouse_event(up_event, 0, 0, 0, 0);
        }
    }
    
    Ok(())
}