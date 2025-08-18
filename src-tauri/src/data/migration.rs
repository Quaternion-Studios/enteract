// Migration utilities for SQLite database initialization
// This module handles database setup and schema creation

use tauri::{AppHandle, Manager, command};
use serde::{Serialize, Deserialize};
use rusqlite::{Connection, params, Result as SqliteResult, Error as SqliteError};
use std::path::PathBuf;
use std::fs;
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub database_exists: bool,
    pub is_initialized: bool,
    pub chat_sessions_count: usize,
    pub conversation_sessions_count: usize,
    pub database_size_bytes: u64,
    pub database_size_mb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub is_healthy: bool,
    pub can_connect: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub foreign_keys_enabled: bool,
    pub wal_mode: bool,
    pub tables_exist: bool,
    pub indexes_exist: bool,
    pub path_accessible: bool,
    pub directory_writable: bool,
    pub last_check: i64,
    pub check_duration_ms: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConnectionPool {
    pub max_connections: usize,
    pub active_connections: usize,
    pub total_created: usize,
    pub total_closed: usize,
    pub connection_errors: usize,
}

/// Comprehensive database health check
#[command]
pub fn check_database_health(app_handle: AppHandle) -> Result<DatabaseHealth, String> {
    let start_time = Instant::now();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    let db_path = match get_database_path(&app_handle) {
        Ok(path) => path,
        Err(e) => {
            errors.push(format!("Failed to get database path: {}", e));
            return Ok(DatabaseHealth {
                is_healthy: false,
                can_connect: false,
                can_read: false,
                can_write: false,
                foreign_keys_enabled: false,
                wal_mode: false,
                tables_exist: false,
                indexes_exist: false,
                path_accessible: false,
                directory_writable: false,
                last_check: chrono::Utc::now().timestamp(),
                check_duration_ms: start_time.elapsed().as_millis() as u64,
                errors,
                warnings,
            });
        }
    };

    // Check path accessibility
    let path_accessible = db_path.exists();
    if !path_accessible {
        warnings.push("Database file does not exist".to_string());
    }

    // Check directory writability
    let directory_writable = if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            match fs::create_dir_all(parent) {
                Ok(_) => true,
                Err(e) => {
                    errors.push(format!("Cannot create database directory: {}", e));
                    false
                }
            }
        } else {
            // Test write access by creating a temp file
            let test_file = parent.join(".write_test");
            match fs::write(&test_file, "test") {
                Ok(_) => {
                    let _ = fs::remove_file(&test_file);
                    true
                }
                Err(e) => {
                    errors.push(format!("Directory not writable: {}", e));
                    false
                }
            }
        }
    } else {
        errors.push("Invalid database path - no parent directory".to_string());
        false
    };

    // Try to connect to database
    let connection = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            errors.push(format!("Cannot connect to database: {}", e));
            return Ok(DatabaseHealth {
                is_healthy: false,
                can_connect: false,
                can_read: false,
                can_write: false,
                foreign_keys_enabled: false,
                wal_mode: false,
                tables_exist: false,
                indexes_exist: false,
                path_accessible,
                directory_writable,
                last_check: chrono::Utc::now().timestamp(),
                check_duration_ms: start_time.elapsed().as_millis() as u64,
                errors,
                warnings,
            });
        }
    };

    let can_connect = true;

    // Test read capability
    let can_read = match connection.execute("SELECT 1", params![]) {
        Ok(_) => true,
        Err(e) => {
            errors.push(format!("Cannot read from database: {}", e));
            false
        }
    };

    // Test write capability with a temporary table
    let can_write = match connection.execute(
        "CREATE TEMP TABLE health_test (id INTEGER PRIMARY KEY)",
        params![]
    ) {
        Ok(_) => {
            let _ = connection.execute("DROP TABLE health_test", params![]);
            true
        }
        Err(e) => {
            errors.push(format!("Cannot write to database: {}", e));
            false
        }
    };

    // Check foreign keys
    let foreign_keys_enabled = match connection.query_row(
        "PRAGMA foreign_keys",
        params![],
        |row| row.get::<_, i32>(0)
    ) {
        Ok(val) => val == 1,
        Err(e) => {
            warnings.push(format!("Cannot check foreign_keys setting: {}", e));
            false
        }
    };

    // Check WAL mode
    let wal_mode = match connection.query_row(
        "PRAGMA journal_mode",
        params![],
        |row| row.get::<_, String>(0)
    ) {
        Ok(mode) => mode.to_lowercase() == "wal",
        Err(e) => {
            warnings.push(format!("Cannot check journal mode: {}", e));
            false
        }
    };

    // Check required tables exist
    let required_tables = vec![
        "chat_sessions", "chat_messages", "message_attachments",
        "thinking_processes", "thinking_steps", "message_metadata",
        "conversation_sessions", "conversation_messages", "conversation_insights"
    ];

    let mut missing_tables = Vec::new();
    for table in &required_tables {
        match connection.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
            params![table],
            |_| Ok(())
        ) {
            Ok(_) => {},
            Err(SqliteError::QueryReturnedNoRows) => {
                missing_tables.push(table.to_string());
            }
            Err(e) => {
                errors.push(format!("Error checking table {}: {}", table, e));
            }
        }
    }

    let tables_exist = missing_tables.is_empty();
    if !tables_exist {
        warnings.push(format!("Missing tables: {}", missing_tables.join(", ")));
    }

    // Check indexes exist
    let required_indexes = vec![
        "idx_chat_sessions_updated_desc",
        "idx_chat_messages_session_timestamp", 
        "idx_conversation_sessions_active_start",
        "idx_conversation_messages_session_timestamp"
    ];

    let mut missing_indexes = Vec::new();
    for index in &required_indexes {
        match connection.query_row(
            "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
            params![index],
            |_| Ok(())
        ) {
            Ok(_) => {},
            Err(SqliteError::QueryReturnedNoRows) => {
                missing_indexes.push(index.to_string());
            }
            Err(e) => {
                warnings.push(format!("Error checking index {}: {}", index, e));
            }
        }
    }

    let indexes_exist = missing_indexes.is_empty();
    if !indexes_exist {
        warnings.push(format!("Missing indexes: {}", missing_indexes.join(", ")));
    }

    let is_healthy = errors.is_empty() && can_connect && can_read && can_write && 
                     tables_exist && directory_writable && path_accessible;

    if !foreign_keys_enabled {
        warnings.push("Foreign keys are not enabled".to_string());
    }
    if !wal_mode {
        warnings.push("Database is not using WAL mode".to_string());
    }

    Ok(DatabaseHealth {
        is_healthy,
        can_connect,
        can_read,
        can_write,
        foreign_keys_enabled,
        wal_mode,
        tables_exist,
        indexes_exist,
        path_accessible,
        directory_writable,
        last_check: chrono::Utc::now().timestamp(),
        check_duration_ms: start_time.elapsed().as_millis() as u64,
        errors,
        warnings,
    })
}

