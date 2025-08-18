// Data storage module - handles all data persistence operations
// Supports both legacy JSON storage and modern SQLite storage with seamless migration

pub mod json_store;           // Legacy JSON-based storage (data_store.rs)
pub mod sqlite_store;         // Modern SQLite-based storage
pub mod migration;            // Migration utilities and commands
pub mod hybrid_store;         // Hybrid storage that auto-selects backend

// Re-export commonly used types and functions
pub use json_store::*;
pub use sqlite_store::*;
pub use migration::*;
pub use hybrid_store::*;

// Re-export the main data structures for external use
pub use json_store::{
    ChatMessage, ChatSession, MessageAttachment, ThinkingProcess, ThinkingStep, MessageMetadata,
    ConversationSession, ConversationMessage, ConversationInsight,
    SaveChatsPayload, LoadChatsResponse, SaveConversationsPayload, LoadConversationsResponse,
    BackupInfo, ConversationMessageUpdate
};