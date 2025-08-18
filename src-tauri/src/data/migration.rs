use tauri::{AppHandle, Manager, command};
use crate::data::sqlite_store::{SqliteDataStore, MigrationResult};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStatus {
    pub is_sqlite_enabled: bool,
    pub has_json_data: bool,
    pub needs_migration: bool,
    pub migration_completed: bool,
    pub database_exists: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationResponse {
    pub success: bool,
    pub message: String,
    pub result: Option<MigrationResult>,
    pub error: Option<String>,
}

/// Check the current migration status
#[command]
pub fn check_migration_status(app_handle: AppHandle) -> Result<MigrationStatus, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let sqlite_path = app_data_dir.join("enteract_data.db");
    let chat_json_path = app_data_dir.join("user_chat_sessions.json");
    let conversation_json_path = app_data_dir.join("user_conversations.json");

    let database_exists = sqlite_path.exists();
    let has_json_data = chat_json_path.exists() || conversation_json_path.exists();
    
    // Check if migration was already completed
    let migration_completed = if database_exists {
        match SqliteDataStore::new(&app_handle) {
            Ok(store) => {
                // Check if migration status table has entries
                match store.connection.query_row(
                    "SELECT COUNT(*) FROM migration_status WHERE migration_name = ?",
                    rusqlite::params!["json_to_sqlite_v1"],
                    |row| row.get::<_, i64>(0)
                ) {
                    Ok(count) => count > 0,
                    Err(_) => false
                }
            }
            Err(_) => false
        }
    } else {
        false
    };

    let needs_migration = has_json_data && !migration_completed;

    Ok(MigrationStatus {
        is_sqlite_enabled: true, // We're enabling SQLite by default now
        has_json_data,
        needs_migration,
        migration_completed,
        database_exists,
    })
}

