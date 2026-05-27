use crate::error::to_string;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ModelStatus {
    pub downloaded: bool,
    pub loaded: bool,
    pub bytes: u64,
}

#[tauri::command]
pub async fn model_status(_state: State<'_, AppState>) -> Result<ModelStatus, String> {
    Err(to_string("not implemented"))
}

#[tauri::command]
pub async fn download_default_model(_state: State<'_, AppState>) -> Result<(), String> {
    Err(to_string("not implemented"))
}
