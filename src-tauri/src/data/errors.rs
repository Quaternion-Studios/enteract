// Error handling types for the database layer
use serde::{Serialize, Deserialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseErrorType {
    ConnectionFailed,
    InitializationFailed,
    SchemaError,
    QueryFailed,
    TransactionFailed,
    ConstraintViolation,
    DataCorruption,
    PermissionDenied,
    DiskFull,
    LockTimeout,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseError {
    pub error_type: DatabaseErrorType,
    pub message: String,
    pub details: Option<String>,
    pub code: Option<i32>,
    pub timestamp: i64,
    pub operation: String,
    pub recoverable: bool,
    pub retry_after: Option<u64>, // seconds
}

impl DatabaseError {
    pub fn new(
        error_type: DatabaseErrorType,
        message: String,
        operation: String,
    ) -> Self {
        Self {
            error_type,
            message,
            details: None,
            code: None,
            timestamp: chrono::Utc::now().timestamp(),
            operation,
            recoverable: false,
            retry_after: None,
        }
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_code(mut self, code: i32) -> Self {
        self.code = Some(code);
        self
    }

    pub fn recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }

    pub fn retry_after(mut self, seconds: u64) -> Self {
        self.retry_after = Some(seconds);
        self.recoverable = true;
        self
    }

    pub fn from_sqlite_error(err: rusqlite::Error, operation: String) -> Self {
        let (error_type, recoverable, retry_after) = match &err {
            rusqlite::Error::SqliteFailure(code, _) => {
                use rusqlite::ErrorCode;
                match code.code {
                    ErrorCode::DatabaseBusy => (DatabaseErrorType::LockTimeout, true, Some(1)),
                    ErrorCode::DatabaseLocked => (DatabaseErrorType::LockTimeout, true, Some(1)),
                    ErrorCode::CannotOpen => (DatabaseErrorType::PermissionDenied, false, None),
                    ErrorCode::DiskFull => (DatabaseErrorType::DiskFull, false, None),
                    ErrorCode::ConstraintViolation => (DatabaseErrorType::ConstraintViolation, false, None),
                    ErrorCode::DatabaseCorrupt => (DatabaseErrorType::DataCorruption, false, None),
                    _ => (DatabaseErrorType::QueryFailed, false, None),
                }
            }
            rusqlite::Error::QueryReturnedNoRows => (DatabaseErrorType::QueryFailed, false, None),
            _ => (DatabaseErrorType::Unknown, false, None),
        };

        DatabaseError {
            error_type,
            message: err.to_string(),
            details: None,
            code: None,
            timestamp: chrono::Utc::now().timestamp(),
            operation,
            recoverable,
            retry_after,
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] {} (Operation: {})",
            self.error_type, self.message, self.operation
        )
    }
}

impl std::error::Error for DatabaseError {}

impl From<DatabaseError> for String {
    fn from(err: DatabaseError) -> String {
        serde_json::to_string(&err).unwrap_or_else(|_| err.to_string())
    }
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<DatabaseError>,
    pub operation_id: String,
    pub duration_ms: u64,
    pub timestamp: i64,
}

impl<T> OperationResult<T> {
    pub fn success(data: T, operation_id: String, duration_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            operation_id,
            duration_ms,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn error(error: DatabaseError, operation_id: String, duration_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            operation_id,
            duration_ms,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

// Utility functions for common error scenarios
pub fn connection_error(details: &str) -> DatabaseError {
    DatabaseError::new(
        DatabaseErrorType::ConnectionFailed,
        "Failed to connect to database".to_string(),
        "database_connection".to_string(),
    )
    .with_details(details.to_string())
}

pub fn initialization_error(details: &str) -> DatabaseError {
    DatabaseError::new(
        DatabaseErrorType::InitializationFailed,
        "Database initialization failed".to_string(),
        "database_initialization".to_string(),
    )
    .with_details(details.to_string())
}

pub fn permission_error(details: &str) -> DatabaseError {
    DatabaseError::new(
        DatabaseErrorType::PermissionDenied,
        "Database permission denied".to_string(),
        "database_access".to_string(),
    )
    .with_details(details.to_string())
}

pub fn lock_timeout_error() -> DatabaseError {
    DatabaseError::new(
        DatabaseErrorType::LockTimeout,
        "Database lock timeout".to_string(),
        "database_lock".to_string(),
    )
    .retry_after(1)
}