// Migration utilities for SQLite database initialization
// This module handles database setup and schema creation

use tauri::{AppHandle, Manager, command};
use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub database_exists: bool,
    pub is_initialized: bool,
    pub chat_sessions_count: usize,
    pub conversation_sessions_count: usize,
    pub database_size_bytes: u64,
    pub database_size_mb: f64,
}

/// Initialize the SQLite database with all necessary tables
#[command]
pub fn initialize_database(app_handle: AppHandle) -> Result<String, String> {
    let db_path = get_database_path(&app_handle)?;
    
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    let connection = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    // Configure SQLite for optimal performance
    connection.execute("PRAGMA foreign_keys = ON", params![])
        .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
    connection.execute("PRAGMA journal_mode = WAL", params![])
        .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
    connection.execute("PRAGMA synchronous = NORMAL", params![])
        .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
    connection.execute("PRAGMA cache_size = 10000", params![])
        .map_err(|e| format!("Failed to set cache size: {}", e))?;
    connection.execute("PRAGMA temp_store = memory", params![])
        .map_err(|e| format!("Failed to set temp store: {}", e))?;
    
    // Create all tables
    let schema = include_str!("../../../migration_schema.sql");
    connection.execute_batch(schema)
        .map_err(|e| format!("Failed to create tables: {}", e))?;
    
    println!("✅ Database initialized successfully at: {:?}", db_path);
    Ok(format!("Database initialized at: {}", db_path.display()))
}

/// Get information about the current database
#[command]
pub fn get_database_info(app_handle: AppHandle) -> Result<DatabaseInfo, String> {
    let db_path = get_database_path(&app_handle)?;
    
    if !db_path.exists() {
        return Ok(DatabaseInfo {
            database_exists: false,
            is_initialized: false,
            chat_sessions_count: 0,
            conversation_sessions_count: 0,
            database_size_bytes: 0,
            database_size_mb: 0.0,
        });
    }

    let connection = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // Check if database is initialized by looking for our tables
    let is_initialized = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('chat_sessions', 'conversation_sessions')",
        params![],
        |row| row.get::<_, i64>(0)
    ).unwrap_or(0) >= 2;

    let chat_sessions_count: i64 = if is_initialized {
        connection.query_row(
            "SELECT COUNT(*) FROM chat_sessions",
            params![],
            |row| row.get(0)
        ).unwrap_or(0)
    } else {
        0
    };

    let conversation_sessions_count: i64 = if is_initialized {
        connection.query_row(
            "SELECT COUNT(*) FROM conversation_sessions",
            params![],
            |row| row.get(0)
        ).unwrap_or(0)
    } else {
        0
    };

    let database_size_bytes = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(DatabaseInfo {
        database_exists: true,
        is_initialized,
        chat_sessions_count: chat_sessions_count as usize,
        conversation_sessions_count: conversation_sessions_count as usize,
        database_size_bytes,
        database_size_mb: database_size_bytes as f64 / 1024.0 / 1024.0,
    })
}

/// Clean up old JSON files after confirming SQLite is working
#[command]
pub fn cleanup_legacy_files(app_handle: AppHandle, confirm: bool) -> Result<Vec<String>, String> {
    if !confirm {
        return Err("Confirmation required to delete legacy files".to_string());
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let mut removed_files = Vec::new();

    // Remove old JSON files if they exist
    let json_files = vec![
        "user_chat_sessions.json",
        "user_conversations.json",
    ];

    for filename in json_files {
        let file_path = app_data_dir.join(filename);
        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .map_err(|e| format!("Failed to remove {}: {}", filename, e))?;
            removed_files.push(filename.to_string());
        }
    }

    Ok(removed_files)
}

// Helper function to get database path
fn get_database_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    Ok(app_data_dir.join("enteract_data.db"))
}