// Context window management for LLM interactions
// Tracks token budgets and builds context from recent conversation history

use crate::data::types::{ConversationMessage, ContextWindow};
use rusqlite::{Connection, params, Result as SqliteResult};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Default maximum tokens for context window
const DEFAULT_MAX_TOKENS: i32 = 4000;

/// Default maximum number of messages to include
const DEFAULT_MAX_MESSAGES: usize = 50;

/// Rough estimate: average tokens per character (OpenAI uses ~4 chars per token)
const CHARS_PER_TOKEN: usize = 4;

/// Manages context windows for LLM interactions
pub struct ContextWindowManager {
    session_id: String,
    max_tokens: i32,
    max_messages: usize,
}

impl ContextWindowManager {
    /// Create a new context window manager for a session
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            max_tokens: DEFAULT_MAX_TOKENS,
            max_messages: DEFAULT_MAX_MESSAGES,
        }
    }

    /// Create with custom token and message limits
    pub fn with_limits(session_id: String, max_tokens: i32, max_messages: usize) -> Self {
        Self {
            session_id,
            max_tokens,
            max_messages,
        }
    }

    /// Fetch recent messages within token budget
    /// Returns messages in chronological order (oldest first)
    pub fn get_context(&self, conn: &Connection) -> SqliteResult<Vec<ConversationMessage>> {
        // Fetch recent messages (most recent first)
        let mut stmt = conn.prepare(
            "SELECT id, type, source, content, timestamp, confidence,
                    audio_level, processing_latency_ms, model_version,
                    is_partial, merged_from, speaker_id
             FROM conversation_messages
             WHERE session_id = ?
             ORDER BY timestamp DESC
             LIMIT ?"
        )?;

        let messages_iter = stmt.query_map(params![&self.session_id, self.max_messages as i32], |row| {
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
                is_partial: row.get("is_partial")?,
                merged_from: row.get("merged_from")?,
                speaker_id: row.get("speaker_id")?,
                // Initialize optional frontend fields as None
                is_preview: None,
                is_typing: None,
                persistence_state: None,
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            })
        })?;

        let mut messages: Vec<ConversationMessage> = messages_iter.collect::<Result<Vec<_>, _>>()?;

        // Apply token budget by removing oldest messages if over budget
        let mut total_tokens = 0;
        let mut kept_messages = Vec::new();

        for msg in messages.iter() {
            let msg_tokens = self.estimate_tokens(&msg.content);
            if total_tokens + msg_tokens <= self.max_tokens {
                total_tokens += msg_tokens;
                kept_messages.push(msg.clone());
            } else {
                // Token budget exceeded, stop adding older messages
                break;
            }
        }

        // Reverse to get chronological order (oldest first)
        kept_messages.reverse();

        Ok(kept_messages)
    }

    /// Save a context window snapshot to the database
    pub fn save_context(&self, conn: &Connection, messages: &[ConversationMessage]) -> SqliteResult<String> {
        let context_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let messages_json = serde_json::to_string(messages)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        let token_count = messages.iter()
            .map(|m| self.estimate_tokens(&m.content))
            .sum::<i32>();

        conn.execute(
            "INSERT INTO context_windows (id, session_id, messages_json, token_count, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![context_id, &self.session_id, messages_json, token_count, now]
        )?;

        Ok(context_id)
    }

    /// Build a prompt-ready context string from messages
    /// Format: "User: <message>\nSystem: <message>\n..."
    pub fn build_llm_context(&self, messages: &[ConversationMessage]) -> String {
        messages.iter()
            .map(|msg| {
                let role = match msg.message_type.as_str() {
                    "user" => "User",
                    "system" => "System",
                    _ => "Unknown",
                };
                format!("{}: {}", role, msg.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Estimate token count for text (rough approximation)
    /// Uses ~4 characters per token rule of thumb
    fn estimate_tokens(&self, text: &str) -> i32 {
        (text.len() / CHARS_PER_TOKEN) as i32
    }

    /// Get the most recent context window from database
    pub fn load_latest_context(&self, conn: &Connection) -> SqliteResult<Option<ContextWindow>> {
        let result = conn.query_row(
            "SELECT id, session_id, messages_json, token_count, created_at
             FROM context_windows
             WHERE session_id = ?
             ORDER BY created_at DESC
             LIMIT 1",
            params![&self.session_id],
            |row| {
                Ok(ContextWindow {
                    id: row.get("id")?,
                    session_id: row.get("session_id")?,
                    messages_json: row.get("messages_json")?,
                    token_count: row.get("token_count")?,
                    created_at: row.get("created_at")?,
                })
            },
        );

        match result {
            Ok(context) => Ok(Some(context)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Summarize old context when exceeding limits
    /// TODO: Implement actual LLM-based summarization in future iteration
    /// For now, this is a placeholder that returns the original messages
    pub fn summarize_old_context(&self, _messages: &[ConversationMessage]) -> String {
        // Placeholder for future implementation
        // In a real implementation, this would:
        // 1. Send old messages to an LLM
        // 2. Request a summary of key points
        // 3. Return the summary as a single message
        // 4. Replace old messages with the summary in context

        "[Context summary placeholder - to be implemented]".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // Create minimal schema
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
                audio_level REAL,
                processing_latency_ms INTEGER,
                model_version TEXT,
                is_partial INTEGER,
                merged_from TEXT,
                speaker_id TEXT,
                FOREIGN KEY (session_id) REFERENCES conversation_sessions(id)
            );

            CREATE TABLE context_windows (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                messages_json TEXT NOT NULL,
                token_count INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES conversation_sessions(id)
            );
        "#).unwrap();

        conn
    }

    #[test]
    fn test_new_context_manager() {
        let manager = ContextWindowManager::new("test-session".to_string());
        assert_eq!(manager.session_id, "test-session");
        assert_eq!(manager.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(manager.max_messages, DEFAULT_MAX_MESSAGES);
    }

    #[test]
    fn test_custom_limits() {
        let manager = ContextWindowManager::with_limits("test".to_string(), 2000, 25);
        assert_eq!(manager.max_tokens, 2000);
        assert_eq!(manager.max_messages, 25);
    }

    #[test]
    fn test_estimate_tokens() {
        let manager = ContextWindowManager::new("test".to_string());
        let text = "a".repeat(400); // 400 chars ~ 100 tokens
        assert_eq!(manager.estimate_tokens(&text), 100);
    }

    #[test]
    fn test_build_llm_context() {
        let manager = ContextWindowManager::new("test".to_string());
        let messages = vec![
            ConversationMessage {
                id: "1".to_string(),
                message_type: "user".to_string(),
                source: "microphone".to_string(),
                content: "Hello".to_string(),
                timestamp: 1000,
                confidence: Some(0.9),
                audio_level: None,
                processing_latency_ms: None,
                model_version: None,
                is_partial: None,
                merged_from: None,
                speaker_id: None,
                is_preview: None,
                is_typing: None,
                persistence_state: None,
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            },
            ConversationMessage {
                id: "2".to_string(),
                message_type: "system".to_string(),
                source: "loopback".to_string(),
                content: "Hi there".to_string(),
                timestamp: 2000,
                confidence: None,
                audio_level: None,
                processing_latency_ms: None,
                model_version: None,
                is_partial: None,
                merged_from: None,
                speaker_id: None,
                is_preview: None,
                is_typing: None,
                persistence_state: None,
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            },
        ];

        let context = manager.build_llm_context(&messages);
        assert_eq!(context, "User: Hello\nSystem: Hi there");
    }

    #[test]
    fn test_save_and_load_context() {
        let conn = setup_test_db();
        let session_id = "test-session";

        // Create session first
        conn.execute(
            "INSERT INTO conversation_sessions (id, name, start_time, is_active) VALUES (?, ?, ?, ?)",
            params![session_id, "Test", 1000, 1]
        ).unwrap();

        let manager = ContextWindowManager::new(session_id.to_string());
        let messages = vec![
            ConversationMessage {
                id: "1".to_string(),
                message_type: "user".to_string(),
                source: "microphone".to_string(),
                content: "Test message".to_string(),
                timestamp: 1000,
                confidence: Some(0.9),
                audio_level: None,
                processing_latency_ms: None,
                model_version: None,
                is_partial: None,
                merged_from: None,
                speaker_id: None,
                is_preview: None,
                is_typing: None,
                persistence_state: None,
                retry_count: None,
                last_save_attempt: None,
                save_error: None,
            },
        ];

        // Save context
        let context_id = manager.save_context(&conn, &messages).unwrap();
        assert!(!context_id.is_empty());

        // Load context
        let loaded = manager.load_latest_context(&conn).unwrap();
        assert!(loaded.is_some());

        let context = loaded.unwrap();
        assert_eq!(context.session_id, session_id);
        assert!(context.token_count > 0);
    }

    #[test]
    fn test_get_context_empty() {
        let conn = setup_test_db();
        let session_id = "test-session";

        // Create session
        conn.execute(
            "INSERT INTO conversation_sessions (id, name, start_time, is_active) VALUES (?, ?, ?, ?)",
            params![session_id, "Test", 1000, 1]
        ).unwrap();

        let manager = ContextWindowManager::new(session_id.to_string());
        let messages = manager.get_context(&conn).unwrap();
        assert_eq!(messages.len(), 0);
    }
}
