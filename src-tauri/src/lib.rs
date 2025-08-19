// src-tauri/src/main.rs
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use tauri::Manager;

// Import our modules
mod transparency;
mod window_manager;
mod eye_tracking;
mod speech;
mod ollama;
mod screenshot;
mod file_handler;
mod data; // Data storage module (JSON, SQLite, migration, hybrid)
mod audio_loopback; // New audio loopback module
mod system_prompts; // System prompts module
mod system_info; // System information module
mod rag_system; // RAG document system module
mod rag_commands; // RAG command handlers
mod simple_embedding_service; // Simple embedding service
mod search_service; // Tantivy search service
mod chunking_service; // Enhanced text chunking service
mod enhanced_rag_system; // Enhanced RAG system
mod enhanced_rag_commands; // Enhanced RAG command handlers
mod mcp; // MCP module for multi-command processing

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
    initialize_whisper_model, transcribe_audio_base64, transcribe_audio_file,
    check_whisper_model_availability, download_whisper_model, list_available_models
};
use ollama::{
    get_ollama_models, get_ollama_status, pull_ollama_model, delete_ollama_model,
    generate_ollama_response, generate_ollama_response_stream, get_ollama_model_info,
    generate_enteract_agent_response, generate_vision_analysis, generate_deep_research,
    generate_conversational_ai, generate_coding_agent_response, cancel_ai_response,
    get_gpu_acceleration_status,

    // MCP enhanced commands
    generate_mcp_enabled_response, create_mcp_session_for_ai, get_mcp_session_for_ai
};
use screenshot::{capture_screenshot, capture_screenshot_area};
use file_handler::{
    upload_file_base64, validate_file_upload, get_file_upload_config,
    process_clipboard_image, cleanup_temp_files
};
// Data storage imports are now handled above in the SQLite section

// Import new audio loopback commands
use audio_loopback::{
    enumerate_loopback_devices, auto_select_best_device, test_audio_device,
    save_audio_settings, load_audio_settings, save_general_settings, load_general_settings,
    start_audio_loopback_capture, stop_audio_loopback_capture, process_audio_for_transcription
};
use system_info::get_system_info;

// Import RAG commands
use rag_commands::{
    RagSystemState, initialize_rag_system, upload_document, get_all_documents,
    delete_document, search_documents, update_rag_settings, get_rag_settings,
    get_storage_stats, generate_embeddings, clear_embedding_cache
};

// Import Enhanced RAG commands
use enhanced_rag_commands::{
    EnhancedRagSystemState, initialize_enhanced_rag_system, upload_enhanced_document,
    get_all_enhanced_documents, delete_enhanced_document, search_enhanced_documents,
    generate_enhanced_embeddings, clear_enhanced_embedding_cache, update_enhanced_rag_settings,
    get_enhanced_rag_settings, get_enhanced_storage_stats, get_embedding_status,
    validate_enhanced_file_upload, check_document_duplicate, get_document_embedding_status,
    ensure_documents_ready_for_search, generate_embeddings_for_selection
};

// Import MCP commands
use mcp::{
    start_mcp_session, end_mcp_session, get_mcp_session_info, list_mcp_tools,
    execute_mcp_tool, respond_to_mcp_approval, get_mcp_session_logs, 
    list_active_mcp_sessions, create_mcp_session_manager, get_mcp_tool_schema,
    get_mcp_session_status, create_execution_plan_iterative, execute_plan_interactive
};

