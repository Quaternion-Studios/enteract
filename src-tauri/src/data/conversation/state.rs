// Conversation state machine implementation
// Tracks conversation lifecycle and handles state transitions

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rusqlite::{Connection, params, Result as SqliteResult};

/// Represents the current state of a conversation session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConversationState {
    /// No recent activity
    Idle,
    /// Receiving audio input from user
    Listening,
    /// Transcribing audio or processing input
    Processing,
    /// AI generating response
    Responding,
}

impl ConversationState {
    /// Convert state to string for database storage
    pub fn to_str(&self) -> &'static str {
        match self {
            ConversationState::Idle => "idle",
            ConversationState::Listening => "listening",
            ConversationState::Processing => "processing",
            ConversationState::Responding => "responding",
        }
    }

    /// Parse state from database string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(ConversationState::Idle),
            "listening" => Some(ConversationState::Listening),
            "processing" => Some(ConversationState::Processing),
            "responding" => Some(ConversationState::Responding),
            _ => None,
        }
    }
}

/// Manages conversation state and transitions
pub struct ConversationStateMachine {
    session_id: String,
    current_state: ConversationState,
    last_transition: i64,
    last_user_speech_at: Option<i64>,
    last_system_speech_at: Option<i64>,
    context_summary: Option<String>,
    topic: Option<String>,
    silence_timeout: Duration,
}

impl ConversationStateMachine {
    /// Create a new state machine for a session
    pub fn new(session_id: String, silence_timeout: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            session_id,
            current_state: ConversationState::Idle,
            last_transition: now,
            last_user_speech_at: None,
            last_system_speech_at: None,
            context_summary: None,
            topic: None,
            silence_timeout,
        }
    }

    /// Load state machine from database
    pub fn load(conn: &Connection, session_id: &str) -> SqliteResult<Option<Self>> {
        let result = conn.query_row(
            "SELECT state, last_user_speech_at, last_system_speech_at,
             context_summary, topic, updated_at
             FROM conversation_state WHERE session_id = ?",
            params![session_id],
            |row| {
                let state_str: String = row.get("state")?;
                let state = ConversationState::from_str(&state_str)
                    .unwrap_or(ConversationState::Idle);

                Ok(Self {
                    session_id: session_id.to_string(),
                    current_state: state,
                    last_transition: row.get("updated_at")?,
                    last_user_speech_at: row.get("last_user_speech_at")?,
                    last_system_speech_at: row.get("last_system_speech_at")?,
                    context_summary: row.get("context_summary")?,
                    topic: row.get("topic")?,
                    silence_timeout: Duration::from_secs(30), // Default 30s
                })
            },
        );

        match result {
            Ok(state_machine) => Ok(Some(state_machine)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Save current state to database
    pub fn save(&self, conn: &Connection) -> SqliteResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO conversation_state
             (session_id, state, last_user_speech_at, last_system_speech_at,
              context_summary, topic, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                &self.session_id,
                self.current_state.to_str(),
                self.last_user_speech_at,
                self.last_system_speech_at,
                &self.context_summary,
                &self.topic,
                self.last_transition,
            ],
        )?;
        Ok(())
    }

    /// Get current state
    pub fn state(&self) -> &ConversationState {
        &self.current_state
    }

    /// Transition to a new state
    fn transition_to(&mut self, new_state: ConversationState) {
        if self.current_state != new_state {
            self.current_state = new_state;
            self.last_transition = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
        }
    }

    /// Handle audio received event
    pub fn on_audio_received(&mut self) {
        match self.current_state {
            ConversationState::Idle | ConversationState::Processing | ConversationState::Responding => {
                self.transition_to(ConversationState::Listening);
            }
            ConversationState::Listening => {
                // Already listening, update timestamp
                self.last_transition = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
            }
        }
    }

    /// Handle transcription complete event
    pub fn on_transcription_complete(&mut self) {
        self.last_user_speech_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        );
        self.transition_to(ConversationState::Processing);
    }

    /// Handle AI response start event
    pub fn on_ai_response_start(&mut self) {
        self.transition_to(ConversationState::Responding);
    }

    /// Handle AI response complete event
    pub fn on_ai_response_complete(&mut self) {
        self.last_system_speech_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        );
        self.transition_to(ConversationState::Idle);
    }

    /// Check for timeout and transition to Idle if needed
    /// Returns Some(new_state) if state changed
    pub fn tick(&mut self) -> Option<ConversationState> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let elapsed_ms = now - self.last_transition;
        let timeout_ms = self.silence_timeout.as_millis() as i64;

        // Don't timeout if we're actively responding
        if self.current_state == ConversationState::Responding {
            return None;
        }

        if elapsed_ms > timeout_ms && self.current_state != ConversationState::Idle {
            self.transition_to(ConversationState::Idle);
            return Some(ConversationState::Idle);
        }

        None
    }

    /// Update context summary
    pub fn set_context_summary(&mut self, summary: String) {
        self.context_summary = Some(summary);
    }

    /// Update conversation topic
    pub fn set_topic(&mut self, topic: String) {
        self.topic = Some(topic);
    }

    /// Get context summary
    pub fn context_summary(&self) -> Option<&str> {
        self.context_summary.as_deref()
    }

    /// Get topic
    pub fn topic(&self) -> Option<&str> {
        self.topic.as_deref()
    }

    /// Get last user speech timestamp
    pub fn last_user_speech_at(&self) -> Option<i64> {
        self.last_user_speech_at
    }

    /// Get last system speech timestamp
    pub fn last_system_speech_at(&self) -> Option<i64> {
        self.last_system_speech_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_state_transitions() {
        let mut sm = ConversationStateMachine::new("test-session".to_string(), Duration::from_secs(30));

        assert_eq!(*sm.state(), ConversationState::Idle);

        sm.on_audio_received();
        assert_eq!(*sm.state(), ConversationState::Listening);

        sm.on_transcription_complete();
        assert_eq!(*sm.state(), ConversationState::Processing);
        assert!(sm.last_user_speech_at().is_some());

        sm.on_ai_response_start();
        assert_eq!(*sm.state(), ConversationState::Responding);

        sm.on_ai_response_complete();
        assert_eq!(*sm.state(), ConversationState::Idle);
        assert!(sm.last_system_speech_at().is_some());
    }

    #[test]
    fn test_timeout_to_idle() {
        let mut sm = ConversationStateMachine::new("test-session".to_string(), Duration::from_millis(100));

        sm.on_audio_received();
        assert_eq!(*sm.state(), ConversationState::Listening);

        // Should not timeout immediately
        assert_eq!(sm.tick(), None);
        assert_eq!(*sm.state(), ConversationState::Listening);

        // Wait for timeout
        thread::sleep(Duration::from_millis(150));

        // Should timeout to Idle
        assert_eq!(sm.tick(), Some(ConversationState::Idle));
        assert_eq!(*sm.state(), ConversationState::Idle);
    }

    #[test]
    fn test_context_and_topic() {
        let mut sm = ConversationStateMachine::new("test-session".to_string(), Duration::from_secs(30));

        sm.set_context_summary("Discussing weather".to_string());
        sm.set_topic("Weather".to_string());

        assert_eq!(sm.context_summary(), Some("Discussing weather"));
        assert_eq!(sm.topic(), Some("Weather"));
    }
}
