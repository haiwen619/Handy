use anyhow::{Context, Result};
use futures_util::StreamExt;
use handy_platform::EventSink;
use serde_json::json;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Streams `url` to `dest`, emitting `event_name` with `{ pct, downloaded, total }`
/// every time the integer percentage advances.
pub async fn download_with_progress<S: EventSink + ?Sized>(
    url: &str,
    dest: &Path,
    sink: &S,
    event_name: &str,
) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).await.context("create parent dir")?;
    }
    let tmp = dest.with_extension("part");

    let resp = reqwest::get(url).await.context("request failed")?;
    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let mut file = fs::File::create(&tmp).await.context("create tmp file")?;
    let mut downloaded: u64 = 0;
    let mut last_emit_pct: i64 = -1;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.context("chunk read")?;
        file.write_all(&bytes).await.context("write chunk")?;
        downloaded += bytes.len() as u64;

        let pct = if total > 0 { (downloaded * 100 / total) as i64 } else { -1 };
        if pct != last_emit_pct {
            sink.emit_json(event_name, json!({
                "pct": pct.max(0),
                "downloaded": downloaded,
                "total": total,
            }));
            last_emit_pct = pct;
        }
    }
    file.flush().await.context("flush")?;
    drop(file);
    fs::rename(&tmp, dest).await.context("rename to dest")?;
    sink.emit_json(event_name, json!({
        "pct": 100, "downloaded": downloaded, "total": total,
    }));
    Ok(downloaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct CapturingSink(Mutex<Vec<serde_json::Value>>);
    impl EventSink for CapturingSink {
        fn emit_json(&self, _: &str, payload: serde_json::Value) {
            self.0.lock().unwrap().push(payload);
        }
    }

    #[tokio::test]
    async fn rejects_unreachable_url() {
        let dest = std::env::temp_dir().join("handy-mobile-test-unreachable.bin");
        let sink = CapturingSink(Mutex::new(Vec::new()));
        let r = download_with_progress("http://127.0.0.1:1/none", &dest, &sink, "test").await;
        assert!(r.is_err());
    }
}
