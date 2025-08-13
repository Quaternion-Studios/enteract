use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use chrono::{DateTime, Utc};
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i32,
    pub text: String,
    pub sender: String, // 'user' | 'assistant' | 'transcription' | 'system'
    pub timestamp: String, // ISO 8601 string
    pub is_interim: Option<bool>,
    pub confidence: Option<f64>,
    pub source: Option<String>,
    pub attachments: Option<Vec<MessageAttachment>>,
    pub thinking: Option<ThinkingProcess>,
    #[serde(rename = "messageType")]
    pub message_type: Option<String>,
    pub metadata: Option<MessageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub id: String,
    #[serde(rename = "type")]
    pub attachment_type: String,
    pub name: String,
    pub size: i64,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub url: Option<String>,
    #[serde(rename = "base64Data")]
    pub base64_data: Option<String>,
    pub thumbnail: Option<String>,
    #[serde(rename = "extractedText")]
    pub extracted_text: Option<String>,
    pub dimensions: Option<FileDimensions>,
    #[serde(rename = "uploadProgress")]
    pub upload_progress: Option<i32>,
    #[serde(rename = "uploadStatus")]
    pub upload_status: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDimensions {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingProcess {
    #[serde(rename = "isVisible")]
    pub is_visible: bool,
    pub content: String,
    #[serde(rename = "isStreaming")]
    pub is_streaming: bool,
    pub steps: Option<Vec<ThinkingStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingStep {
    pub id: String,
    pub title: String,
    pub content: String,
    pub timestamp: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    #[serde(rename = "agentType")]
    pub agent_type: Option<String>,
    pub model: Option<String>,
    pub tokens: Option<i32>,
    #[serde(rename = "processingTime")]
    pub processing_time: Option<f64>,
    #[serde(rename = "analysisType")]
    pub analysis_type: Option<Vec<String>>,
    #[serde(rename = "searchQueries")]
    pub search_queries: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub history: Vec<ChatMessage>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "modelId")]
    pub model_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveChatsPayload {
    pub chats: Vec<ChatSession>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadChatsResponse {
    pub chats: Vec<ChatSession>,
}

// Conversation data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String, // 'user' | 'system'
    pub source: String, // 'microphone' | 'loopback'
    pub content: String,
    pub timestamp: i64,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationInsight {
    pub id: String,
    pub text: String,
    pub timestamp: i64,
    #[serde(rename = "contextLength")]
    pub context_length: i32,
    #[serde(rename = "type")]
    pub insight_type: String, // 'insight' | 'welcome'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    pub id: String,
    pub name: String,
    #[serde(rename = "startTime")]
    pub start_time: i64,
    #[serde(rename = "endTime")]
    pub end_time: Option<i64>,
    pub messages: Vec<ConversationMessage>,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    pub insights: Vec<ConversationInsight>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveConversationsPayload {
    pub conversations: Vec<ConversationSession>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadConversationsResponse {
    pub conversations: Vec<ConversationSession>,
}

fn get_chats_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle   
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Ensure the directory exists
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }

    Ok(app_data_dir.join("user_chat_sessions.json"))
}

fn get_backup_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle   
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let backup_dir = app_data_dir.join("backups");
    
    // Ensure the backup directory exists
    if !backup_dir.exists() {
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
    }
    
    Ok(backup_dir)
}

fn get_conversations_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle   
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Ensure the directory exists
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }

    Ok(app_data_dir.join("user_conversations.json"))
}