/// Perform the migration from JSON to SQLite
#[command]
pub fn migrate_to_sqlite(app_handle: AppHandle) -> MigrationResponse {
    println!("🚀 Starting JSON to SQLite migration...");
    
    match SqliteDataStore::new(&app_handle) {
        Ok(mut store) => {
            match store.migrate_from_json(&app_handle) {
                Ok(result) => {
                    println!("✅ Migration completed successfully!");
                    MigrationResponse {
                        success: true,
                        message: format!(
                            "Migration completed! Migrated {} total records: {} chat sessions with {} messages, {} conversation sessions with {} messages and {} insights",
                            result.total_records(),
                            result.chat_sessions_migrated,
                            result.chat_messages_migrated,
                            result.conversation_sessions_migrated,
                            result.conversation_messages_migrated,
                            result.conversation_insights_migrated
                        ),
                        result: Some(result),
                        error: None,
                    }
                }
                Err(e) => {
                    let error_msg = format!("Migration failed: {}", e);
                    println!("❌ {}", error_msg);
                    MigrationResponse {
                        success: false,
                        message: "Migration failed".to_string(),
                        result: None,
                        error: Some(error_msg),
                    }
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to initialize SQLite database: {}", e);
            println!("❌ {}", error_msg);
            MigrationResponse {
                success: false,
                message: "Failed to initialize database".to_string(),
                result: None,
                error: Some(error_msg),
            }
        }
    }
}

/// Create backup of JSON files before migration
#[command]
pub fn backup_json_files(app_handle: AppHandle) -> Result<Vec<String>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let backup_dir = app_data_dir.join("pre_migration_backup");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    let mut backed_up_files = Vec::new();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

    // Backup chat sessions
    let chat_json_path = app_data_dir.join("user_chat_sessions.json");
    if chat_json_path.exists() {
        let backup_path = backup_dir.join(format!("user_chat_sessions_{}.json", timestamp));
        std::fs::copy(&chat_json_path, &backup_path)
            .map_err(|e| format!("Failed to backup chat sessions: {}", e))?;
        backed_up_files.push(backup_path.to_string_lossy().to_string());
    }

    // Backup conversation sessions
    let conversation_json_path = app_data_dir.join("user_conversations.json");
    if conversation_json_path.exists() {
        let backup_path = backup_dir.join(format!("user_conversations_{}.json", timestamp));
        std::fs::copy(&conversation_json_path, &backup_path)
            .map_err(|e| format!("Failed to backup conversations: {}", e))?;
        backed_up_files.push(backup_path.to_string_lossy().to_string());
    }

    // Backup existing backups directory if it exists
    let existing_backups_dir = app_data_dir.join("backups");
    if existing_backups_dir.exists() {
        let backup_backups_path = backup_dir.join(format!("backups_{}", timestamp));
        copy_dir_all(&existing_backups_dir, &backup_backups_path)
            .map_err(|e| format!("Failed to backup existing backups: {}", e))?;
        backed_up_files.push(backup_backups_path.to_string_lossy().to_string());
    }

    Ok(backed_up_files)
}

/// Get SQLite database statistics
#[command]
pub fn get_sqlite_stats(app_handle: AppHandle) -> Result<SqliteStats, String> {
    match SqliteDataStore::new(&app_handle) {
        Ok(store) => {
            let chat_sessions: i64 = store.connection.query_row(
                "SELECT COUNT(*) FROM chat_sessions", rusqlite::params![], |row| row.get(0)
            ).unwrap_or(0);

            let chat_messages: i64 = store.connection.query_row(
                "SELECT COUNT(*) FROM chat_messages", rusqlite::params![], |row| row.get(0)
            ).unwrap_or(0);

            let conversation_sessions: i64 = store.connection.query_row(
                "SELECT COUNT(*) FROM conversation_sessions", rusqlite::params![], |row| row.get(0)
            ).unwrap_or(0);

            let conversation_messages: i64 = store.connection.query_row(
                "SELECT COUNT(*) FROM conversation_messages", rusqlite::params![], |row| row.get(0)
            ).unwrap_or(0);

            let conversation_insights: i64 = store.connection.query_row(
                "SELECT COUNT(*) FROM conversation_insights", rusqlite::params![], |row| row.get(0)
            ).unwrap_or(0);

            // Get database file size
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data directory: {}", e))?;
            
            let db_path = app_data_dir.join("enteract_data.db");
            let database_size = if db_path.exists() {
                std::fs::metadata(&db_path)
                    .map(|m| m.len())
                    .unwrap_or(0)
            } else {
                0
            };

            Ok(SqliteStats {
                chat_sessions: chat_sessions as usize,
                chat_messages: chat_messages as usize,
                conversation_sessions: conversation_sessions as usize,
                conversation_messages: conversation_messages as usize,
                conversation_insights: conversation_insights as usize,
                database_size_bytes: database_size,
                database_size_mb: database_size as f64 / 1024.0 / 1024.0,
            })
        }
        Err(e) => Err(format!("Failed to get SQLite stats: {}", e))
    }
}

/// Remove JSON files after successful migration (with confirmation)
#[command]
pub fn cleanup_json_files(app_handle: AppHandle, confirm: bool) -> Result<Vec<String>, String> {
    if !confirm {
        return Err("Confirmation required to delete JSON files".to_string());
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let mut removed_files = Vec::new();

    // Remove chat sessions JSON
    let chat_json_path = app_data_dir.join("user_chat_sessions.json");
    if chat_json_path.exists() {
        std::fs::remove_file(&chat_json_path)
            .map_err(|e| format!("Failed to remove chat sessions file: {}", e))?;
        removed_files.push(chat_json_path.to_string_lossy().to_string());
    }

    // Remove conversation sessions JSON
    let conversation_json_path = app_data_dir.join("user_conversations.json");
    if conversation_json_path.exists() {
        std::fs::remove_file(&conversation_json_path)
            .map_err(|e| format!("Failed to remove conversations file: {}", e))?;
        removed_files.push(conversation_json_path.to_string_lossy().to_string());
    }

    Ok(removed_files)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SqliteStats {
    pub chat_sessions: usize,
    pub chat_messages: usize,
    pub conversation_sessions: usize,
    pub conversation_messages: usize,
    pub conversation_insights: usize,
    pub database_size_bytes: u64,
    pub database_size_mb: f64,
}

// Helper function to copy directories recursively
fn copy_dir_all(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}