/// Initialize the SQLite database with all necessary tables and comprehensive error handling
#[command]
pub fn initialize_database(app_handle: AppHandle) -> Result<String, String> {
    println!("🔧 Starting database initialization...");
    
    // First perform health checks on the environment
    let db_path = get_database_path(&app_handle)?;
    println!("📍 Database path: {:?}", db_path);
    
    // Ensure parent directory exists and is writable
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            println!("📁 Creating database directory: {:?}", parent);
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
        
        // Test directory writability
        let test_file = parent.join(".init_test");
        fs::write(&test_file, "test")
            .map_err(|e| format!("Directory not writable {}: {}", parent.display(), e))?;
        let _ = fs::remove_file(&test_file);
        println!("✅ Directory is writable");
    }

    // Create connection with error handling
    println!("🔗 Opening database connection...");
    let connection = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database at {}: {}", db_path.display(), e))?;
    
    println!("⚙️ Configuring SQLite settings...");
    
    // Configure SQLite for optimal performance with comprehensive error handling
    connection.execute("PRAGMA foreign_keys = ON", params![])
        .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
    println!("✅ Foreign keys enabled");
    
    // Set journal mode to WAL for better concurrency (WAL returns a result, so use query_row)
    match connection.query_row("PRAGMA journal_mode = WAL", params![], |row| row.get::<_, String>(0)) {
        Ok(mode) => {
            if mode.to_lowercase() == "wal" {
                println!("✅ WAL mode enabled successfully");
            } else {
                println!("ℹ️ Journal mode is: {} (WAL may not be available)", mode);
            }
        }
        Err(e) => {
            println!("⚠️ Warning: Could not set journal mode: {}", e);
            // Continue anyway - WAL mode is optional
        }
    }
    
    // Set other performance pragmas
    connection.execute("PRAGMA synchronous = NORMAL", params![])
        .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
    connection.execute("PRAGMA cache_size = 10000", params![])
        .map_err(|e| format!("Failed to set cache size: {}", e))?;
    connection.execute("PRAGMA temp_store = memory", params![])
        .map_err(|e| format!("Failed to set temp store: {}", e))?;
    println!("✅ Performance settings applied");
    
    // Create all tables with comprehensive schema
    println!("🏗️ Creating database schema...");
    let schema = get_database_schema();
    connection.execute_batch(&schema)
        .map_err(|e| format!("Failed to create database schema: {}", e))?;
    
    // Verify schema creation
    println!("🔍 Verifying schema creation...");
    let health = check_database_health(app_handle.clone())
        .map_err(|e| format!("Failed to verify database health: {}", e))?;
    
    if !health.is_healthy {
        let error_msg = format!(
            "Database initialization verification failed. Errors: {:?}, Warnings: {:?}", 
            health.errors, health.warnings
        );
        return Err(error_msg);
    }
    
    println!("✅ Database initialized successfully at: {:?}", db_path);
    println!("📊 Health check passed - database is ready for use");
    
    Ok(format!("Database initialized successfully at: {} (Health: {})", 
        db_path.display(), 
        if health.is_healthy { "HEALTHY" } else { "WARNING" }
    ))
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

