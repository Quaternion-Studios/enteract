use tauri::State;
use std::collections::HashMap;
use serde_json::Value;

use super::commands::EnhancedRagSystemState;
use crate::rag::services::{ContextSuggestion, RelatedDocument, FileChangeEvent};
use crate::rag::services::context_engine::{ContextSession, ContextDocument, ContextAnalysis, ConversationMessage, ContextMode};

#[tauri::command]
pub async fn initialize_context_session(
    chat_id: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<ContextSession, String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
    context_engine
        .initialize_context_session(chat_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_cached_context_documents(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<ContextDocument>, String> {
    let (context_engine, rag_system) = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => (system.context_engine.clone(), system.clone()),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
    // First, sync context cache with RAG documents
    let rag_documents = rag_system.get_all_documents().map_err(|e| e.to_string())?;
    context_engine
        .sync_with_rag_documents(rag_documents)
        .await
        .map_err(|e| e.to_string())?;
    
    // Then return cached documents
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
    state: State<'_, EnhancedRagSystemState>,
) -> Result<(), String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
    context_engine
        .update_document_access(document_id, access_count, last_accessed)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_context_documents(
    query: String,
    limit: usize,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<crate::rag::services::RankedDocument>, String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
    let mut results = context_engine
        .search_all_documents(&query)
        .await
        .map_err(|e| e.to_string())?;
    
    // Truncate to requested limit
    if results.len() > limit { results.truncate(limit); }
    Ok(results)
}

#[tauri::command]
pub async fn get_context_for_message(
    message: String,
    document_ids: Vec<String>,
    max_chunks: usize,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<String>, String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
    context_engine
        .get_context_for_message(&message, document_ids, max_chunks)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn process_document_embeddings(
    document_id: String,
    priority: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<(), String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
    context_engine
        .process_document_embeddings(&document_id, &priority)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_context_session(
    session_id: String,
    mode: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<(), String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
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



#[tauri::command]
pub async fn get_context_suggestions(
    conversation_history: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<ContextSuggestion>, String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };

    context_engine
        .suggest_context(&conversation_history)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_related_documents(
    document_ids: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<RelatedDocument>, String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };

    context_engine
        .get_related_documents(&document_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn analyze_conversation_context(
    messages: Vec<ConversationMessage>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<ContextAnalysis, String> {
    let context_engine = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(system) => system.context_engine.clone(),
            None => return Err("RAG system not initialized".to_string())
        }
    };
    
    context_engine
        .analyze_conversation_context(messages)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_file_changes(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<FileChangeEvent>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    match &*rag_state {
        Some(system) => {
            // Scan current documents for missing files and report deletions
            let docs = system.get_all_documents().map_err(|e| e.to_string())?;
            let mut events = Vec::new();
            for doc in docs {
                if !doc.file_path.is_empty() && !std::path::Path::new(&doc.file_path).exists() {
                    events.push(FileChangeEvent {
                        file_path: doc.file_path.clone(),
                        event_type: crate::rag::services::FileEventType::Deleted,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
            Ok(events)
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn cleanup_orphaned_documents(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<String>, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    let documents = system.get_all_documents().map_err(|e| e.to_string())?;
    let mut cleaned_up = Vec::new();
    
    for doc in documents {
        if !doc.file_path.is_empty() && !std::path::Path::new(&doc.file_path).exists() {
            match system.delete_document(&doc.id).await {
                Ok(_) => {
                    cleaned_up.push(doc.id);
                }
                Err(e) => {
                    eprintln!("Failed to clean up orphaned document: {}", e);
                }
            }
        }
    }
    
    Ok(cleaned_up)
}

#[tauri::command]
pub async fn get_document_analytics(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            let documents = system.get_all_documents().map_err(|e| e.to_string())?;
            let mut analytics = HashMap::new();
            
            // Basic analytics
            analytics.insert("total_documents".to_string(), serde_json::json!(documents.len()));
            
            let total_access_count: i32 = documents.iter().map(|d| d.access_count).sum();
            analytics.insert("total_access_count".to_string(), serde_json::json!(total_access_count));
            
            let avg_access_count = if !documents.is_empty() {
                total_access_count as f64 / documents.len() as f64
            } else {
                0.0
            };
            analytics.insert("average_access_count".to_string(), serde_json::json!(avg_access_count));
            
            // Most accessed documents
            let mut sorted_docs = documents.clone();
            sorted_docs.sort_by(|a, b| b.access_count.cmp(&a.access_count));
            let top_docs: Vec<_> = sorted_docs.into_iter().take(5).map(|d| {
                serde_json::json!({
                    "id": d.id,
                    "name": d.file_name,
                    "access_count": d.access_count
                })
            }).collect();
            analytics.insert("most_accessed".to_string(), serde_json::json!(top_docs));
            
            // File type distribution
            let mut type_counts = HashMap::new();
            for doc in &documents {
                *type_counts.entry(doc.file_type.clone()).or_insert(0) += 1;
            }
            analytics.insert("file_type_distribution".to_string(), serde_json::json!(type_counts));
            
            Ok(analytics)
        }
        None => Err("RAG system not initialized".to_string())
    }
}