// Import SQLite data storage commands
use data::{
    // Database initialization and management
    initialize_database, get_database_info, cleanup_legacy_files, check_database_health,
    // Chat operations (Claude conversations)
    save_chat_sessions, load_chat_sessions,
    // Conversation operations (Audio conversations)
    save_conversations, load_conversations, delete_conversation, clear_all_conversations,
    save_conversation_message, batch_save_conversation_messages,
    update_conversation_message, delete_conversation_message,
    save_conversation_insight, get_conversation_insights,
    update_session_metadata, update_session_active_state, ping_backend,
    // Logging commands
    get_database_logs, get_database_logs_by_operation, get_database_logs_by_level,
    get_database_log_stats, clear_database_logs
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(RagSystemState(std::sync::Arc::new(std::sync::Mutex::new(None))))
        .manage(EnhancedRagSystemState(std::sync::Arc::new(std::sync::Mutex::new(None))))
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
            
            // Enhanced RAG system will be initialized on-demand from frontend
            
            // Keep legacy RAG system for compatibility
            let app_handle_legacy = app.handle().clone();
            let rag_state = app.state::<RagSystemState>().inner().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(mut state_guard) = rag_state.0.lock() {
                    match crate::rag_system::RagSystem::new(&app_handle_legacy) {
                        Ok(system) => {
                            *state_guard = Some(system);
                            println!("Legacy RAG system initialized successfully");
                        }
                        Err(e) => {
                            eprintln!("Failed to initialize legacy RAG system: {}", e);
                        }
                    }
                }
            });

            // Initialize MCP session manager
            let mcp_sessions = create_mcp_session_manager();
            app.manage(mcp_sessions);
            
            // Initialize SQLite database with comprehensive health checks
            let app_handle_db = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // First check database health
                match crate::data::check_database_health(app_handle_db.clone()) {
                    Ok(health) => {
                        if health.is_healthy {
                            println!("✅ Database is already healthy and ready");
                        } else {
                            println!("⚠️ Database health issues detected: {:?}", health.errors);
                            println!("🔧 Attempting to initialize/repair database...");
                            
                            match crate::data::initialize_database(app_handle_db.clone()) {
                                Ok(result) => {
                                    println!("✅ Database initialization completed: {}", result);
                                    
                                    // Verify health after initialization
                                    match crate::data::check_database_health(app_handle_db) {
                                        Ok(post_health) => {
                                            if post_health.is_healthy {
                                                println!("🎉 Database is now healthy after initialization");
                                            } else {
                                                eprintln!("❌ Database still has issues after initialization: {:?}", post_health.errors);
                                            }
                                        }
                                        Err(e) => eprintln!("❌ Failed to verify database health after init: {}", e),
                                    }
                                }
                                Err(e) => eprintln!("❌ Database initialization failed: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Database health check failed: {}", e);
                        println!("🔧 Attempting emergency database initialization...");
                        
                        if let Err(init_err) = crate::data::initialize_database(app_handle_db) {
                            eprintln!("❌ Emergency database initialization also failed: {}", init_err);
                        } else {
                            println!("✅ Emergency database initialization succeeded");
                        }
                    }
                }
            });
            
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
            transcribe_audio_base64,
            transcribe_audio_file,
            check_whisper_model_availability,
            download_whisper_model,
            list_available_models,
            
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
            cancel_ai_response,
            get_gpu_acceleration_status,
            
            // Screenshot
            capture_screenshot,
            capture_screenshot_area,
            
            // File handling
            upload_file_base64,
            validate_file_upload,
            get_file_upload_config,
            process_clipboard_image,
            cleanup_temp_files,
            
            // Database management
            initialize_database,
            get_database_info,
            cleanup_legacy_files,
            check_database_health,
            
            // Chat data storage (Claude conversations)
            save_chat_sessions,
            load_chat_sessions,
            
            // Conversation data storage (Audio conversations)
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
            process_audio_for_transcription,
            
            // System info
            get_system_info,
            
            // Message-level persistence
            save_conversation_message,
            batch_save_conversation_messages,
            update_conversation_message,
            delete_conversation_message,
            update_session_metadata,
            update_session_active_state,
            ping_backend,
            
            // Conversation insights
            save_conversation_insight,
            get_conversation_insights,
            
            // RAG system commands (legacy)
            initialize_rag_system,
            upload_document,
            get_all_documents,
            delete_document,
            search_documents,
            update_rag_settings,
            get_rag_settings,
            get_storage_stats,
            generate_embeddings,
            clear_embedding_cache,
            
            // Enhanced RAG system commands
            initialize_enhanced_rag_system,
            upload_enhanced_document,
            get_all_enhanced_documents,
            delete_enhanced_document,
            search_enhanced_documents,
            generate_enhanced_embeddings,
            clear_enhanced_embedding_cache,
            update_enhanced_rag_settings,
            get_enhanced_rag_settings,
            get_enhanced_storage_stats,
            get_embedding_status,
            validate_enhanced_file_upload,
            check_document_duplicate,
            get_document_embedding_status,
            ensure_documents_ready_for_search,
            generate_embeddings_for_selection,

            // MCP commands
            start_mcp_session,
            end_mcp_session,
            get_mcp_session_info,
            list_mcp_tools,
            execute_mcp_tool,
            respond_to_mcp_approval,
            get_mcp_session_logs,
            list_active_mcp_sessions,
            get_mcp_tool_schema,
            get_mcp_session_status,
            
            // LLM-driven MCP commands
            create_execution_plan_iterative,
            execute_plan_interactive,
            // Enhanced AI commands with MCP
            generate_mcp_enabled_response,
            create_mcp_session_for_ai,
            get_mcp_session_for_ai,
            
            // Message-level conversation operations
            save_conversation_message,
            batch_save_conversation_messages,
            update_conversation_message,
            delete_conversation_message,
            
            // Conversation insights
            save_conversation_insight,
            get_conversation_insights,
            
            // Backend connectivity
            ping_backend,
            
            // Database logging
            get_database_logs,
            get_database_logs_by_operation,
            get_database_logs_by_level,
            get_database_log_stats,
            clear_database_logs,

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}