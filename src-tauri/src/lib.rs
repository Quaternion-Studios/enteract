// src-tauri/src/main.rs
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Import our modules
mod transparency;
mod window_manager;
mod eye_tracking;
mod speech;
mod ollama;
mod screenshot;
mod file_handler;
mod data_store;
mod audio_loopback; // New audio loopback module
mod system_prompts; // System prompts module

// Re-export the commands from modules
use transparency::{set_window_transparency, emergency_restore_window, toggle_transparency};
use window_manager::{
    move_window_to_position, get_window_position, get_window_size, get_screen_size,
    get_virtual_desktop_size, get_monitor_layout, set_window_bounds
};
use eye_tracking::{
    start_ml_eye_tracking, stop_ml_eye_tracking, get_ml_gaze_data, calibrate_ml_eye_tracking,
    get_ml_tracking_stats, pause_ml_tracking, resume_ml_tracking, detect_window_drag
};
use speech::{
    initialize_whisper_model, initialize_whisper_model_with_gpu, transcribe_audio_base64, transcribe_audio_file,
    check_whisper_model_availability, download_whisper_model, list_available_models,
    get_gpu_info, get_current_gpu_config
};
use ollama::{
    get_ollama_models, get_ollama_status, pull_ollama_model, delete_ollama_model,
    generate_ollama_response, generate_ollama_response_stream, get_ollama_model_info,
    generate_enteract_agent_response, generate_vision_analysis, generate_deep_research,
    generate_conversational_ai, generate_coding_agent_response, get_ollama_gpu_info,
    check_ollama_gpu_usage
};
use screenshot::{capture_screenshot, capture_screenshot_area};
use file_handler::{
    upload_file_base64, validate_file_upload, get_file_upload_config,
    process_clipboard_image, cleanup_temp_files
};
use data_store::{save_chat_sessions, load_chat_sessions, save_conversations, load_conversations, delete_conversation, clear_all_conversations};

// Import new audio loopback commands
use audio_loopback::{
    enumerate_loopback_devices, auto_select_best_device, test_audio_device,
    save_audio_settings, load_audio_settings, save_general_settings, load_general_settings,
    start_audio_loopback_capture, stop_audio_loopback_capture, process_audio_for_transcription
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Setup emergency global hotkey for transparency restore
            #[cfg(desktop)]
            {
                // Register global hotkey for emergency restore (Ctrl+Shift+Esc)
                // This ensures users can always regain control
                let _handle = app.handle().clone();
                
                // Note: Global hotkey registration would require additional dependencies
                // For now, we'll rely on window-level keyboard shortcuts
            }
            
            // Audio loopback functionality is initialized on-demand
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Existing commands
            greet,
            
            // Window management
            set_window_transparency,
            emergency_restore_window,
            toggle_transparency,
            move_window_to_position,
            get_window_position,
            get_window_size,
            get_screen_size,
            get_virtual_desktop_size,
            get_monitor_layout,
            set_window_bounds,
            
            // Eye tracking
            start_ml_eye_tracking,
            stop_ml_eye_tracking,
            get_ml_gaze_data,
            calibrate_ml_eye_tracking,
            get_ml_tracking_stats,
            pause_ml_tracking,
            resume_ml_tracking,
            detect_window_drag,
            
            // Speech transcription
            initialize_whisper_model,
            initialize_whisper_model_with_gpu,
            transcribe_audio_base64,
            transcribe_audio_file,
            check_whisper_model_availability,
            download_whisper_model,
            list_available_models,
            get_gpu_info,
            get_current_gpu_config,
            
            // Ollama AI
            get_ollama_models,
            get_ollama_status,
            pull_ollama_model,
            delete_ollama_model,
            generate_ollama_response,
            generate_ollama_response_stream,
            get_ollama_model_info,
            generate_enteract_agent_response,
            generate_vision_analysis,
            generate_deep_research,
            generate_conversational_ai,
            generate_coding_agent_response,
            get_ollama_gpu_info,
            check_ollama_gpu_usage,
            
            // Screenshot
            capture_screenshot,
            capture_screenshot_area,
            
            // File handling
            upload_file_base64,
            validate_file_upload,
            get_file_upload_config,
            process_clipboard_image,
            cleanup_temp_files,
            
            // Data storage
            save_chat_sessions,
            load_chat_sessions,
            save_conversations,
            load_conversations,
            delete_conversation,
            clear_all_conversations,
            
            // NEW: Audio loopback commands
            enumerate_loopback_devices,
            auto_select_best_device,
            test_audio_device,
            save_audio_settings,
            load_audio_settings,
            save_general_settings,
            load_general_settings,
            start_audio_loopback_capture,
            stop_audio_loopback_capture,
            process_audio_for_transcription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}