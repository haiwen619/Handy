use handy_platform::{AppStorage, AudioCapture, TextOutput};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use transcribe_rs::whisper_cpp::WhisperEngine;

use crate::event_sink::TauriEventSink;

/// Captured audio samples + the drainer task that fills them. While a recording
/// is in progress, `drainer` is the JoinHandle of a tokio task that continuously
/// reads frames from the platform AudioCapture's mpsc receiver and appends them
/// to `samples`. The drainer exits when the sender side is dropped (i.e. when
/// `AudioCapture::stop` closes the stream).
///
/// Why we need this: the Oboe audio callback can only `try_send` (it's on the
/// realtime audio thread and MUST NOT block). If nothing drains the receiver,
/// the bounded mpsc fills up within ~100ms and every subsequent sample is
/// dropped. Previous code only drained inside stop_recording, so anything past
/// the first ~64 callbacks was lost.
pub struct RecordingBuffer {
    pub samples: Arc<Mutex<Vec<f32>>>,
    pub drainer: JoinHandle<()>,
}

pub struct AppState {
    pub storage: Arc<dyn AppStorage>,
    pub audio: Arc<Mutex<dyn AudioCapture>>,
    pub text_output: Arc<dyn TextOutput>,
    pub sink: Arc<TauriEventSink>,
    pub engine: Arc<Mutex<Option<WhisperEngine>>>,
    pub recording: Arc<Mutex<Option<RecordingBuffer>>>,
}
