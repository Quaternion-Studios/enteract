// Data storage module - Pure SQLite implementation
// Handles all data persistence for Enteract with separate chat and conversation storage

pub mod types;           // Core data structures
pub mod chat;            // Chat session storage (Claude conversations)
pub mod conversation;    // Audio conversation storage
pub mod migration;       // Database initialization and cleanup

// Re-export all the commonly used types and functions
pub use types::*;

// Re-export chat commands
pub use chat::{
    save_chat_sessions,
    load_chat_sessions,
};

// Re-export conversation commands
pub use conversation::{
    save_conversations,
    load_conversations,
    delete_conversation,
    clear_all_conversations,
    save_conversation_message,
    batch_save_conversation_messages,
    update_conversation_message,
    delete_conversation_message,
    save_conversation_insight,
    get_conversation_insights,
    ping_backend,
};

// Re-export migration commands
pub use migration::{
    initialize_database,
    get_database_info,
    cleanup_legacy_files,
};