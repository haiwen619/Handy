use handy_platform::{AppStorage, AudioCapture, AudioFrame, TextOutput};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use transcribe_rs::whisper_cpp::WhisperEngine;

use crate::event_sink::TauriEventSink;

pub struct AppState {
    pub storage: Arc<dyn AppStorage>,
    pub audio: Arc<Mutex<dyn AudioCapture>>,
    pub text_output: Arc<dyn TextOutput>,
    pub sink: Arc<TauriEventSink>,
    pub engine: Arc<Mutex<Option<WhisperEngine>>>,
    pub recording_rx: Arc<Mutex<Option<mpsc::Receiver<AudioFrame>>>>,
}
