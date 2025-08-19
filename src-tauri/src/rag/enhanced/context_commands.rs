use tauri::State;
use std::collections::HashMap;
use serde_json::Value;

use super::commands::EnhancedRagSystemState;
use crate::rag::services::{ContextSuggestion, RankedDocument, RelatedDocument, FileChangeEvent};

#[tauri::command]
pub async fn search_context_documents(
    query: String,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<RankedDocument>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            // Use the context engine from the system (would need to be added to EnhancedRagSystem)
            // For now, simulate the response
            let documents = system.get_all_documents().map_err(|e| e.to_string())?;
            
            let mut ranked_docs = Vec::new();
            for doc in documents {
                if doc.content.to_lowercase().contains(&query.to_lowercase()) ||
                   doc.file_name.to_lowercase().contains(&query.to_lowercase()) {
                    
                    let relevance_score = if doc.file_name.to_lowercase().contains(&query.to_lowercase()) {
                        0.9
                    } else {
                        0.7
                    };
                    
                    let mut rank_factors = HashMap::new();
                    rank_factors.insert("semantic_relevance".to_string(), relevance_score);
                    rank_factors.insert("usage_frequency".to_string(), doc.access_count as f32 / 100.0);
                    
                    ranked_docs.push(RankedDocument {
                        document: doc,
                        relevance_score,
                        rank_factors,
                    });
                }
            }
            
            // Sort by relevance
            ranked_docs.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
            
            Ok(ranked_docs.into_iter().take(20).collect())
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_context_suggestions(
    conversation_history: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<ContextSuggestion>, String> {
    let _rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    // Simple mock implementation - extract keywords and suggest relevant docs
    let combined_text = conversation_history.join(" ");
    let keywords: Vec<&str> = combined_text.split_whitespace()
        .filter(|word| word.len() > 4)
        .take(5)
        .collect();
    
    let mut suggestions = Vec::new();
    for (i, keyword) in keywords.iter().enumerate() {
        suggestions.push(ContextSuggestion {
            document_id: format!("doc_{}", i),
            document_name: format!("Document about {}", keyword),
            relevance_score: 0.8 - (i as f32 * 0.1),
            reason: format!("Contains keyword: {}", keyword),
            preview: format!("This document discusses {} in detail...", keyword),
        });
    }
    
    Ok(suggestions)
}

#[tauri::command]
pub async fn get_related_documents(
    document_ids: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<RelatedDocument>, String> {
    let rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    match &*rag_state {
        Some(system) => {
            let all_documents = system.get_all_documents().map_err(|e| e.to_string())?;
            let mut related = Vec::new();
            
            // Simple similarity based on file type and keywords
            for target_id in &document_ids {
                if let Some(target_doc) = all_documents.iter().find(|d| d.id == *target_id) {
                    for doc in &all_documents {
                        if doc.id != *target_id && doc.file_type == target_doc.file_type {
                            related.push(RelatedDocument {
                                document_id: doc.id.clone(),
                                document_name: doc.file_name.clone(),
                                relationship_type: "similar_type".to_string(),
                                similarity_score: 0.6,
                            });
                        }
                    }
                }
            }
            
            // Remove duplicates and limit results
            related.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
            related.dedup_by(|a, b| a.document_id == b.document_id);
            
            Ok(related.into_iter().take(10).collect())
        }
        None => Err("RAG system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn analyze_conversation_context(
    messages: Vec<String>,
    state: State<'_, EnhancedRagSystemState>,
) -> Result<HashMap<String, Value>, String> {
    let _rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    let combined_text = messages.join(" ");
    let mut context = HashMap::new();
    
    // Extract topics (simple word frequency)
    let words: Vec<&str> = combined_text.split_whitespace().collect();
    let mut word_count = HashMap::new();
    for word in &words {
        let lowercase_word = word.to_lowercase();
        let clean_word = lowercase_word.trim_matches(|c: char| !c.is_alphanumeric());
        if clean_word.len() > 4 {
            *word_count.entry(clean_word.to_string()).or_insert(0) += 1;
        }
    }
    
    let mut topics: Vec<_> = word_count.into_iter().collect();
    topics.sort_by(|a, b| b.1.cmp(&a.1));
    let top_topics: Vec<String> = topics.into_iter().take(5).map(|(word, _)| word).collect();
    
    // Simple intent detection
    let intent = if combined_text.contains("how") || combined_text.contains("what") {
        "question"
    } else if combined_text.contains("implement") || combined_text.contains("create") {
        "implementation"
    } else if combined_text.contains("error") || combined_text.contains("problem") {
        "troubleshooting"
    } else {
        "general"
    };
    
    context.insert("topics".to_string(), serde_json::json!(top_topics));
    context.insert("intent".to_string(), serde_json::json!(intent));
    context.insert("message_count".to_string(), serde_json::json!(messages.len()));
    context.insert("total_words".to_string(), serde_json::json!(words.len()));
    
    Ok(context)
}

#[tauri::command]
pub async fn scan_file_changes(
    state: State<'_, EnhancedRagSystemState>,
) -> Result<Vec<FileChangeEvent>, String> {
    let _rag_state = state.0.lock().map_err(|e| e.to_string())?;
    
    // Mock implementation - in production, this would use the file watcher
    let events = vec![
        // No changes for now
    ];
    
    Ok(events)
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