use crate::error::to_string;
use crate::state::AppState;
use handy_core::text::filter_transcription_output;
use handy_platform::AudioConfig;
use tauri::State;
use tokio::task;
use transcribe_rs::whisper_cpp::WhisperEngine;
use transcribe_rs::{SpeechModel, TranscribeOptions};

// TODO: deduplicate with commands::model::DEFAULT_MODEL_FILENAME
const DEFAULT_MODEL_FILENAME: &str = "ggml-tiny-q5_1.bin";

#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut audio = state.audio.lock().await;
    let rx = audio
        .start(AudioConfig::default())
        .await
        .map_err(to_string)?;
    drop(audio);
    *state.recording_rx.lock().await = Some(rx);
    Ok(())
}

#[tauri::command]
pub async fn cancel_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut audio = state.audio.lock().await;
    audio.stop().await.map_err(to_string)?;
    drop(audio);
    *state.recording_rx.lock().await = None;
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    // 1) Stop capture.
    {
        let mut audio = state.audio.lock().await;
        audio.stop().await.map_err(to_string)?;
    }

    // 2) Take the Receiver out of state and drain any buffered frames.
    let mut rx = state
        .recording_rx
        .lock()
        .await
        .take()
        .ok_or_else(|| "no active recording".to_string())?;

    let mut samples: Vec<f32> = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        samples.extend_from_slice(&frame.samples);
    }
    if samples.is_empty() {
        return Ok(String::new());
    }

    // 3) Lazy-load Whisper engine on first transcription.
    let model_path = state.storage.models_dir().join(DEFAULT_MODEL_FILENAME);
    {
        let mut engine = state.engine.lock().await;
        if engine.is_none() {
            let p = model_path.clone();
            let loaded = task::spawn_blocking(move || WhisperEngine::load(&p))
                .await
                .map_err(to_string)?
                .map_err(to_string)?;
            *engine = Some(loaded);
        }
    }

    // 4) Run transcription on the blocking pool (whisper.cpp is CPU-bound + blocking).
    let engine_arc = state.engine.clone();
    let raw = task::spawn_blocking(move || -> Result<String, transcribe_rs::TranscribeError> {
        let mut guard = engine_arc.blocking_lock();
        let model = guard.as_mut().expect("engine loaded above");
        let result = model.transcribe(&samples, &TranscribeOptions::default())?;
        Ok(result.text)
    })
    .await
    .map_err(to_string)?
    .map_err(to_string)?;

    Ok(filter_transcription_output(&raw, "en", &None))
}
