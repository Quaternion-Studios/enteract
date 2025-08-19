use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::rag::enhanced::system::EnhancedRagSystem;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileChangeEvent {
    pub file_path: String,
    pub event_type: FileEventType,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FileEventType {
    Created,
    Modified,
    Deleted,
    Moved { from: String, to: String },
}

pub struct FileWatcher {
    rag_system: Arc<Mutex<Option<EnhancedRagSystem>>>,
    watched_files: Arc<Mutex<HashMap<String, String>>>, // file_path -> document_id
}

impl FileWatcher {
    pub fn new(rag_system: Arc<Mutex<Option<EnhancedRagSystem>>>) -> Self {
        Self {
            rag_system,
            watched_files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a file to be watched for changes
    pub async fn watch_file(&self, file_path: &str, document_id: &str) -> Result<()> {
        let mut watched_files = self.watched_files.lock().await;
        watched_files.insert(file_path.to_string(), document_id.to_string());
        Ok(())
    }

    /// Unregister a file from watching
    pub async fn unwatch_file(&self, file_path: &str) -> Result<()> {
        let mut watched_files = self.watched_files.lock().await;
        watched_files.remove(file_path);
        Ok(())
    }

    /// Check if a file exists and handle accordingly
    pub async fn check_file_status(&self, file_path: &str) -> Result<()> {
        let path = Path::new(file_path);
        let watched_files = self.watched_files.lock().await;
        
        if let Some(document_id) = watched_files.get(file_path).cloned() {
            if !path.exists() {
                // File was deleted, clean up the document
                drop(watched_files); // Release the lock
                self.handle_file_deleted(file_path, &document_id).await?;
            } else {
                // Check if file was modified
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        // Could implement modification time tracking here
                        self.handle_file_modified(file_path, &document_id).await?;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Handle file deletion event
    async fn handle_file_deleted(&self, file_path: &str, document_id: &str) -> Result<()> {
        println!("File deleted: {}, cleaning up document: {}", file_path, document_id);
        
        let rag_system_guard = self.rag_system.lock().await;
        if let Some(rag_system) = rag_system_guard.as_ref() {
            // Delete the document and its embeddings
            match rag_system.delete_document(document_id).await {
                Ok(_) => {
                    println!("Successfully cleaned up document {} after file deletion", document_id);
                    
                    // Remove from watched files
                    drop(rag_system_guard);
                    let mut watched_files = self.watched_files.lock().await;
                    watched_files.remove(file_path);
                }
                Err(e) => {
                    eprintln!("Failed to clean up document {} after file deletion: {}", document_id, e);
                }
            }
        }
        
        Ok(())
    }

    /// Handle file modification event
    async fn handle_file_modified(&self, file_path: &str, document_id: &str) -> Result<()> {
        println!("File modified: {}, document: {}", file_path, document_id);
        
        let rag_system_guard = self.rag_system.lock().await;
        if let Some(rag_system) = rag_system_guard.as_ref() {
            // Re-read the file and update the document
            match std::fs::read(file_path) {
                Ok(content) => {
                    let file_name = Path::new(file_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    
                    // Determine file type
                    let file_type = self.determine_file_type(file_path);
                    
                    // Delete old document
                    if let Err(e) = rag_system.delete_document(document_id).await {
                        eprintln!("Failed to delete old document during update: {}", e);
                    }
                    
                    // Upload new version
                    match rag_system.upload_document(file_name, content, file_type).await {
                        Ok(new_doc) => {
                            println!("Successfully updated document {} after file modification", new_doc.id);
                            
                            // Update the watched files mapping with new document ID
                            drop(rag_system_guard);
                            let mut watched_files = self.watched_files.lock().await;
                            watched_files.insert(file_path.to_string(), new_doc.id);
                        }
                        Err(e) => {
                            eprintln!("Failed to upload updated document: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read modified file {}: {}", file_path, e);
                }
            }
        }
        
        Ok(())
    }

    /// Determine file type based on extension
    fn determine_file_type(&self, file_path: &str) -> String {
        let path = Path::new(file_path);
        if let Some(extension) = path.extension() {
            match extension.to_string_lossy().to_lowercase().as_str() {
                "txt" => "text/plain".to_string(),
                "md" => "text/markdown".to_string(),
                "pdf" => "application/pdf".to_string(),
                "doc" => "application/msword".to_string(),
                "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                "rtf" => "application/rtf".to_string(),
                _ => "text/plain".to_string(),
            }
        } else {
            "text/plain".to_string()
        }
    }

    /// Scan all watched files for changes
    pub async fn scan_watched_files(&self) -> Result<Vec<FileChangeEvent>> {
        let mut events = Vec::new();
        let watched_files_copy = {
            let watched_files = self.watched_files.lock().await;
            watched_files.clone()
        };
        
        for (file_path, document_id) in watched_files_copy.iter() {
            let path = Path::new(file_path);
            
            if !path.exists() {
                events.push(FileChangeEvent {
                    file_path: file_path.clone(),
                    event_type: FileEventType::Deleted,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
                
                // Handle the deletion
                self.handle_file_deleted(file_path, document_id).await?;
                // Note: Removed recursive call to avoid infinite recursion
            }
        }
        
        Ok(events)
    }

    /// Get list of currently watched files
    pub async fn get_watched_files(&self) -> Result<HashMap<String, String>> {
        let watched_files = self.watched_files.lock().await;
        Ok(watched_files.clone())
    }

    /// Clean up orphaned documents (documents whose files no longer exist)
    pub async fn cleanup_orphaned_documents(&self) -> Result<Vec<String>> {
        let mut cleaned_up = Vec::new();
        
        let rag_system_guard = self.rag_system.lock().await;
        if let Some(rag_system) = rag_system_guard.as_ref() {
            let documents = rag_system.get_all_documents()?;
            
            for doc in documents {
                if !doc.file_path.is_empty() && !Path::new(&doc.file_path).exists() {
                    match rag_system.delete_document(&doc.id).await {
                        Ok(_) => {
                            cleaned_up.push(doc.id.clone());
                            println!("Cleaned up orphaned document: {} ({})", doc.file_name, doc.id);
                        }
                        Err(e) => {
                            eprintln!("Failed to clean up orphaned document {}: {}", doc.id, e);
                        }
                    }
                }
            }
        }
        
        Ok(cleaned_up)
    }
}