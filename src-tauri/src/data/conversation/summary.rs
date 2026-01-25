// Session summarization and topic extraction for conversations
// Provides incremental summarization and keyword-based topic detection

use rusqlite::{Connection, params, Result as SqliteResult};
use std::collections::HashMap;

/// Maximum summary length in characters
const MAX_SUMMARY_LENGTH: usize = 500;

/// Minimum message count before generating summary
const MIN_MESSAGES_FOR_SUMMARY: usize = 5;

/// Common stop words to filter out from topic extraction
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "as", "is", "was", "are", "were", "be",
    "been", "being", "have", "has", "had", "do", "does", "did", "will",
    "would", "should", "could", "may", "might", "must", "can", "this",
    "that", "these", "those", "i", "you", "he", "she", "it", "we", "they",
];

/// Manages session summaries and topic extraction
pub struct SessionSummarizer {
    session_id: String,
}

impl SessionSummarizer {
    /// Create a new summarizer for a session
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }

    /// Update the session summary based on recent messages
    /// Generates an incremental summary and updates conversation_state
    pub fn update_summary(&self, conn: &Connection) -> SqliteResult<String> {
        // Fetch recent messages from this session
        let mut stmt = conn.prepare(
            "SELECT content, type FROM conversation_messages
             WHERE session_id = ?
             ORDER BY timestamp DESC
             LIMIT 20"
        )?;

        let messages: Vec<(String, String)> = stmt.query_map(params![&self.session_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;

        if messages.len() < MIN_MESSAGES_FOR_SUMMARY {
            return Ok("Not enough messages for summary".to_string());
        }

        // Build a simple extractive summary
        // In a real implementation, this would use LLM-based summarization
        let summary = self.generate_extractive_summary(&messages);

        // Update conversation_state with new summary
        conn.execute(
            "INSERT OR REPLACE INTO conversation_state
             (session_id, state, context_summary, updated_at)
             VALUES (?,
                     COALESCE((SELECT state FROM conversation_state WHERE session_id = ?), 'idle'),
                     ?,
                     ?)",
            params![
                &self.session_id,
                &self.session_id,
                &summary,
                chrono::Utc::now().timestamp_millis()
            ]
        )?;

        Ok(summary)
    }

    /// Extract main topics from conversation messages
    /// Returns a comma-separated list of topics
    pub fn extract_topics(&self, conn: &Connection) -> SqliteResult<String> {
        // Fetch all message content
        let mut stmt = conn.prepare(
            "SELECT content FROM conversation_messages
             WHERE session_id = ?
             ORDER BY timestamp"
        )?;

        let messages: Vec<String> = stmt.query_map(params![&self.session_id], |row| {
            row.get(0)
        })?.collect::<Result<Vec<_>, _>>()?;

        if messages.is_empty() {
            return Ok(String::new());
        }

        // Extract topics using keyword frequency analysis
        let topics = self.extract_keywords(&messages);

        // Update conversation_state with topics
        conn.execute(
            "INSERT OR REPLACE INTO conversation_state
             (session_id, state, topic, updated_at)
             VALUES (?,
                     COALESCE((SELECT state FROM conversation_state WHERE session_id = ?), 'idle'),
                     ?,
                     ?)",
            params![
                &self.session_id,
                &self.session_id,
                &topics,
                chrono::Utc::now().timestamp_millis()
            ]
        )?;

        Ok(topics)
    }

    /// Generate an extractive summary from messages
    /// Simple implementation: takes first few sentences from user and system messages
    fn generate_extractive_summary(&self, messages: &[(String, String)]) -> String {
        let mut summary_parts = Vec::new();
        let mut current_length = 0;

        // Iterate through messages (already in reverse chronological order)
        for (content, msg_type) in messages.iter().rev() {
            // Only include meaningful messages (skip very short ones)
            if content.len() < 10 {
                continue;
            }

            // Take first sentence or first 100 chars
            let snippet = if let Some(period_idx) = content.find('.') {
                &content[..period_idx + 1]
            } else {
                &content[..content.len().min(100)]
            };

            let prefix = match msg_type.as_str() {
                "user" => "User: ",
                "system" => "System: ",
                _ => "",
            };

            let part = format!("{}{}", prefix, snippet);
            let part_len = part.len();

            if current_length + part_len > MAX_SUMMARY_LENGTH {
                break;
            }

            summary_parts.push(part);
            current_length += part_len;
        }

        if summary_parts.is_empty() {
            "No summary available".to_string()
        } else {
            summary_parts.join(" ")
        }
    }

    /// Extract keywords/topics using simple frequency analysis
    /// Returns top 3-5 topics as comma-separated string
    fn extract_keywords(&self, messages: &[String]) -> String {
        let mut word_counts: HashMap<String, usize> = HashMap::new();

        // Combine all message content
        let full_text = messages.join(" ").to_lowercase();

        // Tokenize and count words
        for word in full_text.split_whitespace() {
            // Clean word (remove punctuation)
            let clean_word: String = word.chars()
                .filter(|c| c.is_alphabetic())
                .collect();

            // Skip short words and stop words
            if clean_word.len() < 4 {
                continue;
            }

            if STOP_WORDS.contains(&clean_word.as_str()) {
                continue;
            }

            *word_counts.entry(clean_word).or_insert(0) += 1;
        }

        // Sort by frequency
        let mut word_freq: Vec<(String, usize)> = word_counts.into_iter().collect();
        word_freq.sort_by(|a, b| b.1.cmp(&a.1));

        // Take top 5 topics
        let topics: Vec<String> = word_freq.iter()
            .take(5)
            .map(|(word, _)| word.clone())
            .collect();

        if topics.is_empty() {
            "general conversation".to_string()
        } else {
            topics.join(", ")
        }
    }

    /// Get current summary from database
    pub fn get_summary(&self, conn: &Connection) -> SqliteResult<Option<String>> {
        let result = conn.query_row(
            "SELECT context_summary FROM conversation_state WHERE session_id = ?",
            params![&self.session_id],
            |row| row.get(0)
        );

        match result {
            Ok(summary) => Ok(summary),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get current topics from database
    pub fn get_topics(&self, conn: &Connection) -> SqliteResult<Option<String>> {
        let result = conn.query_row(
            "SELECT topic FROM conversation_state WHERE session_id = ?",
            params![&self.session_id],
            |row| row.get(0)
        );

        match result {
            Ok(topic) => Ok(topic),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

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

            CREATE TABLE conversation_state (
                session_id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                last_user_speech_at INTEGER,
                last_system_speech_at INTEGER,
                context_summary TEXT,
                topic TEXT,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES conversation_sessions(id)
            );
        "#).unwrap();

        conn
    }

    #[test]
    fn test_extract_keywords() {
        let summarizer = SessionSummarizer::new("test-session".to_string());
        let messages = vec![
            "Hello, I want to discuss machine learning algorithms".to_string(),
            "Machine learning is fascinating, especially neural networks".to_string(),
            "Yes, neural networks and deep learning are very powerful".to_string(),
        ];

        let topics = summarizer.extract_keywords(&messages);
        assert!(topics.contains("machine"));
        assert!(topics.contains("learning"));
        assert!(topics.contains("neural"));
    }

    #[test]
    fn test_extractive_summary() {
        let summarizer = SessionSummarizer::new("test-session".to_string());
        let messages = vec![
            ("Hello, how are you today?".to_string(), "user".to_string()),
            ("I'm doing well, thank you!".to_string(), "system".to_string()),
            ("Great! Let's talk about the weather.".to_string(), "user".to_string()),
        ];

        let summary = summarizer.generate_extractive_summary(&messages);
        assert!(summary.contains("User:"));
        assert!(summary.contains("System:"));
        assert!(summary.len() <= MAX_SUMMARY_LENGTH);
    }

    #[test]
    fn test_update_summary_integration() {
        let conn = setup_test_db();
        let session_id = "test-session";

        // Create session
        conn.execute(
            "INSERT INTO conversation_sessions (id, name, start_time, is_active) VALUES (?, ?, ?, ?)",
            params![session_id, "Test", 1000, 1]
        ).unwrap();

        // Add messages
        for i in 0..10 {
            conn.execute(
                "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp)
                 VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    format!("msg-{}", i),
                    session_id,
                    if i % 2 == 0 { "user" } else { "system" },
                    "microphone",
                    format!("This is test message number {}.", i),
                    1000 + i
                ]
            ).unwrap();
        }

        let summarizer = SessionSummarizer::new(session_id.to_string());
        let summary = summarizer.update_summary(&conn).unwrap();

        assert!(!summary.is_empty());
        assert!(summary.len() <= MAX_SUMMARY_LENGTH);

        // Verify it was stored
        let stored_summary = summarizer.get_summary(&conn).unwrap();
        assert!(stored_summary.is_some());
        assert_eq!(stored_summary.unwrap(), summary);
    }

    #[test]
    fn test_extract_topics_integration() {
        let conn = setup_test_db();
        let session_id = "test-session";

        // Create session
        conn.execute(
            "INSERT INTO conversation_sessions (id, name, start_time, is_active) VALUES (?, ?, ?, ?)",
            params![session_id, "Test", 1000, 1]
        ).unwrap();

        // Add messages about specific topics
        conn.execute(
            "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
            params!["msg-1", session_id, "user", "microphone", "Let's discuss programming and software development", 1000]
        ).unwrap();

        conn.execute(
            "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
            params!["msg-2", session_id, "system", "loopback", "Programming is a great skill for software engineering", 2000]
        ).unwrap();

        let summarizer = SessionSummarizer::new(session_id.to_string());
        let topics = summarizer.extract_topics(&conn).unwrap();

        assert!(!topics.is_empty());
        assert!(topics.contains("programming") || topics.contains("software"));

        // Verify it was stored
        let stored_topics = summarizer.get_topics(&conn).unwrap();
        assert!(stored_topics.is_some());
        assert_eq!(stored_topics.unwrap(), topics);
    }

    #[test]
    fn test_summary_with_few_messages() {
        let conn = setup_test_db();
        let session_id = "test-session";

        // Create session with only 2 messages (below MIN_MESSAGES_FOR_SUMMARY)
        conn.execute(
            "INSERT INTO conversation_sessions (id, name, start_time, is_active) VALUES (?, ?, ?, ?)",
            params![session_id, "Test", 1000, 1]
        ).unwrap();

        conn.execute(
            "INSERT INTO conversation_messages (id, session_id, type, source, content, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
            params!["msg-1", session_id, "user", "microphone", "Hello", 1000]
        ).unwrap();

        let summarizer = SessionSummarizer::new(session_id.to_string());
        let summary = summarizer.update_summary(&conn).unwrap();

        assert_eq!(summary, "Not enough messages for summary");
    }
}
