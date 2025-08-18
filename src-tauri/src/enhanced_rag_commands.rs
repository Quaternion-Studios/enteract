use crate::enhanced_rag_system::{EnhancedRagSystem, EnhancedDocument, EnhancedDocumentChunk, EnhancedRagSettings};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

// Global enhanced RAG system instance
#[derive(Clone)]
pub struct EnhancedRagSystemState(pub Arc<Mutex<Option<EnhancedRagSystem>>>);

#[tauri::command]
pub async fn initialize_enhanced_rag_system(
    app_handle: tauri::AppHandle,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    // Check if already initialized
    {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        if rag_state.is_some() {
            return Ok("Enhanced RAG system already initialized".to_string());
        }
    }
    
    // Initialize new system
    match EnhancedRagSystem::new(&app_handle).await {
        Ok(system) => {
            let mut rag_state = state.0.lock().map_err(|e| e.to_string())?;
            *rag_state = Some(system);
            Ok("Enhanced RAG system initialized successfully".to_string())
        }
        Err(e) => Err(format!("Failed to initialize enhanced RAG system: {}", e))
    }
}

#[tauri::command]
pub async fn upload_enhanced_document(
    file_name: String,
    file_content: Vec<u8>,
    file_type: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<EnhancedDocument, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.upload_document(file_name, file_content, file_type)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_enhanced_documents(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<EnhancedDocument>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.get_all_documents()
                .map_err(|e| e.to_string())
        }
        None => Err("Enhanced RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn delete_enhanced_document(
    document_id: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.delete_document(&document_id)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(format!("Document {} deleted successfully", document_id))
}

#[tauri::command]
pub async fn search_enhanced_documents(
    query: String,
    context_document_ids: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<EnhancedDocumentChunk>, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.search_documents(&query, context_document_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_enhanced_embeddings(
    document_id: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.generate_embeddings(&document_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_enhanced_embedding_cache(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.clear_embedding_cache()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_enhanced_rag_settings(
    settings: EnhancedRagSettings,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.update_settings(settings)
                .map_err(|e| e.to_string())?;
            Ok("Settings updated successfully".to_string())
        }
        None => Err("Enhanced RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_enhanced_rag_settings(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<EnhancedRagSettings, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            Ok(system.get_settings())
        }
        None => Err("Enhanced RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_enhanced_storage_stats(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.get_storage_stats()
                .map_err(|e| e.to_string())
        }
        None => Err("Enhanced RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_embedding_status(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            let documents = system.get_all_documents().map_err(|e| e.to_string())?;
            let mut status = HashMap::new();
            
            let total_docs = documents.len();
            let completed_docs = documents.iter().filter(|d| d.embedding_status == "completed").count();
            let processing_docs = documents.iter().filter(|d| d.embedding_status == "processing").count();
            let failed_docs = documents.iter().filter(|d| d.embedding_status == "failed").count();
            
            status.insert("total_documents".to_string(), serde_json::json!(total_docs));
            status.insert("completed_documents".to_string(), serde_json::json!(completed_docs));
            status.insert("processing_documents".to_string(), serde_json::json!(processing_docs));
            status.insert("failed_documents".to_string(), serde_json::json!(failed_docs));
            status.insert("completion_percentage".to_string(), serde_json::json!(
                if total_docs > 0 { (completed_docs as f64 / total_docs as f64) * 100.0 } else { 0.0 }
            ));
            
            Ok(status)
        }
        None => Err("Enhanced RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn check_document_duplicate(
    file_name: String,
    file_content: Vec<u8>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    use sha2::{Sha256, Digest};
    
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    // Calculate content hash
    let mut hasher = Sha256::new();
    hasher.update(&file_content);
    hasher.update(file_name.as_bytes());
    let content_hash = format!("{:x}", hasher.finalize());
    
    // Check if duplicate exists
    let mut result = HashMap::new();
    match system.check_duplicate_public(&content_hash) {
        Ok(Some(doc)) => {
            result.insert("is_duplicate".to_string(), serde_json::json!(true));
            result.insert("existing_document".to_string(), serde_json::to_value(doc).unwrap());
        }
        Ok(None) => {
            result.insert("is_duplicate".to_string(), serde_json::json!(false));
        }
        Err(e) => {
            return Err(format!("Failed to check duplicate: {}", e));
        }
    }
    
    Ok(result)
}

#[tauri::command]
pub async fn get_document_embedding_status(
    document_ids: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, String>, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.get_embedding_status_for_documents(&document_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ensure_documents_ready_for_search(
    document_ids: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, String>, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.ensure_documents_ready_for_search(&document_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_embeddings_for_selection(
    document_ids: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("Enhanced RAG system not initialized".to_string())
        }
    }?;
    
    system.generate_embeddings_for_selection(&document_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_enhanced_file_upload(
    file_name: String,
    file_size: usize,
    file_type: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            let settings = system.get_settings();
            let mut validation = HashMap::new();
            
            let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
            let size_valid = file_size_mb <= settings.max_document_size_mb;
            
            // Check supported file types
            let supported_types = vec!["text/plain", "application/pdf", "text/markdown", 
                                     "application/msword", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"];
            let type_valid = supported_types.iter().any(|&t| file_type.contains(t)) || file_type.starts_with("text/");
            
            validation.insert("valid".to_string(), serde_json::json!(size_valid && type_valid));
            validation.insert("size_valid".to_string(), serde_json::json!(size_valid));
            validation.insert("type_valid".to_string(), serde_json::json!(type_valid));
            validation.insert("file_size_mb".to_string(), serde_json::json!(file_size_mb));
            validation.insert("max_size_mb".to_string(), serde_json::json!(settings.max_document_size_mb));
            validation.insert("supported_types".to_string(), serde_json::json!(supported_types));
            
            if !size_valid {
                validation.insert("error".to_string(), serde_json::json!(
                    format!("File size {:.2}MB exceeds limit of {:.2}MB", file_size_mb, settings.max_document_size_mb)
                ));
            } else if !type_valid {
                validation.insert("error".to_string(), serde_json::json!(
                    format!("File type '{}' is not supported", file_type)
                ));
            }
            
            Ok(validation)
        }
        None => Err("Enhanced RAG system not initialized".to_string())
    }
}