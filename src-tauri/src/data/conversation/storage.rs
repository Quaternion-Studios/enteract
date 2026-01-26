// SQLite storage implementation for conversation sessions
use rusqlite::{Connection, Result, params};
use tauri::{AppHandle, Manager};
use crate::data::types::{
    ConversationSession, ConversationMessage, ConversationInsight, ConversationMessageUpdate,
    SaveConversationsPayload, LoadConversationsResponse
};
use std::path::PathBuf;
use uuid::Uuid;
use serde_json;

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
                audio_level REAL,
                processing_latency_ms INTEGER,
                model_version TEXT,
                is_partial INTEGER CHECK(is_partial IN (0, 1)),
                merged_from TEXT,
                speaker_id TEXT,
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

        // Migrate existing tables to add new columns if they don't exist
        self.migrate_conversation_messages_schema()?;

        // Create C1 tables if they don't exist (for databases created before C1)
        self.migrate_c1_tables()?;

        Ok(())
    }

    /// Create conversation_state and context_windows tables from C1 schema
    /// This handles existing databases created before C1 implementation
    fn migrate_c1_tables(&mut self) -> Result<()> {
        // Check if conversation_state table exists
        let state_exists: bool = self.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='conversation_state'",
            [],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            }
        )?;

        if !state_exists {
            println!("🔧 Creating conversation_state table (C1 migration)");
            self.connection.execute(r#"
                CREATE TABLE conversation_state (
                    session_id TEXT PRIMARY KEY,
                    state TEXT NOT NULL CHECK(state IN ('idle', 'listening', 'processing', 'responding')),
                    last_user_speech_at INTEGER,
                    last_system_speech_at INTEGER,
                    context_summary TEXT,
                    topic TEXT,
                    updated_at INTEGER NOT NULL,
                    FOREIGN KEY (session_id) REFERENCES conversation_sessions(id) ON DELETE CASCADE
                )
            "#, params![])?;
            println!("✅ Created conversation_state table");
        }

        // Check if context_windows table exists
        let context_exists: bool = self.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='context_windows'",
            [],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            }
        )?;

        if !context_exists {
            println!("🔧 Creating context_windows table (C1 migration)");
            self.connection.execute(r#"
                CREATE TABLE context_windows (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    messages_json TEXT NOT NULL,
                    token_count INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    FOREIGN KEY (session_id) REFERENCES conversation_sessions(id) ON DELETE CASCADE
                )
            "#, params![])?;

            // Add index for performance
            self.connection.execute(
                "CREATE INDEX IF NOT EXISTS idx_context_windows_session_created ON context_windows(session_id, created_at DESC)",
                params![]
            )?;
            println!("✅ Created context_windows table");
        }

        Ok(())
    }

    /// Migrate conversation_messages table to add new columns from C1 schema
    /// This handles existing databases created before the schema enhancement
    fn migrate_conversation_messages_schema(&mut self) -> Result<()> {
        // Check which columns exist
        let mut stmt = self.connection.prepare("PRAGMA table_info(conversation_messages)")?;
        let existing_columns: Vec<String> = stmt.query_map([], |row| {
            row.get::<_, String>(1) // Column 1 is the name
        })?.collect::<Result<Vec<_>, _>>()?;

        // Columns to add if missing
        let required_columns = vec![
            ("audio_level", "ALTER TABLE conversation_messages ADD COLUMN audio_level REAL"),
            ("processing_latency_ms", "ALTER TABLE conversation_messages ADD COLUMN processing_latency_ms INTEGER"),
            ("model_version", "ALTER TABLE conversation_messages ADD COLUMN model_version TEXT"),
            ("is_partial", "ALTER TABLE conversation_messages ADD COLUMN is_partial INTEGER CHECK(is_partial IN (0, 1))"),
            ("merged_from", "ALTER TABLE conversation_messages ADD COLUMN merged_from TEXT"),
            ("speaker_id", "ALTER TABLE conversation_messages ADD COLUMN speaker_id TEXT"),
        ];

        for (col_name, alter_sql) in required_columns {
            if !existing_columns.contains(&col_name.to_string()) {
                println!("🔧 Migrating conversation_messages: adding column {}", col_name);
                self.connection.execute(alter_sql, params![])?;
                println!("✅ Added column: {}", col_name);
            }
        }

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

    /// Merge multiple partial messages into a single complete message
    /// Returns the ID of the new merged message
    pub fn merge_partial_messages(&mut self, session_id: &str, partial_message_ids: Vec<String>) -> Result<String> {
        if partial_message_ids.is_empty() {
            return Err(rusqlite::Error::InvalidParameterName("No messages to merge".to_string()));
        }

        println!("🔀 Merging {} partial messages", partial_message_ids.len());

        // Fetch all partial messages
        let mut partial_messages = Vec::new();
        for msg_id in &partial_message_ids {
            let message = self.connection.query_row(
                "SELECT id, type, source, content, timestamp, confidence,
                        audio_level, processing_latency_ms, model_version,
                        is_partial, merged_from, speaker_id
                 FROM conversation_messages
                 WHERE id = ? AND session_id = ?",
                params![msg_id, session_id],
                |row| {
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
                        is_preview: None,
                        is_typing: None,
                        persistence_state: None,
                        retry_count: None,
                        last_save_attempt: None,
                        save_error: None,
                    })
                },
            )?;
            partial_messages.push(message);
        }

        // Sort by timestamp to merge in chronological order
        partial_messages.sort_by_key(|m| m.timestamp);

        // Build merged content
        let merged_content = partial_messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Calculate average confidence
        let avg_confidence = if partial_messages.iter().any(|m| m.confidence.is_some()) {
            let confidences: Vec<f64> = partial_messages.iter()
                .filter_map(|m| m.confidence)
                .collect();
            if !confidences.is_empty() {
                Some(confidences.iter().sum::<f64>() / confidences.len() as f64)
            } else {
                None
            }
        } else {
            None
        };

        // Use earliest timestamp and latest audio_level/latency
        let earliest_msg = &partial_messages[0];
        let latest_msg = &partial_messages[partial_messages.len() - 1];

        // Create merged_from JSON array
        let merged_from_json = serde_json::to_string(&partial_message_ids)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        // Generate new ID for merged message
        let merged_id = Uuid::new_v4().to_string();

        // Create merged message
        let merged_message = ConversationMessage {
            id: merged_id.clone(),
            message_type: earliest_msg.message_type.clone(),
            source: earliest_msg.source.clone(),
            content: merged_content,
            timestamp: earliest_msg.timestamp,
            confidence: avg_confidence,
            audio_level: latest_msg.audio_level,
            processing_latency_ms: latest_msg.processing_latency_ms,
            model_version: latest_msg.model_version.clone(),
            is_partial: Some(false), // Mark as complete
            merged_from: Some(merged_from_json),
            speaker_id: earliest_msg.speaker_id.clone(),
            is_preview: None,
            is_typing: None,
            persistence_state: None,
            retry_count: None,
            last_save_attempt: None,
            save_error: None,
        };

        // Save merged message
        self.save_conversation_message(session_id, merged_message)?;

        // Delete original partial messages
        for msg_id in &partial_message_ids {
            self.connection.execute(
                "DELETE FROM conversation_messages WHERE id = ? AND session_id = ?",
                params![msg_id, session_id]
            )?;
        }

        println!("✅ Merged {} partial messages into {}", partial_message_ids.len(), merged_id);
        Ok(merged_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn create_test_storage() -> ConversationStorage {
        let conn = Connection::open_in_memory().unwrap();
        let mut storage = ConversationStorage { connection: conn };
        storage.initialize_conversation_tables().unwrap();
        storage
    }

    #[test]
    fn test_merge_partial_messages() {
        let mut storage = create_test_storage();
        let session_id = "test-session";

        // Create session
        storage.connection.execute(
            "INSERT INTO conversation_sessions (id, name, start_time, is_active) VALUES (?, ?, ?, ?)",
            params![session_id, "Test Session", 1000, 1]
        ).unwrap();

        // Create partial messages
        let partial_ids = vec![
            "msg-1".to_string(),
            "msg-2".to_string(),
            "msg-3".to_string(),
        ];

        let partial_messages = vec![
            ConversationMessage {
                id: "msg-1".to_string(),
                message_type: "user".to_string(),
                source: "microphone".to_string(),
                content: "Hello".to_string(),
                timestamp: 1000,
                confidence: Some(0.9),
                audio_level: Some(0.5),
                processing_latency_ms: Some(100),
                model_version: Some("v1".to_string()),
                is_partial: Some(true),
                merged_from: None,
                speaker_id: Some("speaker1".to_string()),
                is_preview: None,
                is_typing: None,
                persistence_state: None,
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            },
            ConversationMessage {
                id: "msg-2".to_string(),
                message_type: "user".to_string(),
                source: "microphone".to_string(),
                content: "world".to_string(),
                timestamp: 2000,
                confidence: Some(0.8),
                audio_level: Some(0.6),
                processing_latency_ms: Some(150),
                model_version: Some("v1".to_string()),
                is_partial: Some(true),
                merged_from: None,
                speaker_id: Some("speaker1".to_string()),
                is_preview: None,
                is_typing: None,
                persistence_state: None,
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            },
            ConversationMessage {
                id: "msg-3".to_string(),
                message_type: "user".to_string(),
                source: "microphone".to_string(),
                content: "today".to_string(),
                timestamp: 3000,
                confidence: Some(0.95),
                audio_level: Some(0.7),
                processing_latency_ms: Some(120),
                model_version: Some("v1".to_string()),
                is_partial: Some(true),
                merged_from: None,
                speaker_id: Some("speaker1".to_string()),
                is_preview: None,
                is_typing: None,
                persistence_state: None,
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            },
        ];

        // Save partial messages
        for msg in partial_messages {
            storage.save_conversation_message(session_id, msg).unwrap();
        }

        // Merge them
        let merged_id = storage.merge_partial_messages(session_id, partial_ids.clone()).unwrap();
        assert!(!merged_id.is_empty());

        // Verify merged message exists
        let merged_msg = storage.connection.query_row(
            "SELECT content, is_partial, merged_from, confidence
             FROM conversation_messages WHERE id = ?",
            params![&merged_id],
            |row| {
                Ok((
                    row.get::<_, String>("content")?,
                    row.get::<_, Option<i32>>("is_partial")?,
                    row.get::<_, String>("merged_from")?,
                    row.get::<_, f64>("confidence")?,
                ))
            }
        ).unwrap();

        // Check merged content
        assert_eq!(merged_msg.0, "Hello world today");

        // Check is_partial is false
        assert_eq!(merged_msg.1, Some(0));

        // Check merged_from contains original IDs
        let merged_from: Vec<String> = serde_json::from_str(&merged_msg.2).unwrap();
        assert_eq!(merged_from, partial_ids);

        // Check average confidence (0.9 + 0.8 + 0.95) / 3 = 0.8833...
        assert!((merged_msg.3 - 0.8833).abs() < 0.01);

        // Verify original messages are deleted
        let count: i32 = storage.connection.query_row(
            "SELECT COUNT(*) FROM conversation_messages WHERE id IN (?, ?, ?)",
            params!["msg-1", "msg-2", "msg-3"],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_merge_partial_messages_empty() {
        let mut storage = create_test_storage();
        let result = storage.merge_partial_messages("session", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_migration() {
        // Create a database with OLD schema (no C1 columns)
        let conn = Connection::open_in_memory().unwrap();

        // Create old schema (conversation_messages without new columns)
        conn.execute_batch(r#"
            CREATE TABLE conversation_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                is_active INTEGER NOT NULL
            );

            CREATE TABLE conversation_messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                type TEXT NOT NULL,
                source TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                confidence REAL,
                FOREIGN KEY (session_id) REFERENCES conversation_sessions(id)
            );
        "#).unwrap();

        // Verify old schema (should have 7 columns only)
        let old_columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(conversation_messages)").unwrap();
            stmt.query_map([], |row| {
                row.get::<_, String>(1)
            }).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
        };

        assert_eq!(old_columns.len(), 7);
        assert!(!old_columns.contains(&"audio_level".to_string()));

        // Now run migration
        let mut storage = ConversationStorage { connection: conn };
        storage.migrate_conversation_messages_schema().unwrap();

        // Verify new columns were added
        let mut stmt = storage.connection.prepare("PRAGMA table_info(conversation_messages)").unwrap();
        let new_columns: Vec<String> = stmt.query_map([], |row| {
            row.get::<_, String>(1)
        }).unwrap().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(new_columns.len(), 13); // 7 old + 6 new = 13
        assert!(new_columns.contains(&"audio_level".to_string()));
        assert!(new_columns.contains(&"processing_latency_ms".to_string()));
        assert!(new_columns.contains(&"model_version".to_string()));
        assert!(new_columns.contains(&"is_partial".to_string()));
        assert!(new_columns.contains(&"merged_from".to_string()));
        assert!(new_columns.contains(&"speaker_id".to_string()));
    }

    #[test]
    fn test_c1_tables_migration() {
        // Create database without C1 tables
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(r#"
            CREATE TABLE conversation_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                is_active INTEGER NOT NULL
            );
        "#).unwrap();

        // Verify C1 tables don't exist
        let state_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='conversation_state'",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(state_exists, 0);

        let context_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='context_windows'",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(context_exists, 0);

        // Run migration
        let mut storage = ConversationStorage { connection: conn };
        storage.migrate_c1_tables().unwrap();

        // Verify C1 tables now exist
        let state_exists: i32 = storage.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='conversation_state'",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(state_exists, 1);

        let context_exists: i32 = storage.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='context_windows'",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(context_exists, 1);
    }
}