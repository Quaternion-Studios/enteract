use crate::rag_system::{Document, DocumentChunk, RagSettings, RagSystem};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

use std::sync::Arc;

// Global RAG system instance
#[derive(Clone)]
pub struct RagSystemState(pub Arc<Mutex<Option<RagSystem>>>);

#[tauri::command]
pub async fn initialize_rag_system(
    app_handle: tauri::AppHandle,
    state: State<'_, RagSystemState>,
) -> Result<String, String> {
    let mut rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match RagSystem::new(&app_handle) {
        Ok(system) => {
            *rag_state = Some(system);
            Ok("RAG system initialized successfully".to_string())
        }
        Err(e) => Err(format!("Failed to initialize RAG system: {}", e))
    }
}

#[tauri::command]
pub async fn upload_document(
    file_name: String,
    file_content: Vec<u8>,
    file_type: String,
    state: State<'_, RagSystemState>,
) -> Result<Document, String> {
    // Clone the system reference to avoid holding the lock across await
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.upload_document(file_name, file_content, file_type)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_documents(
    state: State<'_, RagSystemState>,
) -> Result<Vec<Document>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.get_all_documents()
                .map_err(|e| e.to_string())
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn delete_document(
    document_id: String,
    state: State<'_, RagSystemState>,
) -> Result<String, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.delete_document(&document_id)
                .map_err(|e| e.to_string())?;
            Ok(format!("Document {} deleted successfully", document_id))
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn search_documents(
    query: String,
    context_document_ids: Vec<String>,
    state: State<'_, RagSystemState>,
) -> Result<Vec<DocumentChunk>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.search_documents(&query, context_document_ids)
                .map_err(|e| e.to_string())
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn update_rag_settings(
    settings: RagSettings,
    state: State<'_, RagSystemState>,
) -> Result<String, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.update_settings(settings)
                .map_err(|e| e.to_string())?;
            Ok("Settings updated successfully".to_string())
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_rag_settings(
    state: State<'_, RagSystemState>,
) -> Result<RagSettings, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            Ok(system.get_settings())
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_storage_stats(
    state: State<'_, RagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.get_storage_stats()
                .map_err(|e| e.to_string())
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn generate_embeddings(
    document_id: String,
    _state: State<'_, RagSystemState>,
) -> Result<String, String> {
    // TODO: Implement embedding generation using a local model
    // For now, return a placeholder
    Ok(format!("Embeddings for document {} will be generated", document_id))
}

#[tauri::command]
pub async fn clear_embedding_cache(
    _state: State<'_, RagSystemState>,
) -> Result<String, String> {
    // TODO: Implement cache clearing
    Ok("Embedding cache cleared".to_string())
}