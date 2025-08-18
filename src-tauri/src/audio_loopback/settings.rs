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
    
    // println!("💾 Audio settings saved"); // Commented out: Audio loopback is working, reducing console noise for debugging focus
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
    
    // println!("📂 Audio settings loaded"); // Commented out: Audio loopback is working, reducing console noise for debugging focus
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
    
    // println!("💾 General settings saved"); // Commented out: Audio loopback is working, reducing console noise for debugging focus
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
    
    // println!("📂 General settings loaded"); // Commented out: Audio loopback is working, reducing console noise for debugging focus
    Ok(Some(settings))
}