// Enhanced window drag and resize functionality
use tauri::{Window, LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowState {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_maximized: bool,
    pub is_minimized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DragStartInfo {
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub window_x: f64,
    pub window_y: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeStartInfo {
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub window_x: f64,
    pub window_y: f64,
    pub window_width: f64,
    pub window_height: f64,
    pub resize_direction: ResizeDirection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ResizeDirection {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[tauri::command]
pub async fn start_window_drag(
    window: Window,
    mouse_x: f64,
    mouse_y: f64,
) -> Result<DragStartInfo, String> {
    let position = window.outer_position()
        .map_err(|e| format!("Failed to get window position: {}", e))?;
    
    let (window_x, window_y) = (position.x as f64, position.y as f64);
    
    Ok(DragStartInfo {
        mouse_x,
        mouse_y,
        window_x,
        window_y,
    })
}

#[tauri::command]
pub async fn update_window_drag(
    window: Window,
    mouse_x: f64,
    mouse_y: f64,
    drag_info: DragStartInfo,
) -> Result<(), String> {
    let new_x = drag_info.window_x + (mouse_x - drag_info.mouse_x);
    let new_y = drag_info.window_y + (mouse_y - drag_info.mouse_y);
    
    let new_position = LogicalPosition::new(new_x, new_y);
    
    window.set_position(new_position)
        .map_err(|e| format!("Failed to set window position: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn start_window_resize(
    window: Window,
    mouse_x: f64,
    mouse_y: f64,
    direction: ResizeDirection,
) -> Result<ResizeStartInfo, String> {
    let position = window.outer_position()
        .map_err(|e| format!("Failed to get window position: {}", e))?;
    let size = window.outer_size()
        .map_err(|e| format!("Failed to get window size: {}", e))?;
    
    let (window_x, window_y) = (position.x as f64, position.y as f64);
    let (window_width, window_height) = (size.width as f64, size.height as f64);
    
    Ok(ResizeStartInfo {
        mouse_x,
        mouse_y,
        window_x,
        window_y,
        window_width,
        window_height,
        resize_direction: direction,
    })
}

#[tauri::command]
pub async fn update_window_resize(
    window: Window,
    mouse_x: f64,
    mouse_y: f64,
    resize_info: ResizeStartInfo,
) -> Result<(), String> {
    let delta_x = mouse_x - resize_info.mouse_x;
    let delta_y = mouse_y - resize_info.mouse_y;
    
    let (new_x, new_y, new_width, new_height) = calculate_new_window_bounds(
        resize_info.window_x,
        resize_info.window_y,
        resize_info.window_width,
        resize_info.window_height,
        delta_x,
        delta_y,
        &resize_info.resize_direction,
    );
    
    // Get window constraints from config
    let min_width = 320.0;
    let min_height = 60.0;
    let max_width = 900.0;
    let max_height = 1600.0;
    
    // Apply constraints
    let constrained_width = new_width.clamp(min_width, max_width);
    let constrained_height = new_height.clamp(min_height, max_height);
    
    // Adjust position if size was constrained
    let final_x = if constrained_width != new_width {
        // If we're resizing from the left edge and width was constrained, adjust position
        match resize_info.resize_direction {
            ResizeDirection::West | ResizeDirection::NorthWest | ResizeDirection::SouthWest => {
                resize_info.window_x + (resize_info.window_width - constrained_width)
            }
            _ => new_x,
        }
    } else {
        new_x
    };
    
    let final_y = if constrained_height != new_height {
        // If we're resizing from the top edge and height was constrained, adjust position
        match resize_info.resize_direction {
            ResizeDirection::North | ResizeDirection::NorthEast | ResizeDirection::NorthWest => {
                resize_info.window_y + (resize_info.window_height - constrained_height)
            }
            _ => new_y,
        }
    } else {
        new_y
    };
    
    // Set new position and size
    let new_position = LogicalPosition::new(final_x, final_y);
    let new_size = LogicalSize::new(constrained_width, constrained_height);
    
    // Apply position change first (if needed)
    if final_x != resize_info.window_x || final_y != resize_info.window_y {
        window.set_position(new_position)
            .map_err(|e| format!("Failed to set window position during resize: {}", e))?;
    }
    
    // Then apply size change
    window.set_size(new_size)
        .map_err(|e| format!("Failed to set window size: {}", e))?;
    
    Ok(())
}

fn calculate_new_window_bounds(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    delta_x: f64,
    delta_y: f64,
    direction: &ResizeDirection,
) -> (f64, f64, f64, f64) {
    match direction {
        ResizeDirection::North => (x, y + delta_y, width, height - delta_y),
        ResizeDirection::South => (x, y, width, height + delta_y),
        ResizeDirection::East => (x, y, width + delta_x, height),
        ResizeDirection::West => (x + delta_x, y, width - delta_x, height),
        ResizeDirection::NorthEast => (x, y + delta_y, width + delta_x, height - delta_y),
        ResizeDirection::NorthWest => (x + delta_x, y + delta_y, width - delta_x, height - delta_y),
        ResizeDirection::SouthEast => (x, y, width + delta_x, height + delta_y),
        ResizeDirection::SouthWest => (x + delta_x, y, width - delta_x, height + delta_y),
    }
}

#[tauri::command]
pub async fn get_window_state(window: Window) -> Result<WindowState, String> {
    let position = window.outer_position()
        .map_err(|e| format!("Failed to get window position: {}", e))?;
    let size = window.outer_size()
        .map_err(|e| format!("Failed to get window size: {}", e))?;
    
    let (x, y) = (position.x as f64, position.y as f64);
    let (width, height) = (size.width as f64, size.height as f64);
    
    let is_maximized = window.is_maximized()
        .map_err(|e| format!("Failed to get maximized state: {}", e))?;
    let is_minimized = window.is_minimized()
        .map_err(|e| format!("Failed to get minimized state: {}", e))?;
    
    Ok(WindowState {
        x,
        y,
        width,
        height,
        is_maximized,
        is_minimized,
    })
}

#[tauri::command]
pub async fn maximize_window(window: Window) -> Result<(), String> {
    window.maximize()
        .map_err(|e| format!("Failed to maximize window: {}", e))
}

#[tauri::command]
pub async fn unmaximize_window(window: Window) -> Result<(), String> {
    window.unmaximize()
        .map_err(|e| format!("Failed to unmaximize window: {}", e))
}

#[tauri::command]
pub async fn minimize_window(window: Window) -> Result<(), String> {
    window.minimize()
        .map_err(|e| format!("Failed to minimize window: {}", e))
}

#[tauri::command]
pub async fn restore_window(window: Window) -> Result<(), String> {
    window.unminimize()
        .map_err(|e| format!("Failed to restore window: {}", e))
}

#[tauri::command]
pub async fn set_window_always_on_top(window: Window, always_on_top: bool) -> Result<(), String> {
    window.set_always_on_top(always_on_top)
        .map_err(|e| format!("Failed to set always on top: {}", e))
}

#[tauri::command]
pub async fn enable_window_drag_region(window: Window) -> Result<(), String> {
    // This would typically be handled by the frontend with data-tauri-drag-region
    // but we can provide a programmatic way to start dragging
    window.start_dragging()
        .map_err(|e| format!("Failed to start window dragging: {}", e))
}

// Utility function to detect resize zones based on mouse position
#[tauri::command]
pub async fn detect_resize_zone(
    mouse_x: f64,
    mouse_y: f64,
    window_width: f64,
    window_height: f64,
    edge_threshold: Option<f64>,
) -> Result<Option<ResizeDirection>, String> {
    let threshold = edge_threshold.unwrap_or(10.0);
    
    let at_left = mouse_x <= threshold;
    let at_right = mouse_x >= window_width - threshold;
    let at_top = mouse_y <= threshold;
    let at_bottom = mouse_y >= window_height - threshold;
    
    let direction = match (at_left, at_right, at_top, at_bottom) {
        (true, false, true, false) => Some(ResizeDirection::NorthWest),
        (false, true, true, false) => Some(ResizeDirection::NorthEast),
        (true, false, false, true) => Some(ResizeDirection::SouthWest),
        (false, true, false, true) => Some(ResizeDirection::SouthEast),
        (true, false, false, false) => Some(ResizeDirection::West),
        (false, true, false, false) => Some(ResizeDirection::East),
        (false, false, true, false) => Some(ResizeDirection::North),
        (false, false, false, true) => Some(ResizeDirection::South),
        _ => None,
    };
    
    Ok(direction)
}