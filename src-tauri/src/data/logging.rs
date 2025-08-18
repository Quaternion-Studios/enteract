// Comprehensive logging system for database operations
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub timestamp: i64,
    pub operation: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub duration_ms: Option<u64>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub thread_id: Option<String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, operation: String, message: String) -> Self {
        Self {
            level,
            timestamp: chrono::Utc::now().timestamp_millis(),
            operation,
            message,
            details: None,
            duration_ms: None,
            session_id: None,
            user_id: None,
            thread_id: Some(format!("{:?}", std::thread::current().id())),
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_duration(mut self, start_time: Instant) -> Self {
        self.duration_ms = Some(start_time.elapsed().as_millis() as u64);
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

#[derive(Debug)]
pub struct DatabaseLogger {
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    max_logs: usize,
    min_level: LogLevel,
}

impl DatabaseLogger {
    pub fn new(max_logs: usize, min_level: LogLevel) -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(max_logs))),
            max_logs,
            min_level,
        }
    }

    pub fn log(&self, entry: LogEntry) {
        if !self.should_log(&entry.level) {
            return;
        }

        let mut logs = match self.logs.lock() {
            Ok(guard) => guard,
            Err(_) => {
                eprintln!("Failed to acquire log mutex for entry: {:?}", entry);
                return;
            }
        };

        // Add new log entry
        logs.push_back(entry.clone());

        // Remove old entries if we exceed max_logs
        while logs.len() > self.max_logs {
            logs.pop_front();
        }

        // Also print to console for immediate visibility
        self.print_to_console(&entry);
    }

    fn should_log(&self, level: &LogLevel) -> bool {
        use LogLevel::*;
        let level_priority = match level {
            Trace => 0,
            Debug => 1,
            Info => 2,
            Warn => 3,
            Error => 4,
            Critical => 5,
        };

        let min_priority = match self.min_level {
            Trace => 0,
            Debug => 1,
            Info => 2,
            Warn => 3,
            Error => 4,
            Critical => 5,
        };

        level_priority >= min_priority
    }

    fn print_to_console(&self, entry: &LogEntry) {
        let level_emoji = match entry.level {
            LogLevel::Trace => "🔍",
            LogLevel::Debug => "🐛", 
            LogLevel::Info => "ℹ️",
            LogLevel::Warn => "⚠️",
            LogLevel::Error => "❌",
            LogLevel::Critical => "🚨",
        };

        let duration_str = entry.duration_ms
            .map(|d| format!(" ({}ms)", d))
            .unwrap_or_default();

        let session_str = entry.session_id
            .as_ref()
            .map(|s| format!(" [{}]", s))
            .unwrap_or_default();

        println!(
            "{} [{}]{}{}: {}",
            level_emoji,
            entry.operation,
            session_str,
            duration_str,
            entry.message
        );

        if let Some(details) = &entry.details {
            println!("   Details: {}", serde_json::to_string_pretty(details).unwrap_or_default());
        }
    }

    pub fn trace(&self, operation: String, message: String) {
        self.log(LogEntry::new(LogLevel::Trace, operation, message));
    }

    pub fn debug(&self, operation: String, message: String) {
        self.log(LogEntry::new(LogLevel::Debug, operation, message));
    }

    pub fn info(&self, operation: String, message: String) {
        self.log(LogEntry::new(LogLevel::Info, operation, message));
    }

    pub fn warn(&self, operation: String, message: String) {
        self.log(LogEntry::new(LogLevel::Warn, operation, message));
    }

    pub fn error(&self, operation: String, message: String) {
        self.log(LogEntry::new(LogLevel::Error, operation, message));
    }

    pub fn critical(&self, operation: String, message: String) {
        self.log(LogEntry::new(LogLevel::Critical, operation, message));
    }

    pub fn get_logs(&self, last_n: Option<usize>) -> Result<Vec<LogEntry>, String> {
        let logs = self.logs.lock()
            .map_err(|_| "Failed to acquire log mutex".to_string())?;

        let entries: Vec<LogEntry> = logs.iter().cloned().collect();
        
        if let Some(n) = last_n {
            let start_idx = entries.len().saturating_sub(n);
            Ok(entries[start_idx..].to_vec())
        } else {
            Ok(entries)
        }
    }

    pub fn get_logs_by_operation(&self, operation: &str) -> Result<Vec<LogEntry>, String> {
        let logs = self.logs.lock()
            .map_err(|_| "Failed to acquire log mutex".to_string())?;

        let filtered: Vec<LogEntry> = logs.iter()
            .filter(|entry| entry.operation == operation)
            .cloned()
            .collect();

        Ok(filtered)
    }

    pub fn get_logs_by_level(&self, level: LogLevel) -> Result<Vec<LogEntry>, String> {
        let logs = self.logs.lock()
            .map_err(|_| "Failed to acquire log mutex".to_string())?;

        let filtered: Vec<LogEntry> = logs.iter()
            .filter(|entry| matches!((&entry.level, &level), 
                (LogLevel::Trace, LogLevel::Trace) |
                (LogLevel::Debug, LogLevel::Debug) |
                (LogLevel::Info, LogLevel::Info) |
                (LogLevel::Warn, LogLevel::Warn) |
                (LogLevel::Error, LogLevel::Error) |
                (LogLevel::Critical, LogLevel::Critical)
            ))
            .cloned()
            .collect();

        Ok(filtered)
    }

    pub fn clear_logs(&self) -> Result<(), String> {
        let mut logs = self.logs.lock()
            .map_err(|_| "Failed to acquire log mutex".to_string())?;
        logs.clear();
        Ok(())
    }

    pub fn get_stats(&self) -> Result<LogStats, String> {
        let logs = self.logs.lock()
            .map_err(|_| "Failed to acquire log mutex".to_string())?;

        let total_logs = logs.len();
        let mut level_counts = [0; 6]; // Trace, Debug, Info, Warn, Error, Critical

        for entry in logs.iter() {
            let index = match entry.level {
                LogLevel::Trace => 0,
                LogLevel::Debug => 1,
                LogLevel::Info => 2,
                LogLevel::Warn => 3,
                LogLevel::Error => 4,
                LogLevel::Critical => 5,
            };
            level_counts[index] += 1;
        }

        Ok(LogStats {
            total_logs,
            trace_count: level_counts[0],
            debug_count: level_counts[1],
            info_count: level_counts[2],
            warn_count: level_counts[3],
            error_count: level_counts[4],
            critical_count: level_counts[5],
            oldest_log_timestamp: logs.front().map(|e| e.timestamp),
            newest_log_timestamp: logs.back().map(|e| e.timestamp),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogStats {
    pub total_logs: usize,
    pub trace_count: usize,
    pub debug_count: usize,
    pub info_count: usize,
    pub warn_count: usize,
    pub error_count: usize,
    pub critical_count: usize,
    pub oldest_log_timestamp: Option<i64>,
    pub newest_log_timestamp: Option<i64>,
}

// Global logger instance
lazy_static::lazy_static! {
    pub static ref DB_LOGGER: DatabaseLogger = DatabaseLogger::new(1000, LogLevel::Info);
}

// Convenience macros for logging
#[macro_export]
macro_rules! db_trace {
    ($operation:expr, $($arg:tt)*) => {
        crate::data::logging::DB_LOGGER.trace($operation.to_string(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! db_debug {
    ($operation:expr, $($arg:tt)*) => {
        crate::data::logging::DB_LOGGER.debug($operation.to_string(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! db_info {
    ($operation:expr, $($arg:tt)*) => {
        crate::data::logging::DB_LOGGER.info($operation.to_string(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! db_warn {
    ($operation:expr, $($arg:tt)*) => {
        crate::data::logging::DB_LOGGER.warn($operation.to_string(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! db_error {
    ($operation:expr, $($arg:tt)*) => {
        crate::data::logging::DB_LOGGER.error($operation.to_string(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! db_critical {
    ($operation:expr, $($arg:tt)*) => {
        crate::data::logging::DB_LOGGER.critical($operation.to_string(), format!($($arg)*))
    };
}

// Tauri commands for log access
use tauri::command;

#[command]
pub fn get_database_logs(last_n: Option<usize>) -> Result<Vec<LogEntry>, String> {
    DB_LOGGER.get_logs(last_n)
}

#[command]
pub fn get_database_logs_by_operation(operation: String) -> Result<Vec<LogEntry>, String> {
    DB_LOGGER.get_logs_by_operation(&operation)
}

#[command]
pub fn get_database_logs_by_level(level: LogLevel) -> Result<Vec<LogEntry>, String> {
    DB_LOGGER.get_logs_by_level(level)
}

#[command]
pub fn get_database_log_stats() -> Result<LogStats, String> {
    DB_LOGGER.get_stats()
}

#[command]
pub fn clear_database_logs() -> Result<(), String> {
    DB_LOGGER.clear_logs()
}