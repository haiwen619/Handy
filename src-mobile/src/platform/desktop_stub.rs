//! Host (non-Android) stubs. `cargo check` on linux/macos/windows compiles
//! these; runtime panics are acceptable because handy-mobile is only meant
//! to be *run* on Android. The desktop CI job uses these to prove the
//! workspace as a whole still compiles.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use handy_platform::{AppStorage, AudioCapture, AudioConfig, AudioFrame, OutputMode, TextOutput};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

pub struct StubAudio;
pub struct StubText;
pub struct StubStorage {
    models: PathBuf,
    db: PathBuf,
    settings: PathBuf,
    cache: PathBuf,
}

#[async_trait]
impl AudioCapture for StubAudio {
    async fn start(&mut self, _config: AudioConfig) -> Result<mpsc::Receiver<AudioFrame>> {
        Err(anyhow!("audio capture not supported on host"))
    }
    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }
    fn is_capturing(&self) -> bool {
        false
    }
}

#[async_trait]
impl TextOutput for StubText {
    async fn deliver(&self, _text: &str, _mode: OutputMode) -> Result<()> {
        Err(anyhow!("text output not supported on host"))
    }
}

impl AppStorage for StubStorage {
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

pub fn new_audio_capture() -> Result<StubAudio> {
    Ok(StubAudio)
}

pub fn new_text_output(_app: &AppHandle) -> Result<StubText> {
    Ok(StubText)
}

pub fn new_storage(app: &AppHandle) -> Result<StubStorage> {
    let base = app.path().app_data_dir()?;
    Ok(StubStorage {
        models: base.join("models"),
        db: base.join("handy.db"),
        settings: base.join("settings.json"),
        cache: base.join("cache"),
    })
}
