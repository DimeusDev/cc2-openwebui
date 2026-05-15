mod api;
mod camera;
mod config;
mod db;
mod detection;
mod docker;
mod error;
mod notifications;
mod printer;
mod update;

use std::sync::Arc;
use std::time::UNIX_EPOCH;

use tokio::sync::{broadcast, watch, RwLock};
use tracing::{error, info, warn};

use camera::{spawn_frame_grabber, FrameBuffer};
use config::AppConfig;
use printer::manager::PrinterManager;
use printer::state::EventKind;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    println!("cc2-openwebui v{VERSION}");

    let _ = std::fs::create_dir_all("data");
    let _ = std::fs::create_dir_all("snapshots");

    let db = match db::init_db("data/cc2.db").await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("error: failed to initialize database: {e}");
            std::process::exit(1);
        }
    };

    db::migrate_jsonl(&db).await;

    let config = match db::load_app_config(&db).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warn: could not load config from db, using defaults: {e}");
            AppConfig::default()
        }
    };
    let pre_configured = !config.printer.ip.is_empty();

    init_tracing(&config.logging.level);
    info!("database initialized at data/cc2.db");

    let config_arc = Arc::new(RwLock::new(config.clone()));

    let (det_enabled_tx, det_enabled_rx) = watch::channel(config.detection.enabled);
    let (det_config_tx, det_config_rx) = watch::channel(config.detection.clone());

    let manager = PrinterManager::new(config.clone());
    let state_changed_tx = manager.state_changed_sender();

    let event_rx_for_db = manager.event_rx();

    if pre_configured {
        if let Err(e) = manager.start().await {
            error!("failed to start printer manager: {e}");
        }
        info!("printer manager started for {}", config.printer.ip);
    } else {
        info!("no printer configured, setup available");
    }

    {
        let past = db::query_events(&db, 100).await;
        let total = db::count_events(&db).await;
        if !past.is_empty() || total > 0 {
            let mut s = manager.state.write().await;
            s.events_total = total.max(past.len() as u64);
            s.events = past;
        }
    }

    {
        let history = db::query_detection_points(&db, None, 300).await;
        if !history.is_empty() {
            let mut s = manager.state.write().await;
            s.detection_history = history.into();
        }
    }

    let manager_state = manager.state.clone();
    let manager_arc = Arc::new(manager);

    {
        let db_w = db.clone();
        tokio::spawn(async move {
            let mut rx = event_rx_for_db;
            loop {
                match rx.recv().await {
                    Ok(evt) => {
                        let ts = evt.timestamp.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                        let kind = event_kind_str_for_db(&evt.kind);
                        db::insert_event(&db_w, ts, &kind, &evt.description, evt.snapshot.as_deref()).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("[db] event writer lagged, {n} events dropped");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    let frame_buffer: FrameBuffer = Arc::new(RwLock::new(None));

    let (camera_ip_tx, camera_ip_rx) = watch::channel(config.printer.ip.clone());
    let camera_ip_tx = Arc::new(camera_ip_tx);

    let (camera_status, frame_broadcast, cam_connected_rx) =
        spawn_frame_grabber(camera_ip_rx, frame_buffer.clone());
    info!("frame grabber started (ip={})", config.printer.ip);

    {
        let watcher_state = manager_state.clone();
        let watcher_changed = state_changed_tx.clone();
        tokio::spawn(async move {
            let mut rx = cam_connected_rx;
            loop {
                if rx.changed().await.is_err() { break; }
                let connected = *rx.borrow();
                let (kind, msg) = if connected {
                    (EventKind::CameraRestored, "Camera feed restored".to_string())
                } else {
                    (EventKind::CameraLost, "Camera feed lost".to_string())
                };
                {
                    let mut s = watcher_state.write().await;
                    s.camera_connected = connected;
                    s.add_event(kind, msg);
                }
                let _ = watcher_changed.send(());
            }
        });
    }

    {
        let det_engine = detection::engine::DetectionEngine::new(
            config.detection.clone(),
            config.server.port,
            frame_buffer.clone(),
            Some(db.clone()),
        );

        let det_enabled_rx_clone = det_enabled_rx.clone();
        let det_config_rx_clone = det_config_rx.clone();
        let det_state = manager_state.clone();
        let det_state_changed = state_changed_tx.clone();
        let det_manager = manager_arc.clone();
        tokio::spawn(async move {
            let (_shutdown_tx, shutdown_rx) = watch::channel(false);
            det_engine
                .run(det_state, det_manager, det_enabled_rx_clone, det_config_rx_clone, shutdown_rx, det_state_changed)
                .await;
        });
        info!("detection engine started");
    }

    {
        let notif_state_rx = manager_arc.state_changed_rx();
        let notif_manager = notifications::manager::NotificationManager::new(
            manager_state.clone(),
            config_arc.clone(),
        );
        tokio::spawn(async move {
            notif_manager.run(notif_state_rx).await;
        });
        info!("notification manager started");
    }

    let update_checker = update::UpdateChecker::new();
    update_checker.clone().start();
    info!("update checker started (current sha={})", env!("GIT_HASH"));

    let router = api::router::build_router(
        manager_arc,
        manager_state,
        config_arc,
        det_enabled_rx,
        det_config_rx,
        det_enabled_tx,
        det_config_tx,
        frame_buffer,
        camera_status,
        frame_broadcast,
        db,
        camera_ip_tx,
        update_checker,
    );

    let addr = format!("{}:{}", config.server.host, config.server.port);
    info!("binding to {addr}");

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("failed to bind to {addr}: {e}");
            std::process::exit(1);
        }
    };

    info!("server listening on http://{addr}");
    println!("server listening on http://{addr}");

    if let Err(e) = axum::serve(listener, router).await {
        error!("server error: {e}");
        std::process::exit(1);
    }
}

fn event_kind_str_for_db(kind: &EventKind) -> String {
    match kind {
        EventKind::Loaded(s) => s.clone(),
        EventKind::PhaseChanged(code, s) => format!("PhaseChanged({code},{s})"),
        other => format!("{other:?}"),
    }
}

fn init_tracing(level: &str) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
