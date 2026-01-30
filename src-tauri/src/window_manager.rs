use tauri::Window;
use tauri::{PhysicalPosition, PhysicalSize};

#[tauri::command]
pub async fn move_window_to_position(window: Window, x: i32, y: i32) -> Result<(), String> {
    let position = PhysicalPosition::new(x, y);
    window.set_position(position).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_window_position(window: Window) -> Result<(i32, i32), String> {
    let position = window.outer_position().map_err(|e| e.to_string())?;
    Ok((position.x, position.y))
}

#[tauri::command]
pub async fn get_window_size(window: Window) -> Result<(u32, u32), String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    Ok((size.width, size.height))
}

#[tauri::command]
pub async fn get_screen_size() -> Result<(u32, u32), String> {
    // Get primary monitor size
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        
        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYSCREEN) as u32;
            return Ok((width, height));
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::CGDisplay;

        let display = CGDisplay::main();
        let width = display.pixels_wide() as u32;
        let height = display.pixels_high() as u32;
        return Ok((width, height));
    }
    
    #[cfg(target_os = "linux")]
    {
        // For Linux, we'll return a default size
        // In a production app, you'd want to query the actual display
        return Ok((1920, 1080));
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    pub name: String,
}

#[tauri::command]
pub async fn get_monitor_layout() -> Result<Vec<MonitorInfo>, String> {
    let mut monitors = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SM_CXVIRTUALSCREEN};
        
        unsafe {
            // For now, just return the primary monitor info
            // EnumDisplayMonitors has complex callback requirements that need more setup
            let width = GetSystemMetrics(SM_CXSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYSCREEN) as u32;
            
            monitors.push(MonitorInfo {
                x: 0,
                y: 0,
                width,
                height,
                is_primary: true,
                name: "Primary".to_string(),
            });
            
            // If there's a secondary monitor, add a basic detection
            // This is a simplified approach - in production you'd use proper enumeration
            let virtual_width = GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32;
            if virtual_width > width {
                monitors.push(MonitorInfo {
                    x: width as i32,
                    y: 0,
                    width: virtual_width - width,
                    height,
                    is_primary: false,
                    name: "Secondary".to_string(),
                });
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Fallback for other platforms
        let (width, height) = get_screen_size().await?;
        monitors.push(MonitorInfo {
            x: 0,
            y: 0,
            width,
            height,
            is_primary: true,
            name: "Primary".to_string(),
        });
    }
    
    Ok(monitors)
}

#[tauri::command]
pub async fn get_virtual_desktop_size() -> Result<(u32, u32), String> {
    // Get full virtual desktop size (all monitors combined)
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN};
        
        unsafe {
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32;
            println!("🖥️ Virtual desktop detected: {}x{}", width, height);
            return Ok((width, height));
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // For macOS, sum up all displays
        use core_graphics::display::{CGDisplay, CGDisplayBounds};
        
        let displays = CGDisplay::active_displays()
            .map_err(|e| format!("Failed to get displays: {:?}", e))?;
        
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for display in displays {
            let bounds = unsafe { CGDisplayBounds(display) };
            min_x = min_x.min(bounds.origin.x);
            min_y = min_y.min(bounds.origin.y);
            max_x = max_x.max(bounds.origin.x + bounds.size.width);
            max_y = max_y.max(bounds.origin.y + bounds.size.height);
        }
        
        let width = (max_x - min_x) as u32;
        let height = (max_y - min_y) as u32;
        return Ok((width, height));
    }
    
    #[cfg(target_os = "linux")]
    {
        // For Linux, fall back to primary display
        return get_screen_size().await;
    }
}

