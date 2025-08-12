// Stub implementation for eye tracking - functionality removed but API preserved
// to avoid breaking the UI

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLGazeData {
    pub x: f64,
    pub y: f64,
    pub confidence: f32,
    pub left_eye_landmarks: Vec<(f32, f32)>,
    pub right_eye_landmarks: Vec<(f32, f32)>,
    pub head_pose: HeadPose,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadPose {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MLTrackingStats {
    pub total_frames_processed: u32,
    pub average_confidence: f32,
    pub frames_per_second: f32,
    pub tracking_duration: f64,
    pub last_update: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLEyeTrackingConfig {
    pub camera_id: i32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub smoothing_window: u32,
    pub confidence_threshold: f32,
    pub kalman_process_noise: f32,
    pub kalman_measurement_noise: f32,
    pub adaptive_smoothing: bool,
}

// Stub command implementations - return disabled status
#[tauri::command]
pub async fn start_ml_eye_tracking(_config: MLEyeTrackingConfig) -> Result<String, String> {
    Err("Eye tracking functionality is currently disabled".to_string())
}

#[tauri::command]
pub async fn stop_ml_eye_tracking() -> Result<String, String> {
    Ok("Eye tracking is not active".to_string())
}

#[tauri::command]
pub async fn get_ml_gaze_data() -> Result<Option<MLGazeData>, String> {
    Ok(None)
}

#[tauri::command]
pub async fn calibrate_ml_eye_tracking() -> Result<String, String> {
    Err("Eye tracking functionality is currently disabled".to_string())
}

#[tauri::command]
pub async fn get_ml_tracking_stats() -> Result<MLTrackingStats, String> {
    Ok(MLTrackingStats {
        total_frames_processed: 0,
        average_confidence: 0.0,
        frames_per_second: 0.0,
        tracking_duration: 0.0,
        last_update: 0,
    })
}

#[tauri::command]
pub async fn pause_ml_tracking() -> Result<String, String> {
    Ok("Eye tracking is not active".to_string())
}

#[tauri::command]
pub async fn resume_ml_tracking() -> Result<String, String> {
    Ok("Eye tracking is not active".to_string())
}

#[tauri::command]
pub async fn detect_window_drag() -> Result<bool, String> {
    Ok(false)
}