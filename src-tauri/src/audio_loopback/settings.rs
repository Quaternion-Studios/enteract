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
    let settings_path = get_general_settings_path()
        .map_err(|e| format!("Failed to get settings path: {}", e))?;
    
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    fs::write(settings_path, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    
    println!("💾 General settings saved");
    Ok(())
}

#[tauri::command]
pub async fn load_general_settings() -> Result<Option<HashMap<String, serde_json::Value>>, String> {
    let settings_path = get_general_settings_path()
        .map_err(|e| format!("Failed to get settings path: {}", e))?;
    
    if !settings_path.exists() {
        return Ok(None);
    }
    
    let json = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    let mut settings: HashMap<String, serde_json::Value> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse settings: {}", e))?;
    
    // Migration logic: convert existing 'small' loopback setting to 'tiny'
    let needs_migration = if let Some(loopback_model) = settings.get("loopbackWhisperModel") {
        loopback_model.as_str() == Some("small")
    } else {
        false
    };
    
    if needs_migration {
        settings.insert("loopbackWhisperModel".to_string(), serde_json::Value::String("tiny".to_string()));
        
        // Save the migrated settings back to disk
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| format!("Failed to serialize migrated settings: {}", e))?;
        
        fs::write(&settings_path, json)
            .map_err(|e| format!("Failed to write migrated settings file: {}", e))?;
        
        println!("🔄 Migrated loopback Whisper model from 'small' to 'tiny' in backend");
    }
    
    println!("📂 General settings loaded");
    Ok(Some(settings))
}