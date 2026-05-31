use crate::download::download_with_progress;
use crate::error::to_string;
use crate::state::AppState;
use handy_platform::EventSink;
use serde::Serialize;
use tauri::State;
use tokio::fs;

const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_1.bin";
const DEFAULT_MODEL_FILENAME: &str = "ggml-tiny-q5_1.bin";

#[derive(Serialize)]
pub struct ModelStatus {
    pub downloaded: bool,
    pub loaded: bool,
    pub bytes: u64,
}

#[tauri::command]
pub async fn model_status(state: State<'_, AppState>) -> Result<ModelStatus, String> {
    let path = state.storage.models_dir().join(DEFAULT_MODEL_FILENAME);
    let meta = fs::metadata(&path).await.ok();
    Ok(ModelStatus {
        downloaded: meta.is_some(),
        loaded: state.engine.lock().await.is_some(),
        bytes: meta.map(|m| m.len()).unwrap_or(0),
    })
}

#[tauri::command]
pub async fn download_default_model(state: State<'_, AppState>) -> Result<(), String> {
    let dest = state.storage.models_dir().join(DEFAULT_MODEL_FILENAME);
    download_with_progress(
        DEFAULT_MODEL_URL,
        &dest,
        state.sink.as_ref(),
        "model/download/progress",
    )
    .await
    .map_err(to_string)?;

    state.sink.emit_json(
        "model/ready",
        serde_json::json!({ "name": DEFAULT_MODEL_FILENAME }),
    );
    Ok(())
}
