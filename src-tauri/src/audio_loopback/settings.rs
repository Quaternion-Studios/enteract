// src-tauri/src/audio_loopback/settings.rs
use crate::audio_loopback::types::AudioDeviceSettings;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use serde_json;

fn get_settings_path() -> anyhow::Result<PathBuf> {
    let app_data = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let app_dir = app_data.join("enteract");
    
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)?;
    }
    
    Ok(app_dir.join("audio_settings.json"))
}

fn get_general_settings_path() -> anyhow::Result<PathBuf> {
    let app_data = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let app_dir = app_data.join("enteract");
    
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)?;
    }
    
    Ok(app_dir.join("general_settings.json"))
}

#[tauri::command]
pub async fn save_audio_settings(settings: AudioDeviceSettings) -> Result<(), String> {
    let settings_path = get_settings_path()
        .map_err(|e| format!("Failed to get settings path: {}", e))?;
    
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    fs::write(settings_path, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    
    println!("💾 Audio settings saved");
    Ok(())
}

#[tauri::command]
pub async fn load_audio_settings() -> Result<Option<AudioDeviceSettings>, String> {
    let settings_path = get_settings_path()
        .map_err(|e| format!("Failed to get settings path: {}", e))?;
    
    if !settings_path.exists() {
        return Ok(None);
    }
    
    let json = fs::read_to_string(settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    let settings: AudioDeviceSettings = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse settings: {}", e))?;
    
    println!("📂 Audio settings loaded");
    Ok(Some(settings))
}

#[tauri::command]
pub async fn save_general_settings(settings: HashMap<String, serde_json::Value>) -> Result<(), String> {
    // Load existing settings to compare
    let existing_settings = load_general_settings().await.unwrap_or(None);
    
    let settings_path = get_general_settings_path()
        .map_err(|e| format!("Failed to get settings path: {}", e))?;
    
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    fs::write(settings_path, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    
    println!("💾 General settings saved");
    
    // Check if loopback whisper model changed
    if let Some(existing) = existing_settings {
        let old_model = existing.get("loopbackWhisperModel")
            .and_then(|v| v.as_str())
            .unwrap_or("small");
        let new_model = settings.get("loopbackWhisperModel")
            .and_then(|v| v.as_str())
            .unwrap_or("small");
        
        if old_model != new_model {
            println!("🔄 Loopback model changed from '{}' to '{}', reloading...", old_model, new_model);
            
            // Reload the whisper model with the new setting
            match crate::speech::reload_whisper_model_for_loopback(new_model.to_string()).await {
                Ok(result) => {
                    println!("✅ Successfully reloaded model: {}", result);
                },
                Err(e) => {
                    println!("❌ Failed to reload model: {}", e);
                    // Don't fail the settings save if model reload fails
                }
            }
        }
    } else {
        // First time saving settings, check if we should load a model
        if let Some(model) = settings.get("loopbackWhisperModel").and_then(|v| v.as_str()) {
            println!("🔄 Initial loopback model setting: '{}', preloading...", model);
            match crate::speech::reload_whisper_model_for_loopback(model.to_string()).await {
                Ok(result) => {
                    println!("✅ Successfully preloaded model: {}", result);
                },
                Err(e) => {
                    println!("❌ Failed to preload model: {}", e);
                }
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
pub async fn load_general_settings() -> Result<Option<HashMap<String, serde_json::Value>>, String> {
    let settings_path = get_general_settings_path()
        .map_err(|e| format!("Failed to get settings path: {}", e))?;
    
    if !settings_path.exists() {
        return Ok(None);
    }
    
    let json = fs::read_to_string(settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    let settings: HashMap<String, serde_json::Value> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse settings: {}", e))?;
    
    println!("📂 General settings loaded");
    Ok(Some(settings))
}