/// Get the complete database schema
fn get_database_schema() -> String {
    r#"
    -- Chat sessions table
    CREATE TABLE IF NOT EXISTS chat_sessions (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        model_id TEXT
    );

    -- Chat messages table
    CREATE TABLE IF NOT EXISTS chat_messages (
        id INTEGER PRIMARY KEY,
        session_id TEXT NOT NULL,
        text TEXT NOT NULL,
        sender TEXT NOT NULL CHECK(sender IN ('user', 'assistant', 'transcription', 'system')),
        timestamp TEXT NOT NULL,
        is_interim INTEGER CHECK(is_interim IN (0, 1)),
        confidence REAL,
        source TEXT,
        message_type TEXT,
        FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
    );

    -- Message attachments table
    CREATE TABLE IF NOT EXISTS message_attachments (
        id TEXT PRIMARY KEY,
        message_id INTEGER NOT NULL,
        type TEXT NOT NULL,
        name TEXT NOT NULL,
        size INTEGER NOT NULL,
        mime_type TEXT NOT NULL,
        url TEXT,
        base64_data TEXT,
        thumbnail TEXT,
        extracted_text TEXT,
        width INTEGER,
        height INTEGER,
        upload_progress INTEGER,
        upload_status TEXT,
        error TEXT,
        FOREIGN KEY (message_id) REFERENCES chat_messages(id) ON DELETE CASCADE
    );

    -- Thinking processes table
    CREATE TABLE IF NOT EXISTS thinking_processes (
        id INTEGER PRIMARY KEY,
        message_id INTEGER NOT NULL,
        is_visible INTEGER NOT NULL CHECK(is_visible IN (0, 1)),
        content TEXT NOT NULL,
        is_streaming INTEGER NOT NULL CHECK(is_streaming IN (0, 1)),
        FOREIGN KEY (message_id) REFERENCES chat_messages(id) ON DELETE CASCADE
    );

    -- Thinking steps table
    CREATE TABLE IF NOT EXISTS thinking_steps (
        id TEXT PRIMARY KEY,
        thinking_id INTEGER NOT NULL,
        title TEXT NOT NULL,
        content TEXT NOT NULL,
        timestamp TEXT NOT NULL,
        status TEXT NOT NULL,
        FOREIGN KEY (thinking_id) REFERENCES thinking_processes(id) ON DELETE CASCADE
    );

    -- Message metadata table
    CREATE TABLE IF NOT EXISTS message_metadata (
        id INTEGER PRIMARY KEY,
        message_id INTEGER NOT NULL,
        agent_type TEXT,
        model TEXT,
        tokens INTEGER,
        processing_time REAL,
        analysis_types TEXT, -- JSON array stored as text
        search_queries TEXT, -- JSON array stored as text
        sources TEXT, -- JSON array stored as text
        FOREIGN KEY (message_id) REFERENCES chat_messages(id) ON DELETE CASCADE
    );

    -- Conversation sessions table
    CREATE TABLE IF NOT EXISTS conversation_sessions (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        start_time INTEGER NOT NULL,
        end_time INTEGER,
        is_active INTEGER NOT NULL CHECK(is_active IN (0, 1))
    );

    -- Conversation messages table
    CREATE TABLE IF NOT EXISTS conversation_messages (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        type TEXT NOT NULL CHECK(type IN ('user', 'system')),
        source TEXT NOT NULL CHECK(source IN ('microphone', 'loopback')),
        content TEXT NOT NULL,
        timestamp INTEGER NOT NULL,
        confidence REAL,
        FOREIGN KEY (session_id) REFERENCES conversation_sessions(id) ON DELETE CASCADE
    );

    -- Conversation insights table
    CREATE TABLE IF NOT EXISTS conversation_insights (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        text TEXT NOT NULL,
        timestamp INTEGER NOT NULL,
        context_length INTEGER NOT NULL,
        insight_type TEXT NOT NULL CHECK(insight_type IN ('insight', 'welcome', 'question', 'answer')),
        FOREIGN KEY (session_id) REFERENCES conversation_sessions(id) ON DELETE CASCADE
    );

    -- Performance indexes for chat system
    CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated_desc ON chat_sessions(updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_chat_messages_session_timestamp ON chat_messages(session_id, timestamp);
    CREATE INDEX IF NOT EXISTS idx_message_attachments_message ON message_attachments(message_id);
    CREATE INDEX IF NOT EXISTS idx_thinking_processes_message ON thinking_processes(message_id);
    CREATE INDEX IF NOT EXISTS idx_thinking_steps_thinking ON thinking_steps(thinking_id);
    CREATE INDEX IF NOT EXISTS idx_message_metadata_message ON message_metadata(message_id);

    -- Performance indexes for conversation system
    CREATE INDEX IF NOT EXISTS idx_conversation_sessions_active_start ON conversation_sessions(is_active, start_time DESC);
    CREATE INDEX IF NOT EXISTS idx_conversation_messages_session_timestamp ON conversation_messages(session_id, timestamp);
    CREATE INDEX IF NOT EXISTS idx_conversation_messages_type ON conversation_messages(type);
    CREATE INDEX IF NOT EXISTS idx_conversation_messages_source ON conversation_messages(source);
    CREATE INDEX IF NOT EXISTS idx_conversation_insights_session_timestamp ON conversation_insights(session_id, timestamp);
    CREATE INDEX IF NOT EXISTS idx_conversation_insights_type ON conversation_insights(insight_type);
    "#.to_string()
}

// Helper function to get database path
fn get_database_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    Ok(app_data_dir.join("enteract_data.db"))
}