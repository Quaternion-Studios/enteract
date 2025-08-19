use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl RagError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

impl From<RagError> for String {
    fn from(error: RagError) -> Self {
        format!("{}: {}", error.code, error.message)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagMetadata {
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub custom_fields: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl ToString for ProcessingStatus {
    fn to_string(&self) -> String {
        match self {
            ProcessingStatus::Pending => "pending".to_string(),
            ProcessingStatus::Processing => "processing".to_string(),
            ProcessingStatus::Completed => "completed".to_string(),
            ProcessingStatus::Failed => "failed".to_string(),
        }
    }
}