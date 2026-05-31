use handy_platform::EventSink;
use tauri::{AppHandle, Emitter};

pub struct TauriEventSink {
    handle: AppHandle,
}

impl TauriEventSink {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl EventSink for TauriEventSink {
    fn emit_json(&self, event_name: &str, payload: serde_json::Value) {
        if let Err(e) = self.handle.emit(event_name, payload) {
            log::warn!("emit_json({event_name}) failed: {e}");
        }
    }
}
