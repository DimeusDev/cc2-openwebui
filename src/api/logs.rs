use axum::extract::State;
use axum::Json;
use serde_json::Value;
use std::time::UNIX_EPOCH;
use tracing::warn;

use super::router::AppState;
use crate::printer::state::{EventKind, EVENTS_LOG_PATH};

pub async fn delete_logs(State(state): State<AppState>) -> Json<Value> {
    let mut s = state.printer_state.write().await;
    s.events.clear();
    if let Err(e) = std::fs::write(EVENTS_LOG_PATH, "") {
        warn!("failed to truncate log file: {e}");
    }
    Json(serde_json::json!({ "deleted": true }))
}

pub async fn get_logs(State(state): State<AppState>) -> Json<Value> {
    let s = state.printer_state.read().await;
    let logs: Vec<Value> = s.events.iter().map(|e| {
        let ts = e.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let kind = match &e.kind {
            EventKind::Loaded(s) => s.clone(),
            other => format!("{other:?}"),
        };
        let mut entry = serde_json::json!({
            "timestamp": ts,
            "kind": kind,
            "message": e.description,
        });
        if let Some(snap) = &e.snapshot {
            entry["snapshot"] = Value::String(snap.clone());
        }
        entry
    }).collect();
    Json(serde_json::json!({ "logs": logs }))
}
