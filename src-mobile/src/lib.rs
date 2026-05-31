#![allow(dead_code)]

mod commands;
mod download;
mod error;
mod event_sink;
mod platform;
mod state;

use handy_platform::{AppStorage, AudioCapture, TextOutput};
use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let storage: Arc<dyn AppStorage> = Arc::new(platform::new_storage(&handle)?);
            let audio: Arc<Mutex<dyn AudioCapture>> =
                Arc::new(Mutex::new(platform::new_audio_capture()?));
            let text_output: Arc<dyn TextOutput> = Arc::new(platform::new_text_output(&handle)?);
            let sink = Arc::new(event_sink::TauriEventSink::new(handle.clone()));
            let state = AppState {
                storage,
                audio,
                text_output,
                sink,
                engine: Arc::new(Mutex::new(None)),
                recording: Arc::new(Mutex::new(None)),
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::transcription::start_recording,
            commands::transcription::stop_recording,
            commands::transcription::cancel_recording,
            commands::model::model_status,
            commands::model::download_default_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
