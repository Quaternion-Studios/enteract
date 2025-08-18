// Tauri commands for conversation storage operations
use tauri::{AppHandle, command};
use crate::data::types::{
    SaveConversationsPayload, LoadConversationsResponse,
    ConversationMessage, ConversationInsight, ConversationMessageUpdate
};
use super::storage::ConversationStorage;

#[command]
pub fn save_conversations(
    app_handle: AppHandle,
    payload: SaveConversationsPayload,
) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => storage.save_conversations(payload)
            .map_err(|e| format!("Failed to save conversations: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

#[command]
pub fn load_conversations(app_handle: AppHandle) -> Result<LoadConversationsResponse, String> {
    match ConversationStorage::new(&app_handle) {
        Ok(storage) => storage.load_conversations()
            .map_err(|e| format!("Failed to load conversations: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

#[command]
pub fn delete_conversation(
    app_handle: AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => storage.delete_conversation(&conversation_id)
            .map_err(|e| format!("Failed to delete conversation: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

#[command]
pub fn clear_all_conversations(app_handle: AppHandle) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => storage.clear_all_conversations()
            .map_err(|e| format!("Failed to clear conversations: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

// Message-level operations
#[command]
pub fn save_conversation_message(
    app_handle: AppHandle,
    session_id: String,
    message: ConversationMessage,
) -> Result<(), String> {
    println!("📥 save_conversation_message called - session_id: {}, message_id: {}", session_id, message.id);
    println!("📝 Message details - type: '{}', source: '{}', content length: {}", 
             message.message_type, message.source, message.content.len());
    
    // Validate required fields
    if message.message_type.is_empty() {
        let error_msg = format!("Message type is empty for message {}", message.id);
        println!("❌ {}", error_msg);
        return Err(error_msg);
    }
    
    if message.source.is_empty() {
        let error_msg = format!("Message source is empty for message {}", message.id);
        println!("❌ {}", error_msg);
        return Err(error_msg);
    }
    
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => {
            let result = storage.save_conversation_message(&session_id, message);
            match result {
                Ok(_) => {
                    println!("✅ Message saved successfully");
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to save conversation message: {}", e);
                    println!("❌ {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to initialize conversation storage: {}", e);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }
}

#[command]
pub fn batch_save_conversation_messages(
    app_handle: AppHandle,
    session_id: String,
    messages: Vec<ConversationMessage>,
) -> Result<(), String> {
    println!("📥 batch_save_conversation_messages called - session_id: {}, message_count: {}", session_id, messages.len());
    
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => {
            let result = storage.batch_save_conversation_messages(&session_id, messages);
            match result {
                Ok(_) => {
                    println!("✅ Batch messages saved successfully");
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to batch save conversation messages: {}", e);
                    println!("❌ {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to initialize conversation storage: {}", e);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }
}

#[command]
pub fn update_conversation_message(
    app_handle: AppHandle,
    session_id: String,
    message_id: String,
    updates: ConversationMessageUpdate,
) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => storage.update_conversation_message(&session_id, &message_id, updates)
            .map_err(|e| format!("Failed to update conversation message: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

#[command]
pub fn delete_conversation_message(
    app_handle: AppHandle,
    session_id: String,
    message_id: String,
) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => storage.delete_conversation_message(&session_id, &message_id)
            .map_err(|e| format!("Failed to delete conversation message: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

// Insight operations
#[command]
pub fn save_conversation_insight(
    app_handle: AppHandle,
    session_id: String,
    insight: ConversationInsight,
) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => storage.save_conversation_insight(&session_id, insight)
            .map_err(|e| format!("Failed to save conversation insight: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

#[command]
pub fn get_conversation_insights(
    app_handle: AppHandle,
    session_id: String,
) -> Result<Vec<ConversationInsight>, String> {
    match ConversationStorage::new(&app_handle) {
        Ok(storage) => storage.get_conversation_insights(&session_id)
            .map_err(|e| format!("Failed to get conversation insights: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

// Session metadata operations  
#[command]
pub fn update_session_metadata(
    app_handle: AppHandle,
    session_id: String,
    name: Option<String>,
    end_time: Option<Option<i64>>,
    is_active: Option<bool>,
) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => {
            let name_ref = name.as_deref();
            storage.update_session_metadata(&session_id, name_ref, end_time, is_active)
                .map_err(|e| format!("Failed to update session metadata: {}", e))
        }
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

#[command]
pub fn update_session_active_state(
    app_handle: AppHandle,
    session_id: String,
    is_active: bool,
) -> Result<(), String> {
    match ConversationStorage::new(&app_handle) {
        Ok(mut storage) => storage.update_session_active_state(&session_id, is_active)
            .map_err(|e| format!("Failed to update session active state: {}", e)),
        Err(e) => Err(format!("Failed to initialize conversation storage: {}", e))
    }
}

#[command]
pub fn ping_backend() -> Result<String, String> {
    Ok("pong".to_string())
}