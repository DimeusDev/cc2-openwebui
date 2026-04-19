use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use tracing::{info, warn};

use super::router::AppState;
use crate::error::AppError;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB, matches observed slicer behavior

pub async fn upload_file(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    let printer_ip = {
        let cfg = state.config.read().await;
        cfg.printer.ip.clone()
    };

    if printer_ip.is_empty() {
        return Err(AppError::Validation("printer IP not configured".to_string()));
    }

    let total = body.len();
    if total == 0 {
        return Err(AppError::Validation("empty file".to_string()));
    }

    let url = format!("http://{}/upload", printer_ip);
    let client = reqwest::Client::new();
    let mut start = 0usize;

    while start < total {
        let end = (start + CHUNK_SIZE - 1).min(total - 1);
        let chunk = body.slice(start..=end);
        let len = chunk.len();

        let resp = client
            .put(&url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", len.to_string())
            .header("Content-Range", format!("bytes {}-{}/{}", start, end, total))
            .body(chunk)
            .send()
            .await
            .map_err(|e| AppError::Validation(format!("upload chunk {start}-{end} failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::Validation(format!(
                "printer rejected chunk {start}-{end}: HTTP {}",
                resp.status()
            )));
        }

        let ack: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Validation(format!("bad ACK from printer: {e}")))?;

        if ack.get("error_code").and_then(|v| v.as_u64()) != Some(0) {
            warn!("printer upload error: {:?}", ack);
            return Err(AppError::Validation(format!(
                "printer error on chunk {start}-{end}: {:?}",
                ack.get("error_code")
            )));
        }

        start = end + 1;
    }

    info!("file upload complete: {} bytes", total);
    Ok(Json(serde_json::json!({ "ok": true, "bytes": total })))
}
