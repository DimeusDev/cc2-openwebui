use axum::extract::Query;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tracing::warn;

use crate::detection::obico::Detection;

const SNAPSHOTS_DIR: &str = "snapshots";

#[derive(Deserialize)]
pub struct ListQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct SnapshotEntry {
    pub filename: String,
    pub size: u64,
    pub mtime: u64,
    pub score_pct: Option<u32>,
    pub boxes: Vec<Detection>,
}

#[derive(Serialize)]
pub struct SnapshotListResponse {
    pub snapshots: Vec<SnapshotEntry>,
    pub total: usize,
    pub total_bytes: u64,
}

pub async fn list_snapshots(Query(q): Query<ListQuery>) -> Json<SnapshotListResponse> {
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(50).min(200);

    let mut entries = read_snapshot_dir();
    entries.sort_by(|a, b| b.mtime.cmp(&a.mtime));

    let total = entries.len();
    let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
    let snapshots: Vec<SnapshotEntry> = entries.into_iter().skip(offset).take(limit).collect();

    Json(SnapshotListResponse { snapshots, total, total_bytes })
}

pub async fn delete_snapshot(
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Json<Value> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Json(serde_json::json!({ "error": "invalid filename" }));
    }
    let path = Path::new(SNAPSHOTS_DIR).join(&filename);
    let deleted = std::fs::remove_file(&path).is_ok();
    let _ = std::fs::remove_file(path.with_extension("json"));
    Json(serde_json::json!({ "deleted": deleted }))
}

pub async fn delete_all_snapshots() -> Json<Value> {
    let dir = Path::new(SNAPSHOTS_DIR);
    let mut deleted = 0u32;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jpg") {
                if let Err(e) = std::fs::remove_file(&path) {
                    warn!("failed to delete snapshot {:?}: {e}", path);
                } else {
                    deleted += 1;
                    let _ = std::fs::remove_file(path.with_extension("json"));
                }
            }
        }
    }
    Json(serde_json::json!({ "deleted": deleted }))
}

fn read_snapshot_dir() -> Vec<SnapshotEntry> {
    let dir = Path::new(SNAPSHOTS_DIR);
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new(); };

    rd.flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("jpg") {
                return None;
            }
            let filename = path.file_name()?.to_str()?.to_string();
            let meta = std::fs::metadata(&path).ok()?;
            let size = meta.len();
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let score_pct = parse_score_from_filename(&filename);
            let json_path = path.with_extension("json");
            let boxes: Vec<Detection> = std::fs::read_to_string(&json_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            Some(SnapshotEntry { filename, size, mtime, score_pct, boxes })
        })
        .collect()
}

fn parse_score_from_filename(name: &str) -> Option<u32> {
    let stem = name.strip_suffix(".jpg")?;
    let parts: Vec<&str> = stem.splitn(3, '_').collect();
    if parts.len() == 3 && parts[0] == "detection" {
        parts[2].parse().ok()
    } else {
        None
    }
}
