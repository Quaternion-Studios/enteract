use tauri::Window;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object, Sel};
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use std::ffi::c_void;

#[tauri::command]
pub fn set_window_transparency(window: Window, alpha: f64) -> Result<(), String> {
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

                // Add transparent style for click-through when window has any transparency
                // UI elements with pointer-events in CSS will still be interactive
                if clamped_alpha < 1.0 {
                    ex_style |= WS_EX_TRANSPARENT.0 as isize;
                } else {
                    // Remove transparent style to enable interaction when fully opaque
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
        if let Ok(ns_window) = window.ns_window() {
            let ns_window = ns_window as *mut Object;
            unsafe {
                // Set window opacity
                let _: () = msg_send![ns_window, setAlphaValue: clamped_alpha];

                // Don't set ignoresMouseEvents here - it will be toggled dynamically
                // by set_mouse_passthrough based on cursor position
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
pub fn emergency_restore_window(window: Window) -> Result<(), String> {
    // Always restore to fully opaque and interactive
    set_window_transparency(window.clone(), 1.0)?;

    // Ensure window is visible and on top
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn toggle_transparency(window: Window, current_alpha: f64) -> Result<f64, String> {
    let new_alpha = if current_alpha > 0.5 { 0.3 } else { 1.0 };
    set_window_transparency(window, new_alpha)?;
    Ok(new_alpha)
}

/// Enable or disable mouse event passthrough
/// When enabled (true), clicks on transparent areas pass through to windows behind
/// When disabled (false), window captures all mouse events
#[tauri::command]
pub fn set_mouse_passthrough(window: Window, passthrough: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(ns_window) = window.ns_window() {
            let ns_window = ns_window as *mut Object;
            unsafe {
                let _: () = msg_send![ns_window, setIgnoresMouseEvents: passthrough];
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TRANSPARENT
        };

        if let Ok(hwnd) = window.hwnd() {
            let hwnd = HWND(hwnd.0 as isize);

            unsafe {
                let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

                if passthrough {
                    ex_style |= WS_EX_TRANSPARENT.0 as isize;
                } else {
                    ex_style &= !(WS_EX_TRANSPARENT.0 as isize);
                }

                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
            }
        }
    }

    Ok(())
}

/// Get global mouse position (screen coordinates)
#[cfg(target_os = "macos")]
fn get_global_mouse_position() -> (f64, f64) {
    unsafe {
        let ns_event_class = class!(NSEvent);
        let mouse_location: cocoa::foundation::NSPoint = msg_send![ns_event_class, mouseLocation];
        (mouse_location.x, mouse_location.y)
    }
}

/// Convert global screen coordinates to window-relative coordinates
#[cfg(target_os = "macos")]
fn global_to_window_coords(window: &Window, global_x: f64, global_y: f64) -> Option<(f64, f64)> {
    unsafe {
        if let Ok(ns_window) = window.ns_window() {
            let ns_window = ns_window as *mut Object;

            // Get window frame in screen coordinates
            let window_frame: cocoa::foundation::NSRect = msg_send![ns_window, frame];

            // Convert to window coordinates (origin at bottom-left)
            let window_x = global_x - window_frame.origin.x;
            let window_y = global_y - window_frame.origin.y;

            // Check if point is within window bounds
            if window_x >= 0.0 && window_x <= window_frame.size.width &&
               window_y >= 0.0 && window_y <= window_frame.size.height {
                // Convert from bottom-left origin to top-left origin (web coordinates)
                let web_y = window_frame.size.height - window_y;
                return Some((window_x, web_y));
            }
        }
    }
    None
}

/// Start global mouse position tracking
/// This polls mouse position even when window has setIgnoresMouseEvents:YES
#[tauri::command]
pub fn start_mouse_tracking(window: Window) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::thread;
        use std::time::Duration;
        use tauri::Emitter;

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(16)); // ~60fps

                let (global_x, global_y) = get_global_mouse_position();

                if let Some((window_x, window_y)) = global_to_window_coords(&window, global_x, global_y) {
                    // Mouse is over our window - emit event to frontend
                    #[derive(Clone, serde::Serialize)]
                    struct MousePosition {
                        x: f64,
                        y: f64,
                        #[serde(rename = "isOverWindow")]
                        is_over_window: bool,
                    }

                    let _ = window.emit("mouse-position", MousePosition {
                        x: window_x,
                        y: window_y,
                        is_over_window: true,
                    });
                } else {
                    // Mouse is outside window
                    #[derive(Clone, serde::Serialize)]
                    struct MousePosition {
                        #[serde(rename = "isOverWindow")]
                        is_over_window: bool,
                        x: f64,
                        y: f64,
                    }

                    let _ = window.emit("mouse-position", MousePosition {
                        is_over_window: false,
                        x: 0.0,
                        y: 0.0,
                    });
                }
            }
        });

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Mouse tracking only supported on macOS".to_string())
    }
}

/// Stop global mouse position tracking
#[tauri::command]
pub fn stop_mouse_tracking() -> Result<(), String> {
    // TODO: Implement tracking thread management with Arc<AtomicBool>
    // For now, this is a placeholder
    Ok(())
}