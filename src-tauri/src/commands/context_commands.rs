use anyhow::Result;
use serde_json::Value;
use tauri::State;
use std::sync::Arc;

use crate::rag::services::context_engine::{
    ContextEngine, ContextSession, ContextDocument, ContextAnalysis,
    ConversationMessage, ContextMode, EmbeddingStatus
};
use crate::state::AppState;

#[tauri::command]
pub async fn initialize_context_session(
    chat_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ContextSession, String> {
    let context_engine = &state.context_engine;
    
    context_engine
        .initialize_context_session(chat_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn analyze_conversation_context(
    messages: Vec<ConversationMessage>,
    state: State<'_, Arc<AppState>>,
) -> Result<ContextAnalysis, String> {
    let context_engine = &state.context_engine;
    
    context_engine
        .analyze_conversation_context(messages)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_cached_context_documents(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ContextDocument>, String> {
    let context_engine = &state.context_engine;
    
    context_engine
        .get_cached_context_documents()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_document_access(
    document_id: String,
    access_count: u32,
    last_accessed: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let context_engine = &state.context_engine;
    
    context_engine
        .update_document_access(document_id, access_count, last_accessed)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_context_documents(
    query: String,
    limit: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let context_engine = &state.context_engine;
    
    context_engine
        .search_context_documents(&query, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_context_for_message(
    message: String,
    document_ids: Vec<String>,
    max_chunks: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let context_engine = &state.context_engine;
    
    context_engine
        .get_context_for_message(&message, document_ids, max_chunks)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn process_document_embeddings(
    document_id: String,
    priority: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let context_engine = &state.context_engine;
    
    context_engine
        .process_document_embeddings(&document_id, &priority)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_context_session(
    session_id: String,
    mode: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let context_engine = &state.context_engine;
    
    // Parse mode string to ContextMode enum
    let context_mode = match mode.as_str() {
        "auto" => ContextMode::Auto,
        "manual" => ContextMode::Manual,
        "search" => ContextMode::Search,
        "all" => ContextMode::All,
        "none" => ContextMode::None,
        _ => return Err(format!("Invalid context mode: {}", mode)),
    };
    
    context_engine
        .update_context_session(&session_id, context_mode)
        .await
        .map_err(|e| e.to_string())
}

// Enhanced RAG search with context awareness
#[tauri::command]
pub async fn search_with_context(
    query: String,
    context_document_ids: Vec<String>,
    limit: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Value>, String> {
    let context_engine = &state.context_engine;
    
    // First get context chunks from specified documents
    let context_chunks = context_engine
        .get_context_for_message(&query, context_document_ids.clone(), 5)
        .await
        .map_err(|e| e.to_string())?;
    
    // Combine context with query for enhanced search
    let enhanced_query = if !context_chunks.is_empty() {
        format!("{} Context: {}", query, context_chunks.join(" "))
    } else {
        query
    };
    
    // Perform search with enhanced query
    let search_results = context_engine
        .search_context_documents(&enhanced_query, limit)
        .await
        .map_err(|e| e.to_string())?;
    
    // Convert to JSON values for frontend
    Ok(search_results
        .into_iter()
        .map(|id| serde_json::json!({ "document_id": id }))
        .collect())
}

// Get smart document suggestions for current conversation
#[tauri::command]
pub async fn get_smart_suggestions(
    recent_messages: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<Value>, String> {
    let context_engine = &state.context_engine;
    
    // Convert strings to ConversationMessage format
    let messages: Vec<ConversationMessage> = recent_messages
        .into_iter()
        .enumerate()
        .map(|(i, content)| ConversationMessage {
            role: if i % 2 == 0 { "user".to_string() } else { "assistant".to_string() },
            content,
        })
        .collect();
    
    // Analyze conversation and get suggestions
    let analysis = context_engine
        .analyze_conversation_context(messages)
        .await
        .map_err(|e| e.to_string())?;
    
    // Convert suggestions to JSON
    Ok(analysis
        .suggested_documents
        .into_iter()
        .map(|suggestion| {
            serde_json::json!({
                "document_id": suggestion.document_id,
                "document_name": suggestion.document_name,
                "relevance_score": suggestion.relevance_score,
                "reason": suggestion.reason,
                "preview": suggestion.preview,
                "confidence": suggestion.confidence,
                "relevant_chunks": suggestion.relevant_chunks,
            })
        })
        .collect())
}