use handy_platform::{AppStorage, AudioCapture, TextOutput};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::event_sink::TauriEventSink;

// TODO: re-introduce real WhisperModel field in Task 5/6 once we settle the
// transcribe-rs import path (whisper_cpp::WhisperModel vs. similar). For now
// a unit placeholder keeps the scaffold compiling on host.
pub struct AppState {
    pub storage: Arc<dyn AppStorage>,
    pub audio: Arc<Mutex<dyn AudioCapture>>,
    pub text_output: Arc<dyn TextOutput>,
    pub sink: Arc<TauriEventSink>,
    pub engine: Arc<Mutex<Option<()>>>,
}
