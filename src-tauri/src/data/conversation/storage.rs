// SQLite storage implementation for conversation sessions
use rusqlite::{Connection, Result, params};
use tauri::{AppHandle, Manager};
use crate::data::types::{
    ConversationSession, ConversationMessage, ConversationInsight, ConversationMessageUpdate,
    SaveConversationsPayload, LoadConversationsResponse
};
use std::path::PathBuf;

pub struct ConversationStorage {
    connection: Connection,
}

impl ConversationStorage {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let db_path = get_database_path(app_handle).map_err(|e| rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            Some(e)
        ))?;
        
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_IOERR),
                        Some(format!("Failed to create directory: {}", e))
                    ))?;
            }
        }

        let connection = Connection::open(&db_path)?;
        println!("ℹ️ Opened database connection at: {:?}", db_path);
        
        // Configure SQLite for optimal performance using safer approach
        connection.execute("PRAGMA foreign_keys = ON", params![]).map_err(|e| {
            println!("⚠️ Warning: Failed to set foreign_keys: {}", e);
            e
        })?;
        
        // Set journal mode with proper handling (WAL returns a result, so use query_row)
        match connection.query_row("PRAGMA journal_mode = WAL", params![], |row| row.get::<_, String>(0)) {
            Ok(mode) => {
                if mode.to_lowercase() == "wal" {
                    println!("✅ WAL mode enabled successfully");
                } else {
                    println!("ℹ️ Journal mode is: {} (WAL may not be available)", mode);
                }
            }
            Err(e) => println!("⚠️ Warning: Could not set journal mode: {}", e),
        }
        
        // Set other pragmas with execute (they don't necessarily return meaningful results)
        connection.execute("PRAGMA synchronous = NORMAL", params![]).ok();
        connection.execute("PRAGMA cache_size = 10000", params![]).ok();
        connection.execute("PRAGMA temp_store = memory", params![]).ok();
        
        println!("✅ SQLite configuration applied successfully");
        
        let mut storage = Self { connection };
        storage.initialize_conversation_tables()?;
        
        Ok(storage)
    }

    fn initialize_conversation_tables(&mut self) -> Result<()> {
        // Create conversation-specific tables
        self.connection.execute_batch(r#"
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

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_conversation_sessions_active_start ON conversation_sessions(is_active, start_time DESC);
            CREATE INDEX IF NOT EXISTS idx_conversation_messages_session_timestamp ON conversation_messages(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_conversation_messages_type ON conversation_messages(type);
            CREATE INDEX IF NOT EXISTS idx_conversation_messages_source ON conversation_messages(source);
            CREATE INDEX IF NOT EXISTS idx_conversation_insights_session_timestamp ON conversation_insights(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_conversation_insights_type ON conversation_insights(insight_type);
        "#)?;

        println!("✅ Conversation tables initialized successfully");
        Ok(())
    }

    pub fn save_conversations(&mut self, payload: SaveConversationsPayload) -> Result<()> {
        // Use incremental updates instead of full table replacement to avoid race conditions
        println!("🔄 Using incremental session updates for {} sessions", payload.conversations.len());
        
        let mut updated_count = 0;
        let mut created_count = 0;
        
        for session in payload.conversations {
            let session_id = session.id.clone(); // Clone for error message
            match self.save_or_update_session(session) {
                Ok(was_created) => {
                    if was_created {
                        created_count += 1;
                    } else {
                        updated_count += 1;
                    }
                }
                Err(e) => {
                    println!("❌ Failed to save session {}: {}", session_id, e);
                    return Err(e);
                }
            }
        }
        
        println!("✅ Session operations complete: {} updated, {} created", updated_count, created_count);
        Ok(())
    }

    /// Save or update a single session with all its data incrementally
    pub fn save_or_update_session(&mut self, session: ConversationSession) -> Result<bool> {
        let tx = self.connection.transaction()?;
        
        // Check if session exists
        let session_exists: bool = match tx.query_row(
            "SELECT 1 FROM conversation_sessions WHERE id = ? LIMIT 1",
            params![session.id],
            |_| Ok(true)
        ) {
            Ok(_) => true,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => return Err(e),
        };
        
        let was_created = !session_exists;
        
        if session_exists {
            // Update existing session metadata only
            tx.execute(
                "UPDATE conversation_sessions SET name = ?, start_time = ?, end_time = ?, is_active = ? WHERE id = ?",
                params![
                    session.name, session.start_time, session.end_time,
                    if session.is_active { 1 } else { 0 }, session.id
                ]
            )?;
            println!("🔄 Updated session metadata: {}", session.id);
        } else {
            // Insert new session
            tx.execute(
                "INSERT INTO conversation_sessions (id, name, start_time, end_time, is_active) VALUES (?, ?, ?, ?, ?)",
                params![
                    session.id, session.name, session.start_time, session.end_time,
                    if session.is_active { 1 } else { 0 }
                ]
            )?;
            println!("🆕 Created new session: {}", session.id);
        }

        // Handle messages incrementally (avoid conflicts with individual message saves)
        for message in session.messages {
            // Use INSERT OR IGNORE to avoid conflicts with concurrent individual message saves
            tx.execute(
                "INSERT OR IGNORE INTO conversation_messages (id, session_id, type, source, content, timestamp, confidence) 
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    message.id, session.id, message.message_type, message.source,
                    message.content, message.timestamp, message.confidence
                ]
            )?;
        }

        // Handle insights incrementally
        for insight in session.insights {
            tx.execute(
                "INSERT OR REPLACE INTO conversation_insights (id, session_id, text, timestamp, context_length, insight_type)
                 VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    insight.id, session.id, insight.text, insight.timestamp,
                    insight.context_length, insight.insight_type
                ]
            )?;
        }

        tx.commit()?;
        Ok(was_created)
    }

    /// Update only session metadata fields (optimized for session state changes)
    pub fn update_session_metadata(&mut self, session_id: &str, name: Option<&str>, end_time: Option<Option<i64>>, is_active: Option<bool>) -> Result<()> {
        let mut set_clauses = Vec::new();
        let mut params = Vec::new();
        
        if let Some(name) = name {
            set_clauses.push("name = ?");
            params.push(rusqlite::types::Value::Text(name.to_string()));
        }
        if let Some(end_time) = end_time {
            set_clauses.push("end_time = ?");
            match end_time {
                Some(time) => params.push(rusqlite::types::Value::Integer(time)),
                None => params.push(rusqlite::types::Value::Null),
            }
        }
        if let Some(is_active) = is_active {
            set_clauses.push("is_active = ?");
            params.push(rusqlite::types::Value::Integer(if is_active { 1 } else { 0 }));
        }
        
        if set_clauses.is_empty() {
            return Ok(()); // No updates to apply
        }
        
        // Add session_id for WHERE clause
        params.push(rusqlite::types::Value::Text(session_id.to_string()));
        
        let sql = format!(
            "UPDATE conversation_sessions SET {} WHERE id = ?",
            set_clauses.join(", ")
        );
        
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let affected = self.connection.execute(&sql, param_refs.as_slice())?;
        
        if affected == 0 {
            println!("⚠️ No session found to update: {}", session_id);
        } else {
            println!("✅ Updated session metadata for: {}", session_id);
        }
        
        Ok(())
    }

    /// Activate/deactivate sessions (common operation during session switching)
    pub fn update_session_active_state(&mut self, session_id: &str, is_active: bool) -> Result<()> {
        let affected = self.connection.execute(
            "UPDATE conversation_sessions SET is_active = ? WHERE id = ?",
            params![if is_active { 1 } else { 0 }, session_id]
        )?;
        
        if affected == 0 {
            println!("⚠️ No session found to update active state: {}", session_id);
        } else {
            println!("✅ Updated session {} active state to: {}", session_id, is_active);
        }
        
        Ok(())
    }

    pub fn load_conversations(&self) -> Result<LoadConversationsResponse> {
        let mut sessions = Vec::new();

        // Query all sessions
        let mut session_stmt = self.connection.prepare(
            "SELECT id, name, start_time, end_time, is_active FROM conversation_sessions ORDER BY start_time DESC"
        )?;

        let session_iter = session_stmt.query_map(params![], |row| {
            Ok((
                row.get::<_, String>("id")?,
                row.get::<_, String>("name")?,
                row.get::<_, i64>("start_time")?,
                row.get::<_, Option<i64>>("end_time")?,
                row.get::<_, i32>("is_active")? != 0,
            ))
        })?;

        for session_result in session_iter {
            let (id, name, start_time, end_time, is_active) = session_result?;
            
            // Load messages and insights for this session
            let messages = self.load_conversation_messages(&id)?;
            let insights = self.load_conversation_insights(&id)?;

            sessions.push(ConversationSession {
                id,
                name,
                start_time,
                end_time,
                is_active,
                messages,
                insights,
            });
        }

        println!("✅ Loaded {} conversation sessions from SQLite", sessions.len());
        Ok(LoadConversationsResponse { conversations: sessions })
    }

    fn load_conversation_messages(&self, session_id: &str) -> Result<Vec<ConversationMessage>> {
        let mut messages = Vec::new();

        let mut stmt = self.connection.prepare(
            "SELECT id, type, source, content, timestamp, confidence,
             audio_level, processing_latency_ms, model_version,
             is_partial, merged_from, speaker_id
             FROM conversation_messages WHERE session_id = ? ORDER BY timestamp"
        )?;

        let message_iter = stmt.query_map([session_id], |row| {
            Ok(ConversationMessage {
                id: row.get("id")?,
                message_type: row.get("type")?,
                source: row.get("source")?,
                content: row.get("content")?,
                timestamp: row.get("timestamp")?,
                confidence: row.get("confidence")?,
                audio_level: row.get("audio_level")?,
                processing_latency_ms: row.get("processing_latency_ms")?,
                model_version: row.get("model_version")?,
                is_partial: row.get::<_, Option<i32>>("is_partial")?.map(|v| v != 0),
                merged_from: row.get("merged_from")?,
                speaker_id: row.get("speaker_id")?,
                // Frontend-only fields set to None when loading from DB
                is_preview: None,
                is_typing: None,
                persistence_state: Some("saved".to_string()),
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            })
        })?;

        for message_result in message_iter {
            messages.push(message_result?);
        }

        Ok(messages)
    }

    fn load_conversation_insights(&self, session_id: &str) -> Result<Vec<ConversationInsight>> {
        let mut insights = Vec::new();

        let mut stmt = self.connection.prepare(
            "SELECT id, text, timestamp, context_length, insight_type 
             FROM conversation_insights WHERE session_id = ? ORDER BY timestamp"
        )?;

        let insight_iter = stmt.query_map([session_id], |row| {
            Ok(ConversationInsight {
                id: row.get("id")?,
                text: row.get("text")?,
                timestamp: row.get("timestamp")?,
                context_length: row.get("context_length")?,
                insight_type: row.get("insight_type")?,
            })
        })?;

        for insight_result in insight_iter {
            insights.push(insight_result?);
        }

        Ok(insights)
    }

    // Individual message operations
    pub fn save_conversation_message(&mut self, session_id: &str, message: ConversationMessage) -> Result<()> {
        println!("🔍 Attempting to save message: id={}, type={}, source={}", 
                 message.id, message.message_type, message.source);
        
        // Check if message already exists (deduplication)
        let exists: bool = match self.connection.query_row(
            "SELECT 1 FROM conversation_messages WHERE id = ? LIMIT 1",
            params![message.id],
            |_| Ok(true)
        ) {
            Ok(val) => val,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(e) => {
                println!("❌ Error checking message existence: {}", e);
                return Err(e);
            }
        };

        if exists {
            println!("⚠️ Message {} already exists in database, skipping duplicate", message.id);
            return Ok(()); // Message already saved - not an error
        }

        // Validate session exists
        let session_exists: bool = match self.connection.query_row(
            "SELECT 1 FROM conversation_sessions WHERE id = ? LIMIT 1",
            params![session_id],
            |_| Ok(true)
        ) {
            Ok(val) => val,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                println!("⚠️ Session {} does not exist, creating it first", session_id);
                false
            }
            Err(e) => {
                println!("❌ Error checking session existence: {}", e);
                return Err(e);
            }
        };

        if !session_exists {
            // Create a minimal session entry if it doesn't exist
            self.connection.execute(
                "INSERT OR IGNORE INTO conversation_sessions (id, name, start_time, end_time, is_active) 
                 VALUES (?, ?, ?, NULL, 1)",
                params![session_id, format!("Session {}", session_id), message.timestamp]
            ).map_err(|e| {
                println!("❌ Failed to create session {}: {}", session_id, e);
                e
            })?;
            println!("✅ Created missing session: {}", session_id);
        }

        let affected = self.connection.execute(
            "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp, confidence,
             audio_level, processing_latency_ms, model_version, is_partial, merged_from, speaker_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                message.id, session_id, message.message_type, message.source,
                message.content, message.timestamp, message.confidence,
                message.audio_level, message.processing_latency_ms, message.model_version,
                message.is_partial.map(|v| if v { 1 } else { 0 }), message.merged_from, message.speaker_id
            ]
        ).map_err(|e| {
            println!("❌ Failed to insert message: {}", e);
            println!("   Message details: id={}, session_id={}, type={}, source={}",
                     message.id, session_id, message.message_type, message.source);
            e
        })?;

        println!("✅ Successfully saved message {} to session {} (rows affected: {})", 
                 message.id, session_id, affected);
        Ok(())
    }

    pub fn batch_save_conversation_messages(&mut self, session_id: &str, messages: Vec<ConversationMessage>) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let tx = self.connection.transaction()?;
        let mut saved_count = 0;
        let mut skipped_count = 0;

        for message in &messages {
            // Check if message already exists (deduplication)
            let exists: bool = match tx.query_row(
                "SELECT 1 FROM conversation_messages WHERE id = ? LIMIT 1",
                params![message.id],
                |_| Ok(true)
            ) {
                Ok(val) => val,
                Err(rusqlite::Error::QueryReturnedNoRows) => false,
                Err(e) => return Err(e),
            };

            if !exists {
                tx.execute(
                    "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp, confidence,
                     audio_level, processing_latency_ms, model_version, is_partial, merged_from, speaker_id)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        message.id, session_id, message.message_type, message.source,
                        message.content, message.timestamp, message.confidence,
                        message.audio_level, message.processing_latency_ms, message.model_version,
                        message.is_partial.map(|v| if v { 1 } else { 0 }), message.merged_from, message.speaker_id
                    ]
                )?;
                saved_count += 1;
            } else {
                skipped_count += 1;
            }
        }

        tx.commit()?;
        println!("✅ Batch saved {} messages to session {}, skipped {} duplicates", saved_count, session_id, skipped_count);
        Ok(())
    }

    pub fn update_conversation_message(&mut self, session_id: &str, message_id: &str, updates: ConversationMessageUpdate) -> Result<()> {
        let mut set_clauses = Vec::new();
        let mut sql_params = Vec::new();

        if let Some(content) = updates.content {
            set_clauses.push("content = ?");
            sql_params.push(rusqlite::types::Value::Text(content));
        }
        if let Some(confidence) = updates.confidence {
            set_clauses.push("confidence = ?");
            sql_params.push(rusqlite::types::Value::Real(confidence));
        }
        if let Some(timestamp) = updates.timestamp {
            set_clauses.push("timestamp = ?");
            sql_params.push(rusqlite::types::Value::Integer(timestamp));
        }

        if set_clauses.is_empty() {
            return Ok(()); // No updates to apply
        }

        // Add message_id and session_id for WHERE clause
        sql_params.push(rusqlite::types::Value::Text(message_id.to_string()));
        sql_params.push(rusqlite::types::Value::Text(session_id.to_string()));

        let sql = format!(
            "UPDATE conversation_messages SET {} WHERE id = ? AND session_id = ?",
            set_clauses.join(", ")
        );

        let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let affected = self.connection.execute(&sql, param_refs.as_slice())?;
        
        if affected == 0 {
            println!("⚠️ No message found to update: {} in session {}", message_id, session_id);
        } else {
            println!("✅ Updated message {} in session {}", message_id, session_id);
        }

        Ok(())
    }

    pub fn delete_conversation_message(&mut self, session_id: &str, message_id: &str) -> Result<()> {
        let affected = self.connection.execute(
            "DELETE FROM conversation_messages WHERE id = ? AND session_id = ?",
            params![message_id, session_id]
        )?;

        if affected == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        Ok(())
    }

    pub fn save_conversation_insight(&mut self, session_id: &str, insight: ConversationInsight) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO conversation_insights (id, session_id, text, timestamp, context_length, insight_type)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                insight.id, session_id, insight.text, insight.timestamp,
                insight.context_length, insight.insight_type
            ]
        )?;

        Ok(())
    }

    pub fn get_conversation_insights(&self, session_id: &str) -> Result<Vec<ConversationInsight>> {
        self.load_conversation_insights(session_id)
    }

    pub fn delete_conversation(&mut self, conversation_id: &str) -> Result<()> {
        let affected = self.connection.execute(
            "DELETE FROM conversation_sessions WHERE id = ?",
            params![conversation_id]
        )?;

        if affected == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        Ok(())
    }

    pub fn clear_all_conversations(&mut self) -> Result<()> {
        self.connection.execute("DELETE FROM conversation_sessions", params![])?;
        Ok(())
    }
}

// Helper function to get database path
fn get_database_path(app_handle: &AppHandle) -> std::result::Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    Ok(app_data_dir.join("enteract_data.db"))
}