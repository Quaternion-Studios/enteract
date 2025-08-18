// Database connection pooling for SQLite
// Manages connections to avoid creating too many connections and improve performance

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use rusqlite::{Connection, Result as SqliteResult};
use tauri::{AppHandle, Manager};
use crate::data::errors::{DatabaseError, DatabaseErrorType, DatabaseResult};

#[derive(Debug)]
pub struct PooledConnection {
    pub connection: Connection,
    pub created_at: Instant,
    pub last_used: Instant,
    pub use_count: u64,
}

impl PooledConnection {
    fn new(connection: Connection) -> Self {
        let now = Instant::now();
        Self {
            connection,
            created_at: now,
            last_used: now,
            use_count: 0,
        }
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }

    fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }

    fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }
}

#[derive(Debug)]
pub struct ConnectionPoolConfig {
    pub max_connections: usize,
    pub max_connection_age: Duration,
    pub idle_timeout: Duration,
    pub connection_timeout: Duration,
    pub cleanup_interval: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            max_connection_age: Duration::from_secs(3600), // 1 hour
            idle_timeout: Duration::from_secs(300),         // 5 minutes
            connection_timeout: Duration::from_secs(30),
            cleanup_interval: Duration::from_secs(60),      // 1 minute
        }
    }
}

#[derive(Debug)]
pub struct ConnectionPoolStats {
    pub active_connections: usize,
    pub total_created: u64,
    pub total_closed: u64,
    pub total_borrowed: u64,
    pub total_returned: u64,
    pub connection_errors: u64,
    pub cleanup_runs: u64,
}

impl Default for ConnectionPoolStats {
    fn default() -> Self {
        Self {
            active_connections: 0,
            total_created: 0,
            total_closed: 0,
            total_borrowed: 0,
            total_returned: 0,
            connection_errors: 0,
            cleanup_runs: 0,
        }
    }
}

#[derive(Debug)]
pub struct ConnectionPool {
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    config: ConnectionPoolConfig,
    stats: Arc<Mutex<ConnectionPoolStats>>,
    db_path: std::path::PathBuf,
    last_cleanup: Arc<Mutex<Instant>>,
}

impl ConnectionPool {
    pub fn new(app_handle: &AppHandle, config: Option<ConnectionPoolConfig>) -> DatabaseResult<Self> {
        let db_path = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| DatabaseError::new(
                DatabaseErrorType::InitializationFailed,
                format!("Failed to get app data directory: {}", e),
                "connection_pool_init".to_string(),
            ))?
            .join("enteract_data.db");

        let config = config.unwrap_or_default();
        
