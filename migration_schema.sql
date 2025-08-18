-- SQLite Migration Schema for Enteract Data Storage
-- Migrating from JSON to SQLite for improved performance and scalability

-- Enable foreign keys and WAL mode for better performance
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = 10000;
PRAGMA temp_store = memory;

-- ============================================================================
-- CHAT SESSIONS TABLES (Main Claude Chat)
-- ============================================================================

-- Main chat sessions
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    model_id TEXT,
    INDEX(created_at),
    INDEX(updated_at)
);

-- Chat messages (normalized to reduce redundancy)
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
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    INDEX(session_id),
    INDEX(timestamp),
    INDEX(sender)
);

-- Message attachments (separate table for better normalization)
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
    FOREIGN KEY (message_id) REFERENCES chat_messages(id) ON DELETE CASCADE,
    INDEX(message_id),
    INDEX(type)
);

-- Thinking processes (for assistant reasoning)
CREATE TABLE IF NOT EXISTS thinking_processes (
    id INTEGER PRIMARY KEY,
    message_id INTEGER NOT NULL,
    is_visible INTEGER NOT NULL CHECK(is_visible IN (0, 1)),
    content TEXT NOT NULL,
    is_streaming INTEGER NOT NULL CHECK(is_streaming IN (0, 1)),
    FOREIGN KEY (message_id) REFERENCES chat_messages(id) ON DELETE CASCADE,
    INDEX(message_id)
);

-- Thinking steps
CREATE TABLE IF NOT EXISTS thinking_steps (
    id TEXT PRIMARY KEY,
    thinking_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    status TEXT NOT NULL,
    FOREIGN KEY (thinking_id) REFERENCES thinking_processes(id) ON DELETE CASCADE,
    INDEX(thinking_id),
    INDEX(timestamp)
);

-- Message metadata
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
    FOREIGN KEY (message_id) REFERENCES chat_messages(id) ON DELETE CASCADE,
    INDEX(message_id),
    INDEX(model)
);

-- ============================================================================
-- CONVERSATION SESSIONS TABLES (Audio Conversations)
-- ============================================================================

-- Audio conversation sessions
CREATE TABLE IF NOT EXISTS conversation_sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    is_active INTEGER NOT NULL CHECK(is_active IN (0, 1)),
    INDEX(start_time),
    INDEX(is_active)
);

-- Conversation messages
CREATE TABLE IF NOT EXISTS conversation_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('user', 'system')),
    source TEXT NOT NULL CHECK(source IN ('microphone', 'loopback')),
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    confidence REAL,
    FOREIGN KEY (session_id) REFERENCES conversation_sessions(id) ON DELETE CASCADE,
    INDEX(session_id),
    INDEX(timestamp),
    INDEX(type),
    INDEX(source)
);

-- Conversation insights
CREATE TABLE IF NOT EXISTS conversation_insights (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    text TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    context_length INTEGER NOT NULL,
    insight_type TEXT NOT NULL CHECK(insight_type IN ('insight', 'welcome', 'question', 'answer')),
    FOREIGN KEY (session_id) REFERENCES conversation_sessions(id) ON DELETE CASCADE,
    INDEX(session_id),
    INDEX(timestamp),
    INDEX(insight_type)
);

-- ============================================================================
-- MIGRATION TRACKING
-- ============================================================================

-- Track migration status and metadata
CREATE TABLE IF NOT EXISTS migration_status (
    id INTEGER PRIMARY KEY,
    migration_name TEXT NOT NULL UNIQUE,
    completed_at TEXT NOT NULL,
    records_migrated INTEGER,
    original_file_size INTEGER,
    notes TEXT
);

-- ============================================================================
-- INDEXES FOR PERFORMANCE
-- ============================================================================

-- Additional composite indexes for common queries
CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated_desc ON chat_sessions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_chat_messages_session_timestamp ON chat_messages(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_conversation_sessions_active_start ON conversation_sessions(is_active, start_time DESC);
CREATE INDEX IF NOT EXISTS idx_conversation_messages_session_timestamp ON conversation_messages(session_id, timestamp);

-- ============================================================================
-- VIEWS FOR EASY DATA ACCESS
-- ============================================================================

-- View for complete chat session data (similar to current JSON structure)
CREATE VIEW IF NOT EXISTS chat_session_summary AS
SELECT 
    cs.id,
    cs.title,
    cs.created_at,
    cs.updated_at,
    cs.model_id,
    COUNT(cm.id) as message_count,
    MAX(cm.timestamp) as last_message_time
FROM chat_sessions cs
LEFT JOIN chat_messages cm ON cs.id = cm.session_id
GROUP BY cs.id, cs.title, cs.created_at, cs.updated_at, cs.model_id;

-- View for conversation session summary
CREATE VIEW IF NOT EXISTS conversation_session_summary AS
SELECT 
    cs.id,
    cs.name,
    cs.start_time,
    cs.end_time,
    cs.is_active,
    COUNT(cm.id) as message_count,
    COUNT(ci.id) as insight_count,
    (cs.end_time - cs.start_time) / 1000.0 as duration_seconds
FROM conversation_sessions cs
LEFT JOIN conversation_messages cm ON cs.id = cm.session_id
LEFT JOIN conversation_insights ci ON cs.id = ci.session_id
GROUP BY cs.id, cs.name, cs.start_time, cs.end_time, cs.is_active;

-- ============================================================================
-- PERFORMANCE OPTIMIZATION SETTINGS
-- ============================================================================

-- Analyze tables for query optimization
ANALYZE;