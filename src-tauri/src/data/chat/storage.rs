// SQLite storage implementation for chat sessions
use rusqlite::{Connection, Result, params};
use tauri::{AppHandle, Manager};
use crate::data::types::{
    ChatSession, ChatMessage, MessageAttachment, ThinkingProcess, ThinkingStep, MessageMetadata,
    SaveChatsPayload, LoadChatsResponse
};
use std::path::PathBuf;

pub struct ChatStorage {
    connection: Connection,
}

impl ChatStorage {
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
        storage.initialize_chat_tables()?;
        
        Ok(storage)
    }

    fn initialize_chat_tables(&mut self) -> Result<()> {
        // Create chat-specific tables
        self.connection.execute_batch(r#"
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

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated_desc ON chat_sessions(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_chat_messages_session_timestamp ON chat_messages(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_message_attachments_message ON message_attachments(message_id);
            CREATE INDEX IF NOT EXISTS idx_thinking_processes_message ON thinking_processes(message_id);
            CREATE INDEX IF NOT EXISTS idx_thinking_steps_thinking ON thinking_steps(thinking_id);
            CREATE INDEX IF NOT EXISTS idx_message_metadata_message ON message_metadata(message_id);
        "#)?;

        Ok(())
    }

    pub fn save_chat_sessions(&mut self, payload: SaveChatsPayload) -> Result<()> {
        let tx = self.connection.transaction()?;

        // Clear existing data (full replacement for now - can be optimized later)
        tx.execute("DELETE FROM chat_sessions", params![])?;

        let sessions_count = payload.chats.len();
        for session in payload.chats {
            // Insert session
            tx.execute(
                "INSERT INTO chat_sessions (id, title, created_at, updated_at, model_id) VALUES (?, ?, ?, ?, ?)",
                params![session.id, session.title, session.created_at, session.updated_at, session.model_id]
            )?;

            // Insert messages and related data
            for message in session.history {
                // Insert main message
                tx.execute(
                    "INSERT INTO chat_messages (id, session_id, text, sender, timestamp, is_interim, confidence, source, message_type) 
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        message.id, session.id, message.text, message.sender, message.timestamp,
                        message.is_interim.map(|b| if b { 1 } else { 0 }),
                        message.confidence, message.source, message.message_type
                    ]
                )?;

                // Insert attachments if present
                if let Some(attachments) = message.attachments {
                    for attachment in attachments {
                        tx.execute(
                            "INSERT INTO message_attachments (id, message_id, type, name, size, mime_type, url, base64_data, thumbnail, extracted_text, width, height, upload_progress, upload_status, error)
                             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                            params![
                                attachment.id, message.id, attachment.attachment_type, attachment.name, attachment.size,
                                attachment.mime_type, attachment.url, attachment.base64_data, attachment.thumbnail,
                                attachment.extracted_text, 
                                attachment.dimensions.as_ref().map(|d| d.width),
                                attachment.dimensions.as_ref().map(|d| d.height),
                                attachment.upload_progress, attachment.upload_status, attachment.error
                            ]
                        )?;
                    }
                }

                // Insert thinking process if present
                if let Some(thinking) = message.thinking {
                    tx.execute(
                        "INSERT INTO thinking_processes (message_id, is_visible, content, is_streaming) VALUES (?, ?, ?, ?)",
                        params![
                            message.id, 
                            if thinking.is_visible { 1 } else { 0 },
                            thinking.content,
                            if thinking.is_streaming { 1 } else { 0 }
                        ]
                    )?;

                    let thinking_id: i64 = tx.last_insert_rowid();

                    // Insert thinking steps if present
                    if let Some(steps) = thinking.steps {
                        for step in steps {
                            tx.execute(
                                "INSERT INTO thinking_steps (id, thinking_id, title, content, timestamp, status) VALUES (?, ?, ?, ?, ?, ?)",
                                params![step.id, thinking_id, step.title, step.content, step.timestamp, step.status]
                            )?;
                        }
                    }
                }

                // Insert message metadata if present
                if let Some(metadata) = message.metadata {
                    tx.execute(
                        "INSERT INTO message_metadata (message_id, agent_type, model, tokens, processing_time, analysis_types, search_queries, sources)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                        params![
                            message.id, metadata.agent_type, metadata.model, metadata.tokens, metadata.processing_time,
                            metadata.analysis_type.map(|v| serde_json::to_string(&v).unwrap_or_default()),
                            metadata.search_queries.map(|v| serde_json::to_string(&v).unwrap_or_default()),
                            metadata.sources.map(|v| serde_json::to_string(&v).unwrap_or_default())
                        ]
                    )?;
                }
            }
        }

        tx.commit()?;
        println!("✅ Saved {} chat sessions to SQLite", sessions_count);
        Ok(())
    }

    pub fn load_chat_sessions(&self) -> Result<LoadChatsResponse> {
        let mut sessions = Vec::new();

        // Query all sessions
        let mut session_stmt = self.connection.prepare(
            "SELECT id, title, created_at, updated_at, model_id FROM chat_sessions ORDER BY updated_at DESC"
        )?;

        let session_iter = session_stmt.query_map(params![], |row| {
            Ok((
                row.get::<_, String>("id")?,
                row.get::<_, String>("title")?,
                row.get::<_, String>("created_at")?,
                row.get::<_, String>("updated_at")?,
                row.get::<_, Option<String>>("model_id")?,
            ))
        })?;

        for session_result in session_iter {
            let (id, title, created_at, updated_at, model_id) = session_result?;
            
            // Load messages for this session
            let history = self.load_messages_for_session(&id)?;

            sessions.push(ChatSession {
                id,
                title,
                created_at,
                updated_at,
                model_id,
                history,
            });
        }

        println!("✅ Loaded {} chat sessions from SQLite", sessions.len());
        Ok(LoadChatsResponse { chats: sessions })
    }

    fn load_messages_for_session(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let mut messages = Vec::new();

        let mut stmt = self.connection.prepare(
            "SELECT id, text, sender, timestamp, is_interim, confidence, source, message_type 
             FROM chat_messages WHERE session_id = ? ORDER BY timestamp"
        )?;

        let message_iter = stmt.query_map([session_id], |row| {
            let message_id: i32 = row.get("id")?;
            Ok(ChatMessage {
                id: message_id,
                text: row.get("text")?,
                sender: row.get("sender")?,
                timestamp: row.get("timestamp")?,
                is_interim: row.get::<_, Option<i32>>("is_interim")?.map(|i| i != 0),
                confidence: row.get("confidence")?,
                source: row.get("source")?,
                message_type: row.get("message_type")?,
                // Load related data separately
                attachments: self.load_attachments_for_message(message_id).ok(),
                thinking: self.load_thinking_for_message(message_id).ok(),
                metadata: self.load_metadata_for_message(message_id).ok(),
            })
        })?;

        for message_result in message_iter {
            messages.push(message_result?);
        }

        Ok(messages)
    }

    fn load_attachments_for_message(&self, message_id: i32) -> Result<Vec<MessageAttachment>> {
        let mut attachments = Vec::new();
        
        let mut stmt = self.connection.prepare(
            "SELECT id, type, name, size, mime_type, url, base64_data, thumbnail, extracted_text, width, height, upload_progress, upload_status, error 
             FROM message_attachments WHERE message_id = ?"
        )?;

        let attachment_iter = stmt.query_map([message_id], |row| {
            let width: Option<i32> = row.get("width")?;
            let height: Option<i32> = row.get("height")?;
            let dimensions = if let (Some(w), Some(h)) = (width, height) {
                Some(crate::data::types::FileDimensions { width: w, height: h })
            } else {
                None
            };

            Ok(MessageAttachment {
                id: row.get("id")?,
                attachment_type: row.get("type")?,
                name: row.get("name")?,
                size: row.get("size")?,
                mime_type: row.get("mime_type")?,
                url: row.get("url")?,
                base64_data: row.get("base64_data")?,
                thumbnail: row.get("thumbnail")?,
                extracted_text: row.get("extracted_text")?,
                dimensions,
                upload_progress: row.get("upload_progress")?,
                upload_status: row.get("upload_status")?,
                error: row.get("error")?,
            })
        })?;

        for attachment_result in attachment_iter {
            attachments.push(attachment_result?);
        }

        Ok(attachments)
    }

    fn load_thinking_for_message(&self, message_id: i32) -> Result<ThinkingProcess> {
        let mut stmt = self.connection.prepare(
            "SELECT id, is_visible, content, is_streaming FROM thinking_processes WHERE message_id = ?"
        )?;

        let thinking_row = stmt.query_row([message_id], |row| {
            let thinking_id: i64 = row.get("id")?;
            Ok((
                thinking_id,
                row.get::<_, i32>("is_visible")? != 0,
                row.get::<_, String>("content")?,
                row.get::<_, i32>("is_streaming")? != 0,
            ))
        })?;

        let (thinking_id, is_visible, content, is_streaming) = thinking_row;

        // Load thinking steps
        let steps = self.load_thinking_steps(thinking_id)?;

        Ok(ThinkingProcess {
            is_visible,
            content,
            is_streaming,
            steps: if steps.is_empty() { None } else { Some(steps) },
        })
    }

    fn load_thinking_steps(&self, thinking_id: i64) -> Result<Vec<ThinkingStep>> {
        let mut steps = Vec::new();
        
        let mut stmt = self.connection.prepare(
            "SELECT id, title, content, timestamp, status FROM thinking_steps WHERE thinking_id = ? ORDER BY timestamp"
        )?;

        let step_iter = stmt.query_map([thinking_id], |row| {
            Ok(ThinkingStep {
                id: row.get("id")?,
                title: row.get("title")?,
                content: row.get("content")?,
                timestamp: row.get("timestamp")?,
                status: row.get("status")?,
            })
        })?;

        for step_result in step_iter {
            steps.push(step_result?);
        }

        Ok(steps)
    }

    fn load_metadata_for_message(&self, message_id: i32) -> Result<MessageMetadata> {
        let mut stmt = self.connection.prepare(
            "SELECT agent_type, model, tokens, processing_time, analysis_types, search_queries, sources 
             FROM message_metadata WHERE message_id = ?"
        )?;

        stmt.query_row([message_id], |row| {
            let analysis_types: Option<String> = row.get("analysis_types")?;
            let search_queries: Option<String> = row.get("search_queries")?;
            let sources: Option<String> = row.get("sources")?;

            Ok(MessageMetadata {
                agent_type: row.get("agent_type")?,
                model: row.get("model")?,
                tokens: row.get("tokens")?,
                processing_time: row.get("processing_time")?,
                analysis_type: analysis_types.and_then(|s| serde_json::from_str(&s).ok()),
                search_queries: search_queries.and_then(|s| serde_json::from_str(&s).ok()),
                sources: sources.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })
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