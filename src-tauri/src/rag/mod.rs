// RAG (Retrieval-Augmented Generation) Module
// 
// This module provides comprehensive document retrieval and search capabilities
// using hybrid search (BM25 + vector embeddings) with Tantivy and FastEmbed.

pub mod types;
pub mod storage;
pub mod embeddings;
pub mod chunking;
pub mod search;
pub mod system;
pub mod commands;

// Re-export main types and functions
pub use types::*;
pub use system::EnhancedRagSystem;
pub use commands::*;