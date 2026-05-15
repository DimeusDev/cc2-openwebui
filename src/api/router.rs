use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use super::camera;
use super::detection;
use super::logs;
use super::notifications;
use super::obico;
use super::printer;
use super::settings;
use super::setup;
use super::snapshots;
use super::upload;
use super::version;
use super::ws;
use crate::camera::{CameraStatus, FrameBuffer, FrameBroadcast};
use crate::config::AppConfig;
use crate::printer::manager::PrinterManager;
use crate::printer::state::PrinterState;
use crate::update::UpdateChecker;

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<PrinterManager>,
    pub printer_state: Arc<RwLock<PrinterState>>,
    pub config: Arc<RwLock<AppConfig>>,
    pub det_enabled_rx: tokio::sync::watch::Receiver<bool>,
    pub det_config_rx: tokio::sync::watch::Receiver<crate::config::DetectionConfig>,
    pub det_enabled_tx: Arc<tokio::sync::watch::Sender<bool>>,
    pub det_config_tx: Arc<tokio::sync::watch::Sender<crate::config::DetectionConfig>>,
    pub frame_buffer: FrameBuffer,
    pub camera_status: Arc<CameraStatus>,
    pub frame_broadcast: FrameBroadcast,
    pub db: sqlx::SqlitePool,
    pub camera_ip_tx: Arc<tokio::sync::watch::Sender<String>>,
    pub update_checker: Arc<UpdateChecker>,
}

pub fn build_router(
    manager: Arc<PrinterManager>,
    state: Arc<RwLock<PrinterState>>,
    config: Arc<RwLock<AppConfig>>,
    det_enabled_rx: tokio::sync::watch::Receiver<bool>,
    det_config_rx: tokio::sync::watch::Receiver<crate::config::DetectionConfig>,
    det_enabled_tx: tokio::sync::watch::Sender<bool>,
    det_config_tx: tokio::sync::watch::Sender<crate::config::DetectionConfig>,
    frame_buffer: FrameBuffer,
    camera_status: Arc<CameraStatus>,
    frame_broadcast: FrameBroadcast,
    db: sqlx::SqlitePool,
    camera_ip_tx: Arc<tokio::sync::watch::Sender<String>>,
    update_checker: Arc<UpdateChecker>,
) -> Router {
    let app_state = AppState {
        manager,
        printer_state: state,
        config,
        det_enabled_rx,
        det_config_rx,
        det_enabled_tx: Arc::new(det_enabled_tx),
        det_config_tx: Arc::new(det_config_tx),
        frame_buffer,
        camera_status,
        frame_broadcast,
        db,
        camera_ip_tx,
        update_checker,
    };

    let snapshots_dir = std::path::Path::new("snapshots");
    let _ = std::fs::create_dir_all(snapshots_dir);

    let api = Router::new()
        .nest_service("/snapshots", ServeDir::new("snapshots"))
        .route("/health", get(health))
        .route("/api/setup/check", get(setup::check_setup))
        .route("/api/setup/scan", post(setup::scan_network))
        .route("/api/setup/verify", post(setup::verify_printer))
        .route("/api/setup/save", post(setup::save_config))
        .route("/api/setup/complete", post(setup::complete_onboarding))
        .route("/api/setup/reset", post(setup::reset_setup))
        .route("/api/setup/host-os", get(setup::host_os))
        .route("/api/printer/status", get(printer::get_status))
        .route("/api/printer/pause", post(printer::pause_print))
        .route("/api/printer/resume", post(printer::resume_print))
        .route("/api/printer/stop", post(printer::stop_print))
        .route("/api/printer/home", post(printer::home_axes))
        .route("/api/printer/jog", post(printer::jog_axis))
        .route("/api/printer/led", post(printer::set_led))
        .route("/api/printer/fan", post(printer::set_fan))
        .route("/api/printer/speed-mode", post(printer::set_speed_mode))
        .route("/api/printer/print", post(printer::start_print))
        .route("/api/printer/files", get(printer::get_files))
        .route("/api/printer/history", get(printer::get_history))
        .route("/api/printer/canvas/refresh", post(printer::canvas_refresh))
        .route("/api/printer/canvas/auto-refill", post(printer::set_canvas_auto_refill))
        .route("/api/printer/thumbnail", get(printer::get_thumbnail))
        .route("/api/printer/file-detail", get(printer::get_file_detail))
        .route(
            "/api/printer/upload",
            post(upload::upload_file).route_layer(DefaultBodyLimit::max(600 * 1024 * 1024)),
        )
        .route("/api/detection/status", get(detection::get_status))
        .route("/api/detection/toggle", post(detection::toggle))
        .route("/api/detection/config", post(detection::update_config))
        .route("/api/detection/history", get(detection::get_history))
        .route("/api/detection/grouped", get(detection::get_grouped))
        .route("/api/detection/latest", get(detection::get_latest))
        .route("/api/detection/zones", axum::routing::put(detection::set_zones))
        .route("/api/detection/run", post(detection::run_detection))
        .route("/api/setup/obico/status", get(obico::get_status))
        .route("/api/setup/obico/start", post(obico::start_container))
        .route("/api/setup/obico/start-stream", get(obico::start_container_stream))
        .route("/api/setup/obico/test", get(obico::test_container))
        .route("/api/setup/obico/test-url", post(obico::test_url))
        .route("/api/setup/obico/stop", post(obico::stop_container))
        .route("/api/notifications/destinations",
            get(notifications::list_destinations).post(notifications::create_destination))
        .route("/api/notifications/destinations/:id",
            axum::routing::put(notifications::update_destination)
                .delete(notifications::delete_destination))
        .route("/api/notifications/destinations/:id/test", post(notifications::test_destination))
        .route("/api/camera/snapshot", get(camera::snapshot))
        .route("/api/camera/stream", get(camera::stream))
        .route("/api/camera/status", get(camera::status))
        .route("/api/settings",
            get(settings::get_settings).post(settings::update_settings))
        .route("/api/logs",
            get(logs::get_logs).delete(logs::delete_logs))
        .route("/api/snapshots",
            get(snapshots::list_snapshots).delete(snapshots::delete_all_snapshots))
        .route("/api/snapshots/:filename", axum::routing::delete(snapshots::delete_snapshot))
        .route("/api/version", get(version::get_version))
        .route("/ws", get(ws::ws_handler))
        .with_state(app_state);

    let frontend_dir = find_frontend_dir().join("frontend/dist");

    tracing::info!("serving frontend from: {:?}", frontend_dir);

    let frontend = Router::new()
        .fallback_service(
            ServeDir::new(&frontend_dir)
                .fallback(get(serve_index)),
        );

    api.merge(frontend)
}

fn find_frontend_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| {
            let mut dir = p;
            for _ in 0..3 {
                dir = dir.parent()?.to_path_buf();
            }
            Some(dir)
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn serve_index() -> axum::response::Html<String> {
    let index_path = find_frontend_dir().join("frontend/dist/index.html");
    match std::fs::read_to_string(&index_path) {
        Ok(html) => axum::response::Html(html),
        Err(e) => {
            tracing::error!("failed to serve index.html: {e}");
            axum::response::Html("<h1>Frontend not built. Run: cd frontend && npm run build</h1>".to_string())
        }
    }
}