#[tauri::command]
pub fn save_chat_sessions(
    app_handle: AppHandle,
    payload: SaveChatsPayload,
) -> Result<(), String> {
    let file_path = get_chats_file_path(&app_handle)?;
    
    // Create backup before saving
    if file_path.exists() {
        create_backup(&app_handle, &file_path, "chat")?;
    }
    
    let json_content = serde_json::to_string_pretty(&payload.chats)
        .map_err(|e| format!("Failed to serialize chat sessions: {}", e))?;

    // Write to temporary file first
    let temp_path = file_path.with_extension("tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    
    temp_file.write_all(json_content.as_bytes())
        .map_err(|e| format!("Failed to write to temporary file: {}", e))?;
    
    temp_file.sync_all()
        .map_err(|e| format!("Failed to sync temporary file: {}", e))?;
    
    // Atomically rename temp file to actual file
    fs::rename(&temp_path, &file_path)
        .map_err(|e| format!("Failed to rename temporary file: {}", e))?;

    println!("Chat sessions saved safely to: {:?}", file_path);
    Ok(())
}

fn create_backup(app_handle: &AppHandle, source_path: &PathBuf, prefix: &str) -> Result<(), String> {
    let backup_dir = get_backup_dir(app_handle)?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("{}_{}.json", prefix, timestamp);
    let backup_path = backup_dir.join(backup_filename);
    
    fs::copy(source_path, &backup_path)
        .map_err(|e| format!("Failed to create backup: {}", e))?;
    
    // Clean up old backups (keep only last 10)
    cleanup_old_backups(&backup_dir, prefix, 10)?;
    
    println!("Backup created: {:?}", backup_path);
    Ok(())
}

fn cleanup_old_backups(backup_dir: &PathBuf, prefix: &str, keep_count: usize) -> Result<(), String> {
    let mut backups: Vec<_> = fs::read_dir(backup_dir)
        .map_err(|e| format!("Failed to read backup directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with(prefix)
        })
        .collect();
    
    // Sort by modification time (newest first)
    backups.sort_by_key(|entry| {
        entry.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    backups.reverse();
    
    // Remove old backups
    if backups.len() > keep_count {
        for backup in backups.iter().skip(keep_count) {
            if let Err(e) = fs::remove_file(backup.path()) {
                eprintln!("Failed to remove old backup: {}", e);
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
pub fn load_chat_sessions(app_handle: AppHandle) -> Result<LoadChatsResponse, String> {
    let file_path = get_chats_file_path(&app_handle)?;

    if !file_path.exists() {
        println!("Chat sessions file does not exist, returning empty list");
        return Ok(LoadChatsResponse {
            chats: Vec::new(),
        });
    }

    let file_content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read chat sessions file: {}", e))?;

    let chats: Vec<ChatSession> = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to deserialize chat sessions: {}", e))?;

    println!("Loaded {} chat sessions from: {:?}", chats.len(), file_path);
    Ok(LoadChatsResponse { chats })
}

// Conversation storage functions
#[tauri::command]
pub fn save_conversations(
    app_handle: AppHandle,
    payload: SaveConversationsPayload,
) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    // Create backup before saving
    if file_path.exists() {
        create_backup(&app_handle, &file_path, "conversation")?;
    }
    
    let json_content = serde_json::to_string_pretty(&payload.conversations)
        .map_err(|e| format!("Failed to serialize conversations: {}", e))?;

    // Write to temporary file first
    let temp_path = file_path.with_extension("tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    
    temp_file.write_all(json_content.as_bytes())
        .map_err(|e| format!("Failed to write to temporary file: {}", e))?;
    
    temp_file.sync_all()
        .map_err(|e| format!("Failed to sync temporary file: {}", e))?;
    
    // Atomically rename temp file to actual file
    fs::rename(&temp_path, &file_path)
        .map_err(|e| format!("Failed to rename temporary file: {}", e))?;

    println!("Conversations saved safely to: {:?}", file_path);
    Ok(())
}

#[tauri::command]
pub fn load_conversations(app_handle: AppHandle) -> Result<LoadConversationsResponse, String> {
    let file_path = get_conversations_file_path(&app_handle)?;

    if !file_path.exists() {
        println!("Conversations file does not exist, returning empty list");
        return Ok(LoadConversationsResponse {
            conversations: Vec::new(),
        });
    }

    let file_content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read conversations file: {}", e))?;

    let conversations: Vec<ConversationSession> = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to deserialize conversations: {}", e))?;

    println!("Loaded {} conversations from: {:?}", conversations.len(), file_path);
    Ok(LoadConversationsResponse { conversations })
}

#[tauri::command] 
pub fn delete_conversation(
    app_handle: AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;

    if !file_path.exists() {
        return Err("Conversations file does not exist".to_string());
    }

    let file_content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read conversations file: {}", e))?;

    let mut conversations: Vec<ConversationSession> = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to deserialize conversations: {}", e))?;

    // Filter out the conversation to delete
    let original_count = conversations.len();
    conversations.retain(|conv| conv.id != conversation_id);

    if conversations.len() == original_count {
        return Err(format!("Conversation with ID {} not found", conversation_id));
    }

    // Save the updated list
    let json_content = serde_json::to_string_pretty(&conversations)
        .map_err(|e| format!("Failed to serialize conversations: {}", e))?;

    fs::write(&file_path, json_content)
        .map_err(|e| format!("Failed to write conversations file: {}", e))?;

    println!("Deleted conversation {} from: {:?}", conversation_id, file_path);
    Ok(())
}

#[tauri::command]
pub fn clear_all_conversations(app_handle: AppHandle) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    // Create backup before clearing
    if file_path.exists() {
        create_backup(&app_handle, &file_path, "conversation_cleared")?;
    }
    
    // Write empty array
    let empty_conversations: Vec<ConversationSession> = Vec::new();
    let json_content = serde_json::to_string_pretty(&empty_conversations)
        .map_err(|e| format!("Failed to serialize empty conversations: {}", e))?;

    fs::write(&file_path, json_content)
        .map_err(|e| format!("Failed to write conversations file: {}", e))?;

    println!("Cleared all conversations in: {:?}", file_path);
    Ok(())
}

#[tauri::command]
pub fn restore_from_backup(
    app_handle: AppHandle,
    backup_type: String,
    backup_filename: String,
) -> Result<(), String> {
    let backup_dir = get_backup_dir(&app_handle)?;
    let backup_path = backup_dir.join(&backup_filename);
    
    if !backup_path.exists() {
        return Err(format!("Backup file not found: {}", backup_filename));
    }
    
    let target_path = match backup_type.as_str() {
        "chat" => get_chats_file_path(&app_handle)?,
        "conversation" => get_conversations_file_path(&app_handle)?,
        _ => return Err(format!("Invalid backup type: {}", backup_type)),
    };
    
    // Create backup of current data before restoring
    if target_path.exists() {
        create_backup(&app_handle, &target_path, &format!("{}_before_restore", backup_type))?;
    }
    
    fs::copy(&backup_path, &target_path)
        .map_err(|e| format!("Failed to restore from backup: {}", e))?;
    
    println!("Restored from backup: {:?} to {:?}", backup_path, target_path);
    Ok(())
}

#[tauri::command]
pub fn list_backups(app_handle: AppHandle) -> Result<Vec<BackupInfo>, String> {
    let backup_dir = get_backup_dir(&app_handle)?;
    
    let mut backups = Vec::new();
    
    for entry in fs::read_dir(&backup_dir)
        .map_err(|e| format!("Failed to read backup directory: {}", e))? {
        
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let backup_type = if filename.starts_with("chat_") {
                        "chat".to_string()
                    } else if filename.starts_with("conversation_") {
                        "conversation".to_string()
                    } else {
                        continue;
                    };
                    
                    let modified = metadata.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    
                    backups.push(BackupInfo {
                        filename,
                        backup_type,
                        size: metadata.len(),
                        modified,
                    });
                }
            }
        }
    }
    
    // Sort by modification time (newest first)
    backups.sort_by(|a, b| b.modified.cmp(&a.modified));
    
    Ok(backups)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub backup_type: String,
    pub size: u64,
    pub modified: i64,
}

// Message-level operations
#[tauri::command]
pub fn save_conversation_message(
    app_handle: AppHandle,
    session_id: String,
    message: ConversationMessage,
) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    // Load existing conversations
    let mut conversations = if file_path.exists() {
        let file_content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read conversations file: {}", e))?;
        serde_json::from_str::<Vec<ConversationSession>>(&file_content)
            .map_err(|e| format!("Failed to deserialize conversations: {}", e))?
    } else {
        Vec::new()
    };
    
    // Find the session
    let session = conversations.iter_mut()
        .find(|s| s.id == session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;
    
    // Check if message already exists (deduplication)
    if session.messages.iter().any(|m| m.id == message.id) {
        return Ok(()); // Message already saved
    }
    
    // Add the message
    session.messages.push(message);
    
    // Save to file atomically
    let json_content = serde_json::to_string_pretty(&conversations)
        .map_err(|e| format!("Failed to serialize conversations: {}", e))?;
    
    let temp_path = file_path.with_extension("tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    
    temp_file.write_all(json_content.as_bytes())
        .map_err(|e| format!("Failed to write to temporary file: {}", e))?;
    
    temp_file.sync_all()
        .map_err(|e| format!("Failed to sync temporary file: {}", e))?;
    
    fs::rename(&temp_path, &file_path)
        .map_err(|e| format!("Failed to rename temporary file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub fn batch_save_conversation_messages(
    app_handle: AppHandle,
    session_id: String,
    messages: Vec<ConversationMessage>,
) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    // Load existing conversations
    let mut conversations = if file_path.exists() {
        let file_content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read conversations file: {}", e))?;
        serde_json::from_str::<Vec<ConversationSession>>(&file_content)
            .map_err(|e| format!("Failed to deserialize conversations: {}", e))?
    } else {
        Vec::new()
    };
    
    // Find the session
    let session = conversations.iter_mut()
        .find(|s| s.id == session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;
    
    // Add messages (with deduplication)
    for message in messages {
        if !session.messages.iter().any(|m| m.id == message.id) {
            session.messages.push(message);
        }
    }
    
    // Save to file atomically
    let json_content = serde_json::to_string_pretty(&conversations)
        .map_err(|e| format!("Failed to serialize conversations: {}", e))?;
    
    let temp_path = file_path.with_extension("tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    
    temp_file.write_all(json_content.as_bytes())
        .map_err(|e| format!("Failed to write to temporary file: {}", e))?;
    
    temp_file.sync_all()
        .map_err(|e| format!("Failed to sync temporary file: {}", e))?;
    
    fs::rename(&temp_path, &file_path)
        .map_err(|e| format!("Failed to rename temporary file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub fn update_conversation_message(
    app_handle: AppHandle,
    session_id: String,
    message_id: String,
    updates: ConversationMessageUpdate,
) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    if !file_path.exists() {
        return Err("Conversations file does not exist".to_string());
    }
    
    let file_content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read conversations file: {}", e))?;
    
    let mut conversations: Vec<ConversationSession> = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to deserialize conversations: {}", e))?;
    
    // Find the session and message
    let session = conversations.iter_mut()
        .find(|s| s.id == session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;
    
    let message = session.messages.iter_mut()
        .find(|m| m.id == message_id)
        .ok_or_else(|| format!("Message {} not found", message_id))?;
    
    // Apply updates
    if let Some(content) = updates.content {
        message.content = content;
    }
    if let Some(confidence) = updates.confidence {
        message.confidence = Some(confidence);
    }
    if let Some(timestamp) = updates.timestamp {
        message.timestamp = timestamp;
    }
    
    // Save to file atomically
    let json_content = serde_json::to_string_pretty(&conversations)
        .map_err(|e| format!("Failed to serialize conversations: {}", e))?;
    
    let temp_path = file_path.with_extension("tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    
    temp_file.write_all(json_content.as_bytes())
        .map_err(|e| format!("Failed to write to temporary file: {}", e))?;
    
    temp_file.sync_all()
        .map_err(|e| format!("Failed to sync temporary file: {}", e))?;
    
    fs::rename(&temp_path, &file_path)
        .map_err(|e| format!("Failed to rename temporary file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub fn delete_conversation_message(
    app_handle: AppHandle,
    session_id: String,
    message_id: String,
) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    if !file_path.exists() {
        return Err("Conversations file does not exist".to_string());
    }
    
    let file_content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read conversations file: {}", e))?;
    
    let mut conversations: Vec<ConversationSession> = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to deserialize conversations: {}", e))?;
    
    // Find the session
    let session = conversations.iter_mut()
        .find(|s| s.id == session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;
    
    // Remove the message
    let original_count = session.messages.len();
    session.messages.retain(|m| m.id != message_id);
    
    if session.messages.len() == original_count {
        return Err(format!("Message {} not found", message_id));
    }
    
    // Save to file atomically
    let json_content = serde_json::to_string_pretty(&conversations)
        .map_err(|e| format!("Failed to serialize conversations: {}", e))?;
    
    let temp_path = file_path.with_extension("tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;
    
    temp_file.write_all(json_content.as_bytes())
        .map_err(|e| format!("Failed to write to temporary file: {}", e))?;
    
    temp_file.sync_all()
        .map_err(|e| format!("Failed to sync temporary file: {}", e))?;
    
    fs::rename(&temp_path, &file_path)
        .map_err(|e| format!("Failed to rename temporary file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub fn save_conversation_insight(
    app_handle: AppHandle,
    session_id: String,
    insight: ConversationInsight,
) -> Result<(), String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    let mut conversations = if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read conversations file: {}", e))?;
        
        if content.trim().is_empty() {
            LoadConversationsResponse { conversations: vec![] }
        } else {
            serde_json::from_str::<LoadConversationsResponse>(&content)
                .map_err(|e| format!("Failed to parse conversations file: {}", e))?
        }
    } else {
        LoadConversationsResponse { conversations: vec![] }
    };
    
    // Find the conversation and add the insight
    if let Some(conversation) = conversations.conversations.iter_mut().find(|c| c.id == session_id) {
        conversation.insights.push(insight);
        
        // Save back to file
        let updated_payload = SaveConversationsPayload {
            conversations: conversations.conversations,
        };
        save_conversations(app_handle, updated_payload)?;
    } else {
        return Err(format!("Conversation with id {} not found", session_id));
    }
    
    Ok(())
}

#[tauri::command]
pub fn get_conversation_insights(
    app_handle: AppHandle,
    session_id: String,
) -> Result<Vec<ConversationInsight>, String> {
    let file_path = get_conversations_file_path(&app_handle)?;
    
    if !file_path.exists() {
        return Ok(vec![]);
    }
    
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read conversations file: {}", e))?;
    
    if content.trim().is_empty() {
        return Ok(vec![]);
    }
    
    let conversations: LoadConversationsResponse = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse conversations file: {}", e))?;
    
    if let Some(conversation) = conversations.conversations.iter().find(|c| c.id == session_id) {
        Ok(conversation.insights.clone())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn ping_backend() -> Result<String, String> {
    Ok("pong".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationMessageUpdate {
    pub content: Option<String>,
    pub confidence: Option<f64>,
    pub timestamp: Option<i64>,
} 