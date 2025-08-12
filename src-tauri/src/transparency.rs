use tauri::Window;

#[tauri::command]
pub async fn set_window_transparency(window: Window, alpha: f64) -> Result<(), String> {
    // Clamp alpha between 0.0 and 1.0
    let clamped_alpha = alpha.clamp(0.0, 1.0);
    
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetLayeredWindowAttributes, 
            GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT, LWA_ALPHA
        };
        
        if let Ok(hwnd) = window.hwnd() {
            let hwnd = HWND(hwnd.0 as isize);
            
            unsafe {
                // Get current extended window style
                let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                
                // Add layered window style for transparency
                ex_style |= WS_EX_LAYERED.0 as isize;
                
                // Add transparent style for click-through when very transparent
                if clamped_alpha < 0.1 {
                    ex_style |= WS_EX_TRANSPARENT.0 as isize;
                } else {
                    // Remove transparent style to enable interaction
                    ex_style &= !(WS_EX_TRANSPARENT.0 as isize);
                }
                
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
                
                // Set transparency level (0-255, where 255 is opaque)
                let alpha_value = (clamped_alpha * 255.0) as u8;
                SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), alpha_value, LWA_ALPHA)
                    .map_err(|e| format!("Failed to set transparency: {}", e))?;
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use objc::runtime::Object;
        use objc::{msg_send, sel, sel_impl};
        
        if let Ok(ns_window) = window.ns_window() {
            let ns_window = ns_window as *mut Object;
            unsafe {
                let _: () = msg_send![ns_window, setAlphaValue: clamped_alpha];
                
                // Enable/disable mouse events based on transparency
                let ignore_mouse = clamped_alpha < 0.1;
                let _: () = msg_send![ns_window, setIgnoresMouseEvents: ignore_mouse];
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux transparency implementation varies by window manager
        // This is a basic implementation for X11
        
        // Note: Linux implementation depends heavily on the desktop environment
        // This is a simplified version that may need adaptation
        window.set_decorations(false).map_err(|e| e.to_string())?;
        
        // For Wayland/X11, additional implementation would be needed
        // based on the specific compositor/window manager
    }
    
    Ok(())
}

#[tauri::command]
pub async fn emergency_restore_window(window: Window) -> Result<(), String> {
    // Always restore to fully opaque and interactive
    set_window_transparency(window.clone(), 1.0).await?;
    
    // Ensure window is visible and on top
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn toggle_transparency(window: Window, current_alpha: f64) -> Result<f64, String> {
    let new_alpha = if current_alpha > 0.5 { 0.3 } else { 1.0 };
    set_window_transparency(window, new_alpha).await?;
    Ok(new_alpha)
} 