#[tauri::command]
pub async fn set_window_bounds(window: Window, x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
    let position = PhysicalPosition::new(x, y);
    let size = PhysicalSize::new(width, height);
    
    window.set_position(position).map_err(|e| e.to_string())?;
    window.set_size(size).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // MonitorInfo struct tests
    // ============================================================================

    #[test]
    fn test_monitor_info_negative_position() {
        // Monitors can have negative positions (e.g., monitor to the left of primary)
        let monitor = MonitorInfo {
            x: -1920,
            y: -200,
            width: 1920,
            height: 1080,
            is_primary: false,
            name: "Left Monitor".to_string(),
        };

        assert_eq!(monitor.x, -1920);
        assert_eq!(monitor.y, -200);
    }

    #[test]
    fn test_monitor_info_serialization() {
        let monitor = MonitorInfo {
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            is_primary: true,
            name: "Test".to_string(),
        };

        let json = serde_json::to_string(&monitor).expect("Failed to serialize MonitorInfo");
        assert!(json.contains("\"x\":100"));
        assert!(json.contains("\"y\":200"));
        assert!(json.contains("\"width\":1920"));
        assert!(json.contains("\"height\":1080"));
        assert!(json.contains("\"is_primary\":true"));
        assert!(json.contains("\"name\":\"Test\""));
    }

    #[test]
    fn test_monitor_info_deserialization() {
        let json = r#"{"x":0,"y":0,"width":2560,"height":1440,"is_primary":false,"name":"External"}"#;
        let monitor: MonitorInfo = serde_json::from_str(json).expect("Failed to deserialize MonitorInfo");

        assert_eq!(monitor.x, 0);
        assert_eq!(monitor.y, 0);
        assert_eq!(monitor.width, 2560);
        assert_eq!(monitor.height, 1440);
        assert!(!monitor.is_primary);
        assert_eq!(monitor.name, "External");
    }

    #[test]
    fn test_monitor_info_roundtrip() {
        let original = MonitorInfo {
            x: -500,
            y: 300,
            width: 3840,
            height: 2160,
            is_primary: true,
            name: "4K Display".to_string(),
        };

        let json = serde_json::to_string(&original).expect("Failed to serialize");
        let deserialized: MonitorInfo = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(original.x, deserialized.x);
        assert_eq!(original.y, deserialized.y);
        assert_eq!(original.width, deserialized.width);
        assert_eq!(original.height, deserialized.height);
        assert_eq!(original.is_primary, deserialized.is_primary);
        assert_eq!(original.name, deserialized.name);
    }

    // ============================================================================
    // get_screen_size tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_screen_size_returns_positive_values() {
        let result = get_screen_size().await;
        assert!(result.is_ok(), "get_screen_size should succeed");

        let (width, height) = result.unwrap();
        assert!(width > 0, "Screen width should be positive, got {}", width);
        assert!(height > 0, "Screen height should be positive, got {}", height);
    }

    #[tokio::test]
    async fn test_get_screen_size_reasonable_dimensions() {
        let result = get_screen_size().await;
        assert!(result.is_ok());

        let (width, height) = result.unwrap();

        // Reasonable bounds: at least 640x480, at most 16K (15360x8640)
        assert!(width >= 640, "Width {} is too small (min 640)", width);
        assert!(height >= 480, "Height {} is too small (min 480)", height);
        assert!(width <= 15360, "Width {} is too large (max 15360)", width);
        assert!(height <= 8640, "Height {} is too large (max 8640)", height);
    }

    #[tokio::test]
    async fn test_get_screen_size_consistent() {
        // Calling multiple times should return same values
        let result1 = get_screen_size().await.expect("First call failed");
        let result2 = get_screen_size().await.expect("Second call failed");

        assert_eq!(result1, result2, "Screen size should be consistent across calls");
    }

    // ============================================================================
    // get_virtual_desktop_size tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_virtual_desktop_size_returns_positive_values() {
        let result = get_virtual_desktop_size().await;
        assert!(result.is_ok(), "get_virtual_desktop_size should succeed");

        let (width, height) = result.unwrap();
        assert!(width > 0, "Virtual desktop width should be positive");
        assert!(height > 0, "Virtual desktop height should be positive");
    }

    #[tokio::test]
    async fn test_get_virtual_desktop_size_at_least_screen_size() {
        let screen = get_screen_size().await.expect("get_screen_size failed");
        let virtual_desktop = get_virtual_desktop_size().await.expect("get_virtual_desktop_size failed");

        // Virtual desktop should be at least as large as primary screen
        assert!(
            virtual_desktop.0 >= screen.0,
            "Virtual desktop width {} should be >= screen width {}",
            virtual_desktop.0, screen.0
        );
        assert!(
            virtual_desktop.1 >= screen.1,
            "Virtual desktop height {} should be >= screen height {}",
            virtual_desktop.1, screen.1
        );
    }

    #[tokio::test]
    async fn test_get_virtual_desktop_size_reasonable_bounds() {
        let result = get_virtual_desktop_size().await;
        assert!(result.is_ok());

        let (width, height) = result.unwrap();

        // Virtual desktop could span multiple 8K monitors
        // Max reasonable: 8 monitors wide × 2 tall = 61440 × 17280
        assert!(width <= 61440, "Virtual width {} exceeds reasonable maximum", width);
        assert!(height <= 17280, "Virtual height {} exceeds reasonable maximum", height);
    }

    // ============================================================================
    // get_monitor_layout tests
    // ============================================================================

    #[tokio::test]
    async fn test_get_monitor_layout_returns_at_least_one_monitor() {
        let result = get_monitor_layout().await;
        assert!(result.is_ok(), "get_monitor_layout should succeed");

        let monitors = result.unwrap();
        assert!(!monitors.is_empty(), "Should return at least one monitor");
    }

    #[tokio::test]
    async fn test_get_monitor_layout_has_primary() {
        let result = get_monitor_layout().await;
        assert!(result.is_ok());

        let monitors = result.unwrap();
        let has_primary = monitors.iter().any(|m| m.is_primary);
        assert!(has_primary, "At least one monitor should be marked as primary");
    }

    #[tokio::test]
    async fn test_get_monitor_layout_valid_dimensions() {
        let result = get_monitor_layout().await;
        assert!(result.is_ok());

        let monitors = result.unwrap();
        for monitor in monitors {
            assert!(monitor.width > 0, "Monitor width should be positive");
            assert!(monitor.height > 0, "Monitor height should be positive");
            assert!(!monitor.name.is_empty(), "Monitor should have a name");
        }
    }

    #[tokio::test]
    async fn test_get_monitor_layout_primary_matches_screen_size() {
        let screen = get_screen_size().await.expect("get_screen_size failed");
        let monitors = get_monitor_layout().await.expect("get_monitor_layout failed");

        let primary = monitors.iter().find(|m| m.is_primary).expect("No primary monitor");

        assert_eq!(
            primary.width, screen.0,
            "Primary monitor width {} should match screen width {}",
            primary.width, screen.0
        );
        assert_eq!(
            primary.height, screen.1,
            "Primary monitor height {} should match screen height {}",
            primary.height, screen.1
        );
    }

    #[tokio::test]
    async fn test_get_monitor_layout_consistent() {
        let result1 = get_monitor_layout().await.expect("First call failed");
        let result2 = get_monitor_layout().await.expect("Second call failed");

        assert_eq!(result1.len(), result2.len(), "Monitor count should be consistent");

        for (m1, m2) in result1.iter().zip(result2.iter()) {
            assert_eq!(m1.width, m2.width);
            assert_eq!(m1.height, m2.height);
            assert_eq!(m1.x, m2.x);
            assert_eq!(m1.y, m2.y);
        }
    }

    // ============================================================================
    // Window-dependent command tests (require Tauri app context)
    // These are marked #[ignore] as they require a running Tauri application
    // ============================================================================

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_move_window_to_position_basic() {
        // Test: move_window_to_position sets window to specified coordinates
        //
        // Expected behavior:
        // - Window moves to (100, 200)
        // - get_window_position returns (100, 200)
        //
        // Implementation when Tauri test harness available:
        // let window = create_test_window();
        // move_window_to_position(window.clone(), 100, 200).await.unwrap();
        // let (x, y) = get_window_position(window).await.unwrap();
        // assert_eq!((x, y), (100, 200));
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_move_window_to_position_negative_coords() {
        // Test: move_window_to_position handles negative coordinates
        // Windows can have negative positions on multi-monitor setups
        //
        // Expected behavior:
        // - Window moves to (-100, -50)
        // - No error is returned
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_move_window_to_position_large_coords() {
        // Test: move_window_to_position handles large coordinates
        //
        // Expected behavior:
        // - Window moves to (10000, 5000)
        // - No error (window may be off-screen but still valid)
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_move_window_to_position_origin() {
        // Test: move_window_to_position moves window to origin
        //
        // Expected behavior:
        // - Window moves to (0, 0)
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_get_window_position_initial() {
        // Test: get_window_position returns current position
        //
        // Expected behavior:
        // - Returns valid (x, y) tuple
        // - Values are integers (can be negative on multi-monitor)
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_get_window_size_initial() {
        // Test: get_window_size returns current size
        //
        // Expected behavior:
        // - Returns valid (width, height) tuple
        // - Both values are positive u32
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_set_window_bounds_basic() {
        // Test: set_window_bounds sets position and size
        //
        // Expected behavior:
        // - Window moves to (50, 100)
        // - Window resizes to (800, 600)
        // - get_window_position returns (50, 100)
        // - get_window_size returns (800, 600)
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_set_window_bounds_minimum_size() {
        // Test: set_window_bounds with minimum size
        //
        // Expected behavior:
        // - Window may enforce minimum size
        // - No crash with small values like (1, 1)
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_set_window_bounds_large_size() {
        // Test: set_window_bounds with large size
        //
        // Expected behavior:
        // - Window resizes (may be constrained by screen)
        // - No error returned
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_window_position_after_bounds_change() {
        // Test: position remains after size change via set_window_bounds
        //
        // Expected behavior:
        // - Move window to (200, 300)
        // - Change size with set_window_bounds keeping same position
        // - Position should still be (200, 300)
        panic!("Test requires Tauri app context");
    }

    #[tokio::test]
    #[ignore = "Requires Tauri app context with Window"]
    async fn test_window_size_after_position_change() {
        // Test: size remains after position change
        //
        // Expected behavior:
        // - Set window to size (500, 400)
        // - Move window with move_window_to_position
        // - Size should still be (500, 400)
        panic!("Test requires Tauri app context");
    }

    // ============================================================================
    // Boundary condition tests
    // ============================================================================

    #[test]
    fn test_monitor_info_zero_dimensions() {
        // While unusual, MonitorInfo struct should handle zero dimensions
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            is_primary: false,
            name: "Invalid".to_string(),
        };

        assert_eq!(monitor.width, 0);
        assert_eq!(monitor.height, 0);
    }

    #[test]
    fn test_monitor_info_max_values() {
        // Test with maximum i32/u32 values
        let monitor = MonitorInfo {
            x: i32::MAX,
            y: i32::MIN,
            width: u32::MAX,
            height: u32::MAX,
            is_primary: true,
            name: "Extreme".to_string(),
        };

        assert_eq!(monitor.x, i32::MAX);
        assert_eq!(monitor.y, i32::MIN);
        assert_eq!(monitor.width, u32::MAX);
        assert_eq!(monitor.height, u32::MAX);
    }

    #[test]
    fn test_monitor_info_empty_name() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: true,
            name: String::new(),
        };

        assert!(monitor.name.is_empty());
    }

    #[test]
    fn test_monitor_info_unicode_name() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: true,
            name: "Écran Principal 显示器 🖥️".to_string(),
        };

        assert!(monitor.name.contains("Écran"));
        assert!(monitor.name.contains("显示器"));
        assert!(monitor.name.contains("🖥️"));
    }

    #[test]
    fn test_monitor_info_serialization_with_special_chars() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: true,
            name: "Test \"quotes\" and \\backslash".to_string(),
        };

        let json = serde_json::to_string(&monitor).expect("Failed to serialize");
        let deserialized: MonitorInfo = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(monitor.name, deserialized.name);
    }

    // ============================================================================
    // Error handling tests
    // ============================================================================

    #[test]
    fn test_monitor_info_deserialization_missing_field() {
        // Missing required field should fail
        let json = r#"{"x":0,"y":0,"width":1920,"height":1080,"is_primary":true}"#;
        let result: Result<MonitorInfo, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Deserialization should fail when 'name' is missing");
    }

    #[test]
    fn test_monitor_info_deserialization_wrong_type() {
        // Wrong type for field should fail
        let json = r#"{"x":"not a number","y":0,"width":1920,"height":1080,"is_primary":true,"name":"Test"}"#;
        let result: Result<MonitorInfo, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Deserialization should fail with wrong type");
    }

    #[test]
    fn test_monitor_info_deserialization_null_name() {
        // null for string should fail
        let json = r#"{"x":0,"y":0,"width":1920,"height":1080,"is_primary":true,"name":null}"#;
        let result: Result<MonitorInfo, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Deserialization should fail with null name");
    }

    // ============================================================================
    // Multi-monitor scenario tests
    // ============================================================================

    #[test]
    fn test_monitor_layout_side_by_side() {
        // Simulate side-by-side monitor layout
        let monitors = vec![
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                is_primary: true,
                name: "Primary".to_string(),
            },
            MonitorInfo {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
                is_primary: false,
                name: "Secondary".to_string(),
            },
        ];

        // Calculate total virtual desktop width
        let total_width = monitors.iter().map(|m| m.width).sum::<u32>();
        let max_height = monitors.iter().map(|m| m.height).max().unwrap_or(0);

        // Verify layout calculations
        assert_eq!(total_width, 4480); // 1920 + 2560
        assert_eq!(max_height, 1440);

        // Verify non-overlapping horizontally
        let second = &monitors[1];
        let first = &monitors[0];
        assert_eq!(second.x as u32, first.width);
    }

    #[test]
    fn test_monitor_layout_stacked() {
        // Simulate vertically stacked monitors
        let monitors = vec![
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                is_primary: true,
                name: "Top".to_string(),
            },
            MonitorInfo {
                x: 0,
                y: 1080,
                width: 1920,
                height: 1080,
                is_primary: false,
                name: "Bottom".to_string(),
            },
        ];

        let max_width = monitors.iter().map(|m| m.width).max().unwrap_or(0);
        let total_height = monitors.iter().map(|m| m.height).sum::<u32>();

        assert_eq!(max_width, 1920);
        assert_eq!(total_height, 2160); // 1080 + 1080
    }

    #[test]
    fn test_monitor_layout_with_offset() {
        // Monitor to the left of primary (negative x)
        let monitors = vec![
            MonitorInfo {
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
                is_primary: false,
                name: "Left".to_string(),
            },
            MonitorInfo {
                x: 0,
                y: 0,
                width: 2560,
                height: 1440,
                is_primary: true,
                name: "Primary".to_string(),
            },
        ];

        // Find bounds
        let min_x = monitors.iter().map(|m| m.x).min().unwrap_or(0);
        let max_x = monitors.iter().map(|m| m.x + m.width as i32).max().unwrap_or(0);

        assert_eq!(min_x, -1920);
        assert_eq!(max_x, 2560);
        assert_eq!((max_x - min_x) as u32, 4480);
    }
} 