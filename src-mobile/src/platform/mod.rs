use anyhow::Result;
use handy_platform::{AppStorage, AudioCapture, TextOutput};
use tauri::AppHandle;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::{new_audio_capture, new_storage, new_text_output};

#[cfg(not(target_os = "android"))]
mod desktop_stub;
#[cfg(not(target_os = "android"))]
pub use desktop_stub::{new_audio_capture, new_storage, new_text_output};
