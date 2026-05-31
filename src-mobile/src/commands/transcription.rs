use crate::error::to_string;
use crate::state::{AppState, RecordingBuffer};
use handy_core::text::filter_transcription_output;
use handy_platform::AudioConfig;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tokio::task;
use transcribe_rs::whisper_cpp::WhisperEngine;
use transcribe_rs::{SpeechModel, TranscribeOptions};

// TODO: deduplicate with commands::model::DEFAULT_MODEL_FILENAME
const DEFAULT_MODEL_FILENAME: &str = "ggml-tiny-q5_1.bin";

const SAMPLE_RATE_HZ: usize = 16_000;
/// Below this duration we don't even bother calling whisper — it produces
/// noise / hallucinations on sub-second clips and the user almost certainly
/// just mistapped the button.
const MIN_USEFUL_SAMPLES: usize = SAMPLE_RATE_HZ / 4; // 0.25 s

#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    // Refuse to start a second recording while one is active. Otherwise the
    // old drainer keeps holding samples while the new one starts a fresh
    // buffer, and stop_recording would see the wrong one.
    {
        let guard = state.recording.lock().await;
        if guard.is_some() {
            return Err("recording already in progress".into());
        }
    }

    let mut audio = state.audio.lock().await;
    let mut rx = audio
        .start(AudioConfig::default())
        .await
        .map_err(to_string)?;
    drop(audio);

    // Continuously drain frames into a shared Vec while recording. The drainer
    // exits when the audio stream is closed (sender dropped → rx.recv() returns
    // None). This is the critical fix for "transcription is always empty":
    // Oboe's audio callback can only `try_send`, so a bounded 64-frame mpsc
    // fills up within ~50ms if nothing reads from it.
    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(SAMPLE_RATE_HZ * 8)));
    let samples_for_task = samples.clone();
    let drainer = tokio::spawn(async move {
        let mut frame_count: usize = 0;
        let mut total_samples: usize = 0;
        while let Some(frame) = rx.recv().await {
            total_samples += frame.samples.len();
            frame_count += 1;
            samples_for_task.lock().await.extend_from_slice(&frame.samples);
        }
        log::info!(
            "audio drainer exited: {frame_count} frames, {total_samples} samples \
             (~{:.2}s @ {SAMPLE_RATE_HZ}Hz)",
            total_samples as f32 / SAMPLE_RATE_HZ as f32,
        );
    });

    *state.recording.lock().await = Some(RecordingBuffer { samples, drainer });
    log::info!("start_recording: drainer spawned");
    Ok(())
}

#[tauri::command]
pub async fn cancel_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut audio = state.audio.lock().await;
    audio.stop().await.map_err(to_string)?;
    drop(audio);

    // Take the buffer so the drainer JoinHandle is dropped (which is harmless
    // — the task will finish on its own once the mpsc sender is dropped above).
    let _ = state.recording.lock().await.take();
    log::info!("cancel_recording: stopped + cleared buffer");
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<String, String> {
    // 1) Stop audio capture. This causes Oboe to drop its sender, which
    //    causes our drainer's `rx.recv().await` to return None and exit.
    {
        let mut audio = state.audio.lock().await;
        audio.stop().await.map_err(to_string)?;
    }

    // 2) Take the RecordingBuffer out of state.
    let buf = state
        .recording
        .lock()
        .await
        .take()
        .ok_or_else(|| "no active recording".to_string())?;

    // 3) Wait for the drainer to finish so we know we've consumed every frame
    //    Oboe handed to the mpsc. spawn_blocking-style join inside async.
    if let Err(e) = buf.drainer.await {
        log::warn!("drainer join failed: {e}");
    }

    let samples = Arc::try_unwrap(buf.samples)
        .map(|m| m.into_inner())
        .unwrap_or_else(|arc| arc.blocking_lock().clone());

    log::info!(
        "stop_recording: collected {} samples (~{:.2}s)",
        samples.len(),
        samples.len() as f32 / SAMPLE_RATE_HZ as f32,
    );

    if samples.len() < MIN_USEFUL_SAMPLES {
        return Ok(String::new());
    }

    // 4) Lazy-load Whisper engine on first transcription.
    let model_path = state.storage.models_dir().join(DEFAULT_MODEL_FILENAME);
    {
        let mut engine = state.engine.lock().await;
        if engine.is_none() {
            log::info!("loading whisper engine from {}", model_path.display());
            let p = model_path.clone();
            let loaded = task::spawn_blocking(move || WhisperEngine::load(&p))
                .await
                .map_err(to_string)?
                .map_err(to_string)?;
            *engine = Some(loaded);
            log::info!("whisper engine loaded");
        }
    }

    // 5) Run transcription on the blocking pool (whisper.cpp is CPU-bound + blocking).
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

    log::info!("whisper output ({} chars): {:?}", raw.len(), raw);
    Ok(filter_transcription_output(&raw, "en", &None))
}
