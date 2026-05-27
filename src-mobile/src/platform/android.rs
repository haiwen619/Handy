//! Android platform implementations. Stubs in this commit; real impls in Task 8.
#![cfg(target_os = "android")]

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use handy_platform::{AppStorage, AudioCapture, AudioConfig, AudioFrame, OutputMode, TextOutput};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

pub struct AndroidAudioCapture;
pub struct AndroidTextOutput;
pub struct AndroidStorage {
    models: PathBuf,
    db: PathBuf,
    settings: PathBuf,
    cache: PathBuf,
}

#[async_trait]
impl AudioCapture for AndroidAudioCapture {
    async fn start(&mut self, _config: AudioConfig) -> Result<mpsc::Receiver<AudioFrame>> {
        Err(anyhow!("not yet implemented"))
    }
    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }
    fn is_capturing(&self) -> bool {
        false
    }
}

#[async_trait]
impl TextOutput for AndroidTextOutput {
    async fn deliver(&self, _text: &str, _mode: OutputMode) -> Result<()> {
        Err(anyhow!("not yet implemented"))
    }
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
    Ok(AndroidAudioCapture)
}
pub fn new_text_output(_app: &AppHandle) -> Result<AndroidTextOutput> {
    Ok(AndroidTextOutput)
}
pub fn new_storage(app: &AppHandle) -> Result<AndroidStorage> {
    let base = app.path().app_data_dir()?;
    Ok(AndroidStorage {
        models: base.join("models"),
        db: base.join("handy.db"),
        settings: base.join("settings.json"),
        cache: base.join("cache"),
    })
}
