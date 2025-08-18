// Core data types for the Enteract data storage system
// This file defines all the data structures used across chat and conversation storage

use serde::{Deserialize, Serialize};

// ============================================================================
// CHAT SESSION TYPES (Main Claude Chat)
// ============================================================================

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

// Request/Response types for chat operations
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveChatsPayload {
    pub chats: Vec<ChatSession>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadChatsResponse {
    pub chats: Vec<ChatSession>,
}

// ============================================================================
// CONVERSATION TYPES (Audio Conversations)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String, // 'user' | 'system'
    pub source: String, // 'microphone' | 'loopback'
    pub content: String,
    pub timestamp: i64,
    pub confidence: Option<f64>,
    // Additional fields for frontend compatibility
    #[serde(rename = "isPreview", skip_serializing_if = "Option::is_none")]
    pub is_preview: Option<bool>,
    #[serde(rename = "isTyping", skip_serializing_if = "Option::is_none")]
    pub is_typing: Option<bool>,
    #[serde(rename = "persistenceState", skip_serializing_if = "Option::is_none")]
    pub persistence_state: Option<String>,
    #[serde(rename = "retryCount", skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<i32>,
    #[serde(rename = "lastSaveAttempt", skip_serializing_if = "Option::is_none")]
    pub last_save_attempt: Option<i64>,
    #[serde(rename = "saveError", skip_serializing_if = "Option::is_none")]
    pub save_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationInsight {
    pub id: String,
    pub text: String,
    pub timestamp: i64,
    #[serde(rename = "contextLength")]
    pub context_length: i32,
    #[serde(rename = "type")]
    pub insight_type: String, // 'insight' | 'welcome' | 'question' | 'answer'
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
    #[serde(default)]
    pub insights: Vec<ConversationInsight>,
}

// Request/Response types for conversation operations
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveConversationsPayload {
    pub conversations: Vec<ConversationSession>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadConversationsResponse {
    pub conversations: Vec<ConversationSession>,
}

// Update structures for granular operations
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationMessageUpdate {
    pub content: Option<String>,
    pub confidence: Option<f64>,
    pub timestamp: Option<i64>,
}

// ============================================================================
// BACKUP AND UTILITY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub backup_type: String,
    pub size: u64,
    pub modified: i64,
}