        Ok(Self {
            connections: Arc::new(Mutex::new(VecDeque::new())),
            config,
            stats: Arc::new(Mutex::new(ConnectionPoolStats::default())),
            db_path,
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        })
    }

    pub fn get_connection(&self) -> DatabaseResult<PooledConnection> {
        // Check if cleanup is needed
        self.maybe_cleanup();

        let mut connections = self.connections.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire connection pool lock".to_string(),
                "get_connection".to_string(),
            )
        })?;

        let mut stats = self.stats.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire stats lock".to_string(),
                "get_connection".to_string(),
            )
        })?;

        // Try to reuse an existing connection
        while let Some(mut conn) = connections.pop_front() {
            // Check if connection is still valid
            if !conn.is_expired(self.config.max_connection_age) && !conn.is_idle(self.config.idle_timeout) {
                // Test connection health
                if self.test_connection(&conn.connection).is_ok() {
                    conn.mark_used();
                    stats.total_borrowed += 1;
                    return Ok(conn);
                }
            }
            
            // Connection is expired or invalid, close it
            stats.total_closed += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }

        // Create new connection if under limit
        if stats.active_connections < self.config.max_connections {
            match self.create_connection() {
                Ok(mut conn) => {
                    conn.mark_used();
                    stats.total_created += 1;
                    stats.total_borrowed += 1;
                    stats.active_connections += 1;
                    Ok(conn)
                }
                Err(e) => {
                    stats.connection_errors += 1;
                    Err(DatabaseError::new(
                        DatabaseErrorType::ConnectionFailed,
                        format!("Failed to create new connection: {}", e),
                        "create_connection".to_string(),
                    ))
                }
            }
        } else {
            stats.connection_errors += 1;
            Err(DatabaseError::new(
                DatabaseErrorType::ConnectionFailed,
                "Connection pool exhausted".to_string(),
                "get_connection".to_string(),
            ).retry_after(1))
        }
    }

    pub fn return_connection(&self, connection: PooledConnection) -> DatabaseResult<()> {
        let mut connections = self.connections.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire connection pool lock".to_string(),
                "return_connection".to_string(),
            )
        })?;

        let mut stats = self.stats.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire stats lock".to_string(),
                "return_connection".to_string(),
            )
        })?;

        // Only return connection if it's not expired and pool isn't full
        if !connection.is_expired(self.config.max_connection_age) 
            && connections.len() < self.config.max_connections {
            connections.push_back(connection);
            stats.total_returned += 1;
        } else {
            // Connection expired or pool full, close it
            stats.total_closed += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }

        Ok(())
    }

    fn create_connection(&self) -> SqliteResult<PooledConnection> {
        let connection = Connection::open(&self.db_path)?;
        
        // Configure the connection
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        // WAL mode returns a result, so use query_row
        connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get::<_, String>(0)).ok();
        connection.execute("PRAGMA synchronous = NORMAL", [])?;
        connection.execute("PRAGMA cache_size = 10000", [])?;
        connection.execute("PRAGMA temp_store = memory", [])?;
        
        Ok(PooledConnection::new(connection))
    }

    fn test_connection(&self, connection: &Connection) -> SqliteResult<()> {
        connection.execute("SELECT 1", [])?;
        Ok(())
    }

    fn maybe_cleanup(&self) {
        let mut last_cleanup = match self.last_cleanup.lock() {
            Ok(guard) => guard,
            Err(_) => return, // Skip cleanup if we can't get lock
        };

        if last_cleanup.elapsed() >= self.config.cleanup_interval {
            *last_cleanup = Instant::now();
            drop(last_cleanup); // Release lock before cleanup
            
            let _ = self.cleanup();
        }
    }

    fn cleanup(&self) -> DatabaseResult<()> {
        let mut connections = self.connections.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire connection pool lock for cleanup".to_string(),
                "cleanup".to_string(),
            )
        })?;

        let mut stats = self.stats.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire stats lock for cleanup".to_string(),
                "cleanup".to_string(),
            )
        })?;

        let mut cleaned = 0;
        let mut retained = VecDeque::new();

        while let Some(conn) = connections.pop_front() {
            if conn.is_expired(self.config.max_connection_age) || conn.is_idle(self.config.idle_timeout) {
                cleaned += 1;
                stats.total_closed += 1;
                stats.active_connections = stats.active_connections.saturating_sub(1);
            } else {
                retained.push_back(conn);
            }
        }

        *connections = retained;
        stats.cleanup_runs += 1;

        if cleaned > 0 {
            println!("🧹 Connection pool cleanup: removed {} expired/idle connections", cleaned);
        }

        Ok(())
    }

    pub fn get_stats(&self) -> DatabaseResult<ConnectionPoolStats> {
        let stats = self.stats.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire stats lock".to_string(),
                "get_stats".to_string(),
            )
        })?;

        let connections = self.connections.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire connection pool lock".to_string(),
                "get_stats".to_string(),
            )
        })?;

        let mut result = ConnectionPoolStats {
            active_connections: connections.len(),
            total_created: stats.total_created,
            total_closed: stats.total_closed,
            total_borrowed: stats.total_borrowed,
            total_returned: stats.total_returned,
            connection_errors: stats.connection_errors,
            cleanup_runs: stats.cleanup_runs,
        };
        
        Ok(result)
    }

    pub fn close_all(&self) -> DatabaseResult<()> {
        let mut connections = self.connections.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire connection pool lock for close_all".to_string(),
                "close_all".to_string(),
            )
        })?;

        let mut stats = self.stats.lock().map_err(|_| {
            DatabaseError::new(
                DatabaseErrorType::LockTimeout,
                "Failed to acquire stats lock for close_all".to_string(),
                "close_all".to_string(),
            )
        })?;

        let closed_count = connections.len();
        connections.clear();
        
        stats.total_closed += closed_count as u64;
        stats.active_connections = 0;

        println!("🔒 Connection pool closed: {} connections terminated", closed_count);
        Ok(())
    }
}

// RAII wrapper for automatic connection return
pub struct ManagedConnection {
    connection: Option<PooledConnection>,
    pool: Arc<ConnectionPool>,
}

impl ManagedConnection {
    pub fn new(connection: PooledConnection, pool: Arc<ConnectionPool>) -> Self {
        Self {
            connection: Some(connection),
            pool,
        }
    }

    pub fn get(&mut self) -> &mut Connection {
        &mut self.connection.as_mut().unwrap().connection
    }
}

impl Drop for ManagedConnection {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let _ = self.pool.return_connection(connection);
        }
    }
}