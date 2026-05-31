use crate::error::to_string;
use crate::state::{AppState, RecordingBuffer};
use handy_core::text::filter_transcription_output;
use handy_platform::AudioConfig;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio::task;
use transcribe_rs::whisper_cpp::WhisperEngine;
use transcribe_rs::{SpeechModel, TranscribeOptions};

/// Emit a log line to the frontend `app/log` event AND to the rust log
/// facade in one call. The UI LogPanel listens to `app/log`. Failures to
/// emit are swallowed (UI may not be mounted yet / no listeners attached).
fn ui_log(app: &AppHandle, level: &str, msg: impl Into<String>) {
    let msg = msg.into();
    match level {
        "warn" => log::warn!("{msg}"),
        "error" => log::error!("{msg}"),
        _ => log::info!("{msg}"),
    }
    let ts_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let _ = app.emit(
        "app/log",
        json!({ "level": level, "msg": msg, "ts_ms": ts_ms }),
    );
}

// TODO: deduplicate with commands::model::DEFAULT_MODEL_FILENAME
const DEFAULT_MODEL_FILENAME: &str = "ggml-tiny-q5_1.bin";

const SAMPLE_RATE_HZ: usize = 16_000;
/// Below this duration we don't even bother calling whisper — it produces
/// noise / hallucinations on sub-second clips and the user almost certainly
/// just mistapped the button.
const MIN_USEFUL_SAMPLES: usize = SAMPLE_RATE_HZ / 4; // 0.25 s

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    ui_log(&app, "info", "start_recording: invoked");
    // Refuse to start a second recording while one is active. Otherwise the
    // old drainer keeps holding samples while the new one starts a fresh
    // buffer, and stop_recording would see the wrong one.
    {
        let guard = state.recording.lock().await;
        if guard.is_some() {
            ui_log(&app, "warn", "start_recording: already recording, ignoring");
            return Err("recording already in progress".into());
        }
    }

    let mut audio = state.audio.lock().await;
    let mut rx = audio
        .start(AudioConfig::default())
        .await
        .map_err(|e| {
            let s = to_string(e);
            ui_log(&app, "error", format!("audio.start failed: {s}"));
            s
        })?;
    drop(audio);
    ui_log(&app, "info", "audio capture started (Oboe stream open)");

    // Continuously drain frames into a shared Vec while recording. The drainer
    // exits when the audio stream is closed (sender dropped → rx.recv() returns
    // None). This is the critical fix for "transcription is always empty":
    // Oboe's audio callback can only `try_send`, so a bounded 64-frame mpsc
    // fills up within ~50ms if nothing reads from it.
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(SAMPLE_RATE_HZ * 8)));
    let samples_for_task = samples.clone();
    let app_for_task = app.clone();
    let drainer = tokio::spawn(async move {
        let mut frame_count: usize = 0;
        let mut total_samples: usize = 0;
        while let Some(frame) = rx.recv().await {
            total_samples += frame.samples.len();
            frame_count += 1;
            samples_for_task.lock().await.extend_from_slice(&frame.samples);
        }
        ui_log(
            &app_for_task,
            "info",
            format!(
                "audio drainer exited: {frame_count} frames, {total_samples} samples (~{:.2}s @ {SAMPLE_RATE_HZ}Hz)",
                total_samples as f32 / SAMPLE_RATE_HZ as f32,
            ),
        );
    });

    *state.recording.lock().await = Some(RecordingBuffer { samples, drainer });
    ui_log(&app, "info", "start_recording: drainer spawned, ready to record");
    Ok(())
}

#[tauri::command]
pub async fn cancel_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    ui_log(&app, "info", "cancel_recording: invoked");
    let mut audio = state.audio.lock().await;
    audio.stop().await.map_err(|e| {
        let s = to_string(e);
        ui_log(&app, "error", format!("audio.stop failed: {s}"));
        s
    })?;
    drop(audio);

    // Take the buffer so the drainer JoinHandle is dropped (which is harmless
    // — the task will finish on its own once the mpsc sender is dropped above).
    let _ = state.recording.lock().await.take();
    ui_log(&app, "info", "cancel_recording: stopped + cleared buffer");
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    ui_log(&app, "info", "stop_recording: invoked");
    // 1) Stop audio capture. This causes Oboe to drop its sender, which
    //    causes our drainer's `rx.recv().await` to return None and exit.
    {
        let mut audio = state.audio.lock().await;
        audio.stop().await.map_err(|e| {
            let s = to_string(e);
            ui_log(&app, "error", format!("audio.stop failed: {s}"));
            s
        })?;
    }

    // 2) Take the RecordingBuffer out of state.
    let buf = state
        .recording
        .lock()
        .await
        .take()
        .ok_or_else(|| {
            ui_log(&app, "warn", "stop_recording: no active recording");
            "no active recording".to_string()
        })?;

    // 3) Wait for the drainer to finish so we know we've consumed every frame
    //    Oboe handed to the mpsc. spawn_blocking-style join inside async.
    if let Err(e) = buf.drainer.await {
        ui_log(&app, "warn", format!("drainer join failed: {e}"));
    }

    let samples = Arc::try_unwrap(buf.samples)
        .map(|m| m.into_inner())
        .unwrap_or_else(|arc| arc.blocking_lock().clone());

    let dur_s = samples.len() as f32 / SAMPLE_RATE_HZ as f32;
    ui_log(
        &app,
        "info",
        format!("collected {} samples (~{:.2}s)", samples.len(), dur_s),
    );

    if samples.len() < MIN_USEFUL_SAMPLES {
        ui_log(
            &app,
            "warn",
            format!("clip too short ({dur_s:.2}s < 0.25s), skipping whisper"),
        );
        return Ok(String::new());
    }

    // 4) Lazy-load Whisper engine on first transcription.
    let model_path = state.storage.models_dir().join(DEFAULT_MODEL_FILENAME);
    {
        let mut engine = state.engine.lock().await;
        if engine.is_none() {
            ui_log(
                &app,
                "info",
                format!("loading whisper engine from {}", model_path.display()),
            );
            let p = model_path.clone();
            let loaded = task::spawn_blocking(move || WhisperEngine::load(&p))
                .await
                .map_err(|e| {
                    let s = to_string(e);
                    ui_log(&app, "error", format!("whisper load join error: {s}"));
                    s
                })?
                .map_err(|e| {
                    let s = to_string(e);
                    ui_log(&app, "error", format!("whisper load failed: {s}"));
                    s
                })?;
            *engine = Some(loaded);
            ui_log(&app, "info", "whisper engine loaded");
        }
    }

    // 5) Run transcription on the blocking pool (whisper.cpp is CPU-bound + blocking).
    ui_log(&app, "info", "transcribing…");
    let engine_arc = state.engine.clone();
    let raw = task::spawn_blocking(move || -> Result<String, transcribe_rs::TranscribeError> {
        let mut guard = engine_arc.blocking_lock();
        let model = guard.as_mut().expect("engine loaded above");
        let result = model.transcribe(&samples, &TranscribeOptions::default())?;
        Ok(result.text)
    })
    .await
    .map_err(|e| {
        let s = to_string(e);
        ui_log(&app, "error", format!("transcribe join error: {s}"));
        s
    })?
    .map_err(|e| {
        let s = to_string(e);
        ui_log(&app, "error", format!("transcribe failed: {s}"));
        s
    })?;

    ui_log(
        &app,
        "info",
        format!("whisper output ({} chars): {:?}", raw.len(), raw),
    );
    Ok(filter_transcription_output(&raw, "en", &None))
}
