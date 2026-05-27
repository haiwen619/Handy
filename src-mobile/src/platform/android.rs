//! Android platform implementations: Oboe-based audio capture, clipboard text
//! output, and app-data-dir-rooted storage.
#![cfg(target_os = "android")]

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use handy_platform::{AppStorage, AudioCapture, AudioConfig, AudioFrame, OutputMode, TextOutput};
use oboe::{
    AudioInputCallback, AudioInputStreamSafe, AudioStreamAsync, AudioStreamBuilder,
    DataCallbackResult, Input, InputPreset, Mono, PerformanceMode,
    SampleRateConversionQuality, SharingMode,
};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

pub struct OboeCallback {
    tx: mpsc::Sender<AudioFrame>,
}

impl AudioInputCallback for OboeCallback {
    type FrameType = (f32, Mono);

    fn on_audio_ready(
        &mut self,
        _audio_stream: &mut dyn AudioInputStreamSafe,
        audio_data: &[f32],
    ) -> DataCallbackResult {
        // Best-effort delivery. The audio thread MUST NOT block, so we drop
        // frames if the consumer is slow (channel full / closed).
        let _ = self.tx.try_send(AudioFrame {
            samples: audio_data.to_vec(),
            timestamp_ms: 0,
        });
        DataCallbackResult::Continue
    }
}

pub struct AndroidAudioCapture {
    stream: Option<AudioStreamAsync<Input, OboeCallback>>,
}

impl AndroidAudioCapture {
    pub fn new() -> Self {
        Self { stream: None }
    }
}

#[async_trait]
impl AudioCapture for AndroidAudioCapture {
    async fn start(&mut self, _config: AudioConfig) -> Result<mpsc::Receiver<AudioFrame>> {
        if self.stream.is_some() {
            anyhow::bail!("AndroidAudioCapture already running");
        }
        let (tx, rx) = mpsc::channel::<AudioFrame>(64);

        let stream = AudioStreamBuilder::default()
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sharing_mode(SharingMode::Shared)
            .set_format::<f32>()
            .set_channel_count::<Mono>()
            .set_sample_rate(16_000)
            .set_sample_rate_conversion_quality(SampleRateConversionQuality::Medium)
            .set_input_preset(InputPreset::VoiceRecognition)
            .set_input()
            .set_callback(OboeCallback { tx })
            .open_stream()
            .map_err(|e| anyhow!("oboe open_stream: {e:?}"))?;

        let mut stream = stream;
        stream
            .start()
            .map_err(|e| anyhow!("oboe start: {e:?}"))?;
        self.stream = Some(stream);
        Ok(rx)
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(mut s) = self.stream.take() {
            s.stop().map_err(|e| anyhow!("oboe stop: {e:?}"))?;
        }
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.stream.is_some()
    }
}

pub struct AndroidTextOutput {
    handle: AppHandle,
}

#[async_trait]
impl TextOutput for AndroidTextOutput {
    async fn deliver(&self, text: &str, mode: OutputMode) -> Result<()> {
        match mode {
            OutputMode::Clipboard => {
                use tauri_plugin_clipboard_manager::ClipboardExt;
                self.handle
                    .clipboard()
                    .write_text(text.to_string())
                    .map_err(|e| anyhow!("clipboard write: {e}"))?;
                Ok(())
            }
            OutputMode::Typed | OutputMode::ImeCommit => {
                Err(anyhow!("OutputMode {mode:?} not supported in phase 2a"))
            }
        }
    }
}

pub struct AndroidStorage {
    models: PathBuf,
    db: PathBuf,
    settings: PathBuf,
    cache: PathBuf,
}

impl AppStorage for AndroidStorage {
    fn models_dir(&self) -> PathBuf {
        self.models.clone()
    }
    fn db_path(&self) -> PathBuf {
        self.db.clone()
    }
    fn settings_path(&self) -> PathBuf {
        self.settings.clone()
    }
    fn cache_dir(&self) -> PathBuf {
        self.cache.clone()
    }
}

pub fn new_audio_capture() -> Result<AndroidAudioCapture> {
    Ok(AndroidAudioCapture::new())
}

pub fn new_text_output(app: &AppHandle) -> Result<AndroidTextOutput> {
    Ok(AndroidTextOutput {
        handle: app.clone(),
    })
}

pub fn new_storage(app: &AppHandle) -> Result<AndroidStorage> {
    let base = app.path().app_data_dir()?;
    std::fs::create_dir_all(base.join("models"))?;
    std::fs::create_dir_all(base.join("cache"))?;
    Ok(AndroidStorage {
        models: base.join("models"),
        db: base.join("handy.db"),
        settings: base.join("settings.json"),
        cache: base.join("cache"),
    })
}
