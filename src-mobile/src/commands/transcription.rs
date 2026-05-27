use crate::error::to_string;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn start_recording(_state: State<'_, AppState>) -> Result<(), String> {
    Err(to_string("start_recording not yet implemented"))
}

#[tauri::command]
pub async fn stop_recording(_state: State<'_, AppState>) -> Result<String, String> {
    Err(to_string("stop_recording not yet implemented"))
}

#[tauri::command]
pub async fn cancel_recording(_state: State<'_, AppState>) -> Result<(), String> {
    Err(to_string("cancel_recording not yet implemented"))
}
