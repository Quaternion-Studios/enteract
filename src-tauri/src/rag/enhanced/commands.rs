use super::system::{EnhancedRagSystem, EnhancedDocument, EnhancedDocumentChunk, EnhancedRagSettings};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

// Global RAG system instance
#[derive(Clone)]
pub struct EnhancedRagSystemState(pub Arc<Mutex<Option<EnhancedRagSystem>>>);

#[tauri::command]
pub async fn initialize_rag_system(
    app_handle: tauri::AppHandle,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    // Check if already initialized
    {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        if rag_state.is_some() {
            return Ok("RAG system already initialized".to_string());
        }
    }
    
    // Initialize new system
    match EnhancedRagSystem::new(&app_handle).await {
        Ok(system) => {
            let mut rag_state = state.0.lock().map_err(|e| e.to_string())?;
            *rag_state = Some(system);
            Ok("RAG system initialized successfully".to_string())
        }
        Err(e) => Err(format!("Failed to initialize RAG system: {}", e))
    }
}

#[tauri::command]
pub async fn upload_document(
    fileName: String,
    fileContent: Vec<u8>,
    fileType: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<EnhancedDocument, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.upload_document(fileName, fileContent, fileType)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_documents(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<EnhancedDocument>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.get_all_documents()
                .map_err(|e| e.to_string())
        }
        None => Err(" RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn delete_document(
    documentId: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.delete_document(&documentId)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(format!("Document {} deleted successfully", documentId))
}

#[tauri::command]
pub async fn search_documents(
    query: String,
    contextDocumentIds: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<EnhancedDocumentChunk>, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.search_documents(&query, contextDocumentIds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_embeddings(
    documentId: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.generate_embeddings(&documentId)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_embedding_cache(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.clear_embedding_cache()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_rag_settings(
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
        None => Err(" RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_rag_settings(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<EnhancedRagSettings, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            Ok(system.get_settings())
        }
        None => Err(" RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_storage_stats(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            system.get_storage_stats()
                .map_err(|e| e.to_string())
        }
        None => Err(" RAG system not initialized".to_string())
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
        None => Err(" RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn check_document_duplicate(
    fileName: String,
    fileContent: Vec<u8>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    use sha2::{Sha256, Digest};
    
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    // Calculate content hash
    let mut hasher = Sha256::new();
    hasher.update(&fileContent);
    hasher.update(fileName.as_bytes());
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
    documentIds: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, String>, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.get_embedding_status_for_documents(&documentIds)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ensure_documents_ready_for_search(
    documentIds: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, String>, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.ensure_documents_ready_for_search(&documentIds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_embeddings_for_selection(
    documentIds: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<String, String> {
    let system = {
        let rag_state = state.0.lock().map_err(|e| e.to_string())?;
        match &*rag_state {
            Some(sys) => Ok(sys.clone()),
            None => Err("RAG system not initialized".to_string())
        }
    }?;
    
    system.generate_embeddings_for_selection(&documentIds)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_rag_file_upload(
    fileName: String,
    fileSize: usize,
    fileType: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            let settings = system.get_settings();
            let mut validation = HashMap::new();
            
            let file_size_mb = fileSize as f64 / (1024.0 * 1024.0);
            let size_valid = file_size_mb <= settings.max_document_size_mb;
            
            // Check supported file types
            let supported_types = vec!["text/plain", "application/pdf", "text/markdown", 
                                     "application/msword", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"];
            let type_valid = supported_types.iter().any(|&t| fileType.contains(t)) || fileType.starts_with("text/");
            
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
                    format!("File type '{}' is not supported", fileType)
                ));
            }
            
            Ok(validation)
        }
        None => Err(" RAG system not initialized".to_string())
    }
}