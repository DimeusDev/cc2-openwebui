mod api;
mod camera;
mod config;
mod detection;
mod docker;
mod error;
mod notifications;
mod printer;

use std::sync::Arc;

use tokio::sync::{watch, Mutex, RwLock};
use tracing::{error, info};

use camera::{spawn_frame_grabber, FrameBuffer};
use config::AppConfig;
use printer::manager::PrinterManager;
use printer::state::PrinterState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    println!("cc2-openwebui v{VERSION}");

    let (config, pre_configured) = AppConfig::load_or_default();

    init_tracing(&config.logging.level);

    let _ = std::fs::create_dir_all("data");
    let _ = std::fs::create_dir_all("snapshots");

    let config_arc = Arc::new(RwLock::new(config.clone()));

    let (det_enabled_tx, det_enabled_rx) = watch::channel(config.detection.enabled);
    let (det_config_tx, det_config_rx) = watch::channel(config.detection.clone());

    let mut manager = PrinterManager::new(config.clone());
    let state_changed_tx = manager.state_changed_sender();

    if pre_configured {
        if let Err(e) = manager.start().await {
            error!("failed to start printer manager: {e}");
        }
        info!("printer manager started for {}", config.printer.ip);
    } else {
        info!("no printer configured, setup available");
    }

    {
        let past = PrinterState::load_events_from_log(100);
        if !past.is_empty() {
            let mut s = manager.state.write().await;
            s.events = past;
        }
    }

    let manager_state = manager.state.clone();
    let manager_arc = Arc::new(Mutex::new(manager));

    let frame_buffer: FrameBuffer = Arc::new(RwLock::new(None));

    if pre_configured {
        spawn_frame_grabber(config.printer.ip.clone(), frame_buffer.clone());
        info!("frame grabber started for {}", config.printer.ip);

        let det_engine = detection::engine::DetectionEngine::new(
            config.detection.clone(),
            config.server.port,
            frame_buffer.clone(),
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
        let notif_state_rx = {
            let manager = manager_arc.lock().await;
            manager.state_changed_rx()
        };
        let notif_manager = notifications::manager::NotificationManager::new(
            manager_state.clone(),
            config_arc.clone(),
        );
        tokio::spawn(async move {
            notif_manager.run(notif_state_rx).await;
        });
        info!("notification manager started");
    }

    let router = api::router::build_router(
        manager_arc,
        manager_state,
        config_arc,
        det_enabled_rx,
        det_config_rx,
        det_enabled_tx,
        det_config_tx,
        frame_buffer,
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

fn init_tracing(level: &str) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
