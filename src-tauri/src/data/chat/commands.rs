// Tauri commands for chat storage operations
use tauri::{AppHandle, command};
use crate::data::types::{SaveChatsPayload, LoadChatsResponse};
use super::storage::ChatStorage;

#[command]
pub fn save_chat_sessions(
    app_handle: AppHandle,
    payload: SaveChatsPayload,
) -> Result<(), String> {
    match ChatStorage::new(&app_handle) {
        Ok(mut storage) => storage.save_chat_sessions(payload)
            .map_err(|e| format!("Failed to save chat sessions: {}", e)),
        Err(e) => Err(format!("Failed to initialize chat storage: {}", e))
    }
}

#[command]
pub fn load_chat_sessions(app_handle: AppHandle) -> Result<LoadChatsResponse, String> {
    match ChatStorage::new(&app_handle) {
        Ok(storage) => storage.load_chat_sessions()
            .map_err(|e| format!("Failed to load chat sessions: {}", e)),
        Err(e) => Err(format!("Failed to initialize chat storage: {}", e))
    }
}