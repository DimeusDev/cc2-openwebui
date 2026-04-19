use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use super::router::AppState;
use crate::error::AppError;

#[derive(serde::Serialize)]
pub struct PrinterStatusResponse {
    pub connected: bool,
    pub connected_raw: bool,
    pub connected_ws: bool,
    pub printer_id: String,
    pub printer_ip: String,
    pub state: Value,
}

pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<PrinterStatusResponse>, AppError> {
    let printer_state = state.printer_state.read().await;
    let full = serde_json::to_value(&printer_state.full).unwrap_or(Value::Null);
    let manager = state.manager.lock().await;

    Ok(Json(PrinterStatusResponse {
        connected: printer_state.connected,
        connected_raw: printer_state.connected_raw,
        connected_ws: printer_state.connected_ws,
        printer_id: manager.printer_id().to_string(),
        printer_ip: manager.printer_ip().to_string(),
        state: full,
    }))
}

#[derive(Deserialize)]
pub struct PrintRequest {
    pub filename: String,
    pub storage_media: String,
    #[serde(default = "default_plate")]
    pub plate: String,
    #[serde(default)]
    pub tray_id: Option<i64>,
    #[serde(default)]
    pub timelapse: bool,
    #[serde(default = "default_true")]
    pub bedlevel_force: bool,
}

fn default_plate() -> String { "textured".to_string() }
fn default_true() -> bool { true }

pub async fn start_print(
    State(state): State<AppState>,
    Json(req): Json<PrintRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: start_print {} plate={} tray={:?}", req.filename, req.plate, req.tray_id);
    let manager = state.manager.lock().await;
    manager.start_print(&req.filename, &req.storage_media, &req.plate, req.tray_id, req.timelapse, req.bedlevel_force).await?;
    Ok(Json(serde_json::json!({ "status": "queued" })))
}

pub async fn pause_print(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: pause_print");
    let manager = state.manager.lock().await;
    manager.pause().await?;
    Ok(Json(serde_json::json!({ "status": "queued" })))
}

pub async fn resume_print(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: resume_print");
    let manager = state.manager.lock().await;
    manager.resume().await?;
    Ok(Json(serde_json::json!({ "status": "queued" })))
}

pub async fn stop_print(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: stop_print");
    let manager = state.manager.lock().await;
    manager.stop_print().await?;
    Ok(Json(serde_json::json!({ "status": "queued" })))
}

#[derive(Deserialize)]
pub struct LedRequest {
    pub power: i64,
}

pub async fn set_led(
    State(state): State<AppState>,
    Json(req): Json<LedRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: set_led power={}", req.power);
    let manager = state.manager.lock().await;
    manager.set_led(req.power != 0).await?;
    Ok(Json(serde_json::json!({ "status": "queued" })))
}

#[derive(Deserialize)]
pub struct FanRequest {
    pub name: String,
    pub speed: u8,
}

pub async fn set_fan(
    State(state): State<AppState>,
    Json(req): Json<FanRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: set_fan {}={}", req.name, req.speed);
    let manager = state.manager.lock().await;
    manager.set_fan(&req.name, req.speed).await?;
    Ok(Json(serde_json::json!({ "status": "queued" })))
}

#[derive(Deserialize)]
pub struct SpeedModeRequest {
    pub mode: u8,
}

pub async fn set_speed_mode(
    State(state): State<AppState>,
    Json(req): Json<SpeedModeRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: set_speed_mode mode={}", req.mode);
    let manager = state.manager.lock().await;
    manager.set_speed_mode(req.mode).await?;
    Ok(Json(serde_json::json!({ "status": "queued" })))
}

#[derive(Deserialize)]
pub struct FileListQuery {
    pub storage: Option<String>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn get_files(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<FileListQuery>,
) -> Result<Json<Value>, AppError> {
    let storage = query.storage.as_deref().unwrap_or("local");
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(20);

    let manager = state.manager.lock().await;
    let result = manager.get_file_list(storage, offset, limit).await?;
    Ok(Json(result))
}

pub async fn get_history(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let manager = state.manager.lock().await;
    let data = manager.get_print_history().await?;
    let history = data.get("history_task_list")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(Json(serde_json::json!({ "history": history })))
}

pub async fn canvas_refresh(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: canvas_refresh");
    let manager = state.manager.lock().await;
    let data = manager.canvas_refresh().await?;
    Ok(Json(data))
}
