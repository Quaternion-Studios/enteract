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
        
        // Configure SQLite for optimal performance
        connection.execute("PRAGMA foreign_keys = ON", params![])?;
        connection.execute("PRAGMA journal_mode = WAL", params![])?;
        connection.execute("PRAGMA synchronous = NORMAL", params![])?;
        connection.execute("PRAGMA cache_size = 10000", params![])?;
        connection.execute("PRAGMA temp_store = memory", params![])?;
        
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

        Ok(())
    }

    pub fn save_conversations(&mut self, payload: SaveConversationsPayload) -> Result<()> {
        let tx = self.connection.transaction()?;

        // Clear existing data (full replacement for now - can be optimized later)
        tx.execute("DELETE FROM conversation_sessions", params![])?;

        let sessions_count = payload.conversations.len();
        for session in payload.conversations {
            // Insert session
            tx.execute(
                "INSERT INTO conversation_sessions (id, name, start_time, end_time, is_active) VALUES (?, ?, ?, ?, ?)",
                params![
                    session.id, session.name, session.start_time, session.end_time,
                    if session.is_active { 1 } else { 0 }
                ]
            )?;

            // Insert messages
            for message in session.messages {
                tx.execute(
                    "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp, confidence) 
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                    params![
                        message.id, session.id, message.message_type, message.source,
                        message.content, message.timestamp, message.confidence
                    ]
                )?;
            }

            // Insert insights
            for insight in session.insights {
                tx.execute(
                    "INSERT INTO conversation_insights (id, session_id, text, timestamp, context_length, insight_type)
                     VALUES (?, ?, ?, ?, ?, ?)",
                    params![
                        insight.id, session.id, insight.text, insight.timestamp,
                        insight.context_length, insight.insight_type
                    ]
                )?;
            }
        }

        tx.commit()?;
        println!("✅ Saved {} conversation sessions to SQLite", sessions_count);
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
            "SELECT id, type, source, content, timestamp, confidence 
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
        // Check if message already exists (deduplication)
        let exists: bool = self.connection.query_row(
            "SELECT 1 FROM conversation_messages WHERE id = ?",
            params![message.id],
            |_| Ok(true)
        ).unwrap_or(false);

        if exists {
            return Ok(()); // Message already saved
        }

        self.connection.execute(
            "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp, confidence) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                message.id, session_id, message.message_type, message.source,
                message.content, message.timestamp, message.confidence
            ]
        )?;

        Ok(())
    }

    pub fn batch_save_conversation_messages(&mut self, session_id: &str, messages: Vec<ConversationMessage>) -> Result<()> {
        let tx = self.connection.transaction()?;

        for message in messages {
            // Check if message already exists (deduplication)
            let exists: bool = tx.query_row(
                "SELECT 1 FROM conversation_messages WHERE id = ?",
                params![message.id],
                |_| Ok(true)
            ).unwrap_or(false);

            if !exists {
                tx.execute(
                    "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp, confidence) 
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                    params![
                        message.id, session_id, message.message_type, message.source,
                        message.content, message.timestamp, message.confidence
                    ]
                )?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn update_conversation_message(&mut self, session_id: &str, message_id: &str, updates: ConversationMessageUpdate) -> Result<()> {
        let mut set_clauses = Vec::new();
        let mut params = Vec::new();

        if let Some(content) = updates.content {
            set_clauses.push("content = ?");
            params.push(content);
        }
        if let Some(confidence) = updates.confidence {
            set_clauses.push("confidence = ?");
            params.push(confidence.to_string());
        }
        if let Some(timestamp) = updates.timestamp {
            set_clauses.push("timestamp = ?");
            params.push(timestamp.to_string());
        }

        if set_clauses.is_empty() {
            return Ok(()); // No updates to apply
        }

        // Add message_id and session_id for WHERE clause
        params.push(message_id.to_string());
        params.push(session_id.to_string());

        let sql = format!(
            "UPDATE conversation_messages SET {} WHERE id = ? AND session_id = ?",
            set_clauses.join(", ")
        );

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        self.connection.execute(&sql, param_refs.as_slice())?;

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