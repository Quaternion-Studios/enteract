// Hybrid data store that can use either JSON or SQLite based on migration status
// This provides a seamless transition path from JSON to SQLite

use tauri::{AppHandle, Manager, command};
use crate::data::json_store::{
    SaveChatsPayload, LoadChatsResponse, SaveConversationsPayload, LoadConversationsResponse,
    BackupInfo, ConversationMessage, ConversationInsight
};
use crate::data::sqlite_store::SqliteDataStore;
use std::path::PathBuf;

pub struct HybridDataStore;

impl HybridDataStore {
    /// Determines if we should use SQLite based on migration status
    fn should_use_sqlite(app_handle: &AppHandle) -> bool {
        // Check if SQLite database exists and migration is completed
        if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
            let db_path = app_data_dir.join("enteract_data.db");
            if db_path.exists() {
                // Try to check if migration was completed
                if let Ok(store) = SqliteDataStore::new(app_handle) {
                    if let Ok(count) = store.connection.query_row(
                        "SELECT COUNT(*) FROM migration_status WHERE migration_name = ?",
                        rusqlite::params!["json_to_sqlite_v1"],
                        |row| row.get::<_, i64>(0)
                    ) {
                        return count > 0;
                    }
                }
            }
        }
        false
    }
}

// New hybrid commands that automatically choose the right storage backend

#[command]
pub fn save_chat_sessions_hybrid(
    app_handle: AppHandle,
    payload: SaveChatsPayload,
) -> Result<(), String> {
    if HybridDataStore::should_use_sqlite(&app_handle) {
        // Use SQLite
        match SqliteDataStore::new(&app_handle) {
            Ok(mut store) => store.save_chat_sessions(payload)
                .map_err(|e| format!("SQLite save failed: {}", e)),
            Err(e) => {
                println!("⚠️ SQLite failed, falling back to JSON: {}", e);
                crate::data::json_store::save_chat_sessions(app_handle, payload)
            }
        }
    } else {
        // Use JSON (legacy)
        crate::data::json_store::save_chat_sessions(app_handle, payload)
    }
}

#[command]
pub fn load_chat_sessions_hybrid(app_handle: AppHandle) -> Result<LoadChatsResponse, String> {
    if HybridDataStore::should_use_sqlite(&app_handle) {
        // Use SQLite
        match SqliteDataStore::new(&app_handle) {
            Ok(store) => store.load_chat_sessions()
                .map_err(|e| format!("SQLite load failed: {}", e)),
            Err(e) => {
                println!("⚠️ SQLite failed, falling back to JSON: {}", e);
                crate::data::json_store::load_chat_sessions(app_handle)
            }
        }
    } else {
        // Use JSON (legacy)
        crate::data::json_store::load_chat_sessions(app_handle)
    }
}

#[command]
pub fn save_conversations_hybrid(
    app_handle: AppHandle,
    payload: SaveConversationsPayload,
) -> Result<(), String> {
    if HybridDataStore::should_use_sqlite(&app_handle) {
        // Use SQLite
        match SqliteDataStore::new(&app_handle) {
            Ok(mut store) => store.save_conversations(payload)
                .map_err(|e| format!("SQLite save failed: {}", e)),
            Err(e) => {
                println!("⚠️ SQLite failed, falling back to JSON: {}", e);
                crate::data::json_store::save_conversations(app_handle, payload)
            }
        }
    } else {
        // Use JSON (legacy)
        crate::data::json_store::save_conversations(app_handle, payload)
    }
}

#[command]
pub fn load_conversations_hybrid(app_handle: AppHandle) -> Result<LoadConversationsResponse, String> {
    if HybridDataStore::should_use_sqlite(&app_handle) {
        // Use SQLite
        match SqliteDataStore::new(&app_handle) {
            Ok(store) => store.load_conversations()
                .map_err(|e| format!("SQLite load failed: {}", e)),
            Err(e) => {
                println!("⚠️ SQLite failed, falling back to JSON: {}", e);
                crate::data::json_store::load_conversations(app_handle)
            }
        }
    } else {
        // Use JSON (legacy)
        crate::data::json_store::load_conversations(app_handle)
    }
}

// For other operations, we can fallback to JSON implementations for now
// These can be gradually migrated to SQLite as needed

#[command]
pub fn delete_conversation_hybrid(
    app_handle: AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    // For now, delegate to JSON implementation
    // TODO: Implement SQLite version when needed
    crate::data::json_store::delete_conversation(app_handle, conversation_id)
}

#[command]
pub fn clear_all_conversations_hybrid(app_handle: AppHandle) -> Result<(), String> {
    // For now, delegate to JSON implementation  
    // TODO: Implement SQLite version when needed
    crate::data::json_store::clear_all_conversations(app_handle)
}

#[command]
pub fn list_backups_hybrid(app_handle: AppHandle) -> Result<Vec<BackupInfo>, String> {
    // Backups are still JSON-based for now
    crate::data::json_store::list_backups(app_handle)
}

#[command]
pub fn restore_from_backup_hybrid(
    app_handle: AppHandle,
    backup_type: String,
    backup_filename: String,
) -> Result<(), String> {
    // Backup restoration is still JSON-based for now
    crate::data::json_store::restore_from_backup(app_handle, backup_type, backup_filename)
}

// Message-level operations continue to use JSON for now
// These require more careful migration due to their granular nature

#[command]
pub fn save_conversation_message_hybrid(
    app_handle: AppHandle,
    session_id: String,
    message: ConversationMessage,
) -> Result<(), String> {
    crate::data::json_store::save_conversation_message(app_handle, session_id, message)
}

#[command]
pub fn batch_save_conversation_messages_hybrid(
    app_handle: AppHandle,
    session_id: String,
    messages: Vec<ConversationMessage>,
) -> Result<(), String> {
    crate::data::json_store::batch_save_conversation_messages(app_handle, session_id, messages)
}

#[command]
pub fn save_conversation_insight_hybrid(
    app_handle: AppHandle,
    session_id: String,
    insight: ConversationInsight,
) -> Result<(), String> {
    crate::data::json_store::save_conversation_insight(app_handle, session_id, insight)
}

#[command]
pub fn get_conversation_insights_hybrid(
    app_handle: AppHandle,
    session_id: String,
) -> Result<Vec<ConversationInsight>, String> {
    crate::data::json_store::get_conversation_insights(app_handle, session_id)
}