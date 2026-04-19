use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::{broadcast, oneshot, watch, RwLock};
use tracing::{error, info};

use super::client_raw::MqttRawClient;
use super::client_ws::MqttWsClient;
use super::commands::{Command, PendingRpcs};
use super::models::{
    METHOD_GET_AMS_INFO, METHOD_GET_PRINT_HISTORY,
    METHOD_PAUSE_PRINT, METHOD_RESUME_PRINT, METHOD_SET_FAN, METHOD_SET_LED,
    METHOD_SET_SPEED_MODE, METHOD_START_PRINT, METHOD_STOP_PRINT,
};
use super::state::{EventKind, PrinterEvent, PrinterState};
use crate::config::AppConfig;
use crate::error::PrinterError;

pub struct PrinterManager {
    pub state: Arc<RwLock<PrinterState>>,
    pub event_tx: broadcast::Sender<PrinterEvent>,

    raw_connected_tx: watch::Sender<bool>,
    raw_connected_rx: watch::Receiver<bool>,
    ws_connected_tx: watch::Sender<bool>,
    ws_connected_rx: watch::Receiver<bool>,

    raw_shutdown: Option<watch::Sender<bool>>,
    ws_shutdown: Option<watch::Sender<bool>>,

    ws_cmd_tx: broadcast::Sender<Command>,
    raw_cmd_tx: broadcast::Sender<Command>,
    state_changed_tx: broadcast::Sender<()>,

    /// Pending RPC calls waiting for a printer response by id.
    pending_rpcs: PendingRpcs,
    /// Monotonic counter for RPC IDs (starts high to avoid collision with client-local sequences).
    rpc_id_seq: Arc<AtomicU64>,

    printer_id: String,
    raw_client_id: String,
    ws_client_id: String,
    config: AppConfig,
    running: Arc<AtomicBool>,
}

impl PrinterManager {
    pub fn new(config: AppConfig) -> Self {
        let state = Arc::new(RwLock::new(PrinterState::new()));
        let (raw_connected_tx, raw_connected_rx) = watch::channel(false);
        let (ws_connected_tx, ws_connected_rx) = watch::channel(false);
        let (event_tx, _) = broadcast::channel(100);
        let (ws_cmd_tx, _) = broadcast::channel(32);
        let (raw_cmd_tx, _) = broadcast::channel(32);
        let (state_changed_tx, _) = broadcast::channel(64);

        Self {
            state,
            event_tx,
            raw_connected_tx,
            raw_connected_rx,
            ws_connected_tx,
            ws_connected_rx,
            raw_shutdown: None,
            ws_shutdown: None,
            ws_cmd_tx,
            raw_cmd_tx,
            state_changed_tx,
            pending_rpcs: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            rpc_id_seq: Arc::new(AtomicU64::new(10_000)),
            printer_id: config.printer.printer_id.clone(),
            raw_client_id: format!("cc2_{}", rand_digits(4)),
            ws_client_id: format!("0clid{}", rand_hex(8)),
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&mut self) -> Result<(), PrinterError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let printer_id = if self.config.printer.printer_id.is_empty() {
            let discovered = super::discovery::discover_printer_id(
                &self.config.printer.ip,
                "elegoo",
                self.config.printer_password(),
                3,
            )
            .await?;
            info!("discovered printer ID: {discovered}");
            discovered
        } else {
            self.config.printer.printer_id.clone()
        };

        self.printer_id = printer_id.clone();
        self.state.write().await.printer_ip = self.config.printer.ip.clone();

        let (raw_shutdown_tx, raw_shutdown_rx) = watch::channel(false);
        let (ws_shutdown_tx, ws_shutdown_rx) = watch::channel(false);
        self.raw_shutdown = Some(raw_shutdown_tx);
        self.ws_shutdown = Some(ws_shutdown_tx);

        // Watcher task: keeps PrinterState.connected in sync with both connections.
        {
            let state = self.state.clone();
            let state_changed_tx = self.state_changed_tx.clone();
            let mut raw_rx = self.raw_connected_rx.clone();
            let mut ws_rx = self.ws_connected_rx.clone();
            tokio::spawn(async move {
                loop {
                    let raw = *raw_rx.borrow();
                    let ws = *ws_rx.borrow();
                    {
                        let mut s = state.write().await;
                        s.connected = raw && ws;
                        s.connected_raw = raw;
                        s.connected_ws = ws;
                    }
                    state_changed_tx.send(()).ok();
                    tokio::select! {
                        _ = raw_rx.changed() => {}
                        _ = ws_rx.changed() => {}
                    }
                }
            });
        }

        // Raw MQTT client (port 1883)
        {
            let state = self.state.clone();
            let connected_tx = self.raw_connected_tx.clone();
            let state_changed_tx = self.state_changed_tx.clone();
            let raw_cmd_tx = self.raw_cmd_tx.clone();
            let pid = printer_id.clone();
            let raw_client_id = self.raw_client_id.clone();
            let ip = self.config.printer.ip.clone();
            let password = self.config.printer_password().to_string();
            let running = self.running.clone();

            tokio::spawn(async move {
                running.store(true, Ordering::SeqCst);
                let mut backoff = 2u64;
                loop {
                    let attempt_start = std::time::Instant::now();
                    let result = MqttRawClient::connect_and_run(
                        &ip,
                        &pid,
                        "elegoo",
                        &password,
                        &raw_client_id,
                        state.clone(),
                        connected_tx.clone(),
                        state_changed_tx.clone(),
                        raw_cmd_tx.subscribe(),
                        raw_shutdown_rx.clone(),
                    )
                    .await;

                    if let Err(e) = result {
                        error!("raw client disconnected: {e}");
                        connected_tx.send(false).ok();
                    }

                    if !running.load(Ordering::SeqCst) {
                        break;
                    }

                    if attempt_start.elapsed().as_secs() >= 10 {
                        backoff = 2;
                    }

                    info!("raw client reconnecting in {backoff}s");
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = (backoff * 2).min(30);
                }
            });
        }

        // WebSocket MQTT client (port 9001)
        {
            let state = self.state.clone();
            let connected_tx = self.ws_connected_tx.clone();
            let state_changed_tx = self.state_changed_tx.clone();
            let ws_cmd_tx = self.ws_cmd_tx.clone();
            let pid = printer_id.clone();
            let ws_client_id = self.ws_client_id.clone();
            let ip = self.config.printer.ip.clone();
            let password = self.config.printer_password().to_string();
            let running = self.running.clone();
            let pending_rpcs = self.pending_rpcs.clone();

            tokio::spawn(async move {
                let mut backoff = 2u64;
                loop {
                    let attempt_start = std::time::Instant::now();
                    let result = MqttWsClient::connect_and_run(
                        &ip,
                        &pid,
                        "elegoo",
                        &password,
                        &ws_client_id,
                        state.clone(),
                        connected_tx.clone(),
                        state_changed_tx.clone(),
                        ws_cmd_tx.subscribe(),
                        ws_shutdown_rx.clone(),
                        pending_rpcs.clone(),
                    )
                    .await;

                    if let Err(e) = result {
                        error!("ws client disconnected: {e}");
                        connected_tx.send(false).ok();
                    }

                    if !running.load(Ordering::SeqCst) {
                        break;
                    }

                    if attempt_start.elapsed().as_secs() >= 10 {
                        backoff = 2;
                    }

                    info!("ws client reconnecting in {backoff}s");
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = (backoff * 2).min(30);
                }
            });
        }

        info!("printer manager started for {printer_id}");
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(tx) = self.raw_shutdown.take() {
            tx.send(true).ok();
        }
        if let Some(tx) = self.ws_shutdown.take() {
            tx.send(true).ok();
        }
        self.raw_connected_tx.send(false).ok();
        self.ws_connected_tx.send(false).ok();
        info!("printer manager stopped");
    }

    pub fn update_config(&mut self, new_config: AppConfig) {
        if self.running.load(Ordering::SeqCst) {
            self.running.store(false, Ordering::SeqCst);
            if let Some(tx) = self.raw_shutdown.take() {
                tx.send(true).ok();
            }
            if let Some(tx) = self.ws_shutdown.take() {
                tx.send(true).ok();
            }
        }
        self.config = new_config;
        self.raw_client_id = format!("cc2_{}", rand_digits(4));
        self.ws_client_id = format!("0clid{}", rand_hex(8));
        self.printer_id = self.config.printer.printer_id.clone();
        info!("printer manager config updated");
    }

    async fn rpc_call(
        &self,
        method: u16,
        params: Option<serde_json::Value>,
        timeout_secs: u64,
    ) -> Result<serde_json::Value, PrinterError> {
        let id = self.rpc_id_seq.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel::<serde_json::Value>();
        self.pending_rpcs.lock().await.insert(id, tx);

        let send_result = self.ws_cmd_tx.send(Command { id, method, params });
        if send_result.is_err() {
            self.pending_rpcs.lock().await.remove(&id);
            return Err(PrinterError::NotConnected);
        }

        match tokio::time::timeout(Duration::from_secs(timeout_secs), rx).await {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(_)) | Err(_) => {
                self.pending_rpcs.lock().await.remove(&id);
                Err(PrinterError::RpcTimeout)
            }
        }
    }

    pub async fn pause(&self) -> Result<(), PrinterError> {
        info!("[cmd] pause_print");
        self.state.write().await.add_event(EventKind::CommandPause, "Pause print sent".to_string());
        self.ws_cmd_tx
            .send(Command { id: 0, method: METHOD_PAUSE_PRINT, params: None })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), PrinterError> {
        info!("[cmd] resume_print");
        self.state.write().await.add_event(EventKind::CommandResume, "Resume print sent".to_string());
        self.ws_cmd_tx
            .send(Command { id: 0, method: METHOD_RESUME_PRINT, params: None })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(())
    }

    pub async fn stop_print(&self) -> Result<(), PrinterError> {
        info!("[cmd] stop_print");
        self.state.write().await.add_event(EventKind::CommandStop, "Stop print sent".to_string());
        self.ws_cmd_tx
            .send(Command { id: 0, method: METHOD_STOP_PRINT, params: None })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(())
    }

    pub async fn set_led(&self, power: bool) -> Result<(), PrinterError> {
        info!("[cmd] set_led power={power}");
        self.state.write().await.add_event(
            EventKind::CommandLed(power),
            format!("LED set to {}", if power { "on" } else { "off" }),
        );
        self.ws_cmd_tx
            .send(Command {
                id: 0,
                method: METHOD_SET_LED,
                params: Some(serde_json::json!({ "power": if power { 1i64 } else { 0i64 } })),
            })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(())
    }

    pub async fn set_fan(&self, name: &str, speed: u8) -> Result<(), PrinterError> {
        info!("[cmd] set_fan name={name} speed={speed}");
        self.state.write().await.add_event(
            EventKind::CommandFan(name.to_string(), speed),
            format!("Fan '{}' set to {}", name, speed),
        );
        self.ws_cmd_tx
            .send(Command {
                id: 0,
                method: METHOD_SET_FAN,
                params: Some(serde_json::json!({ name: speed as i64 })),
            })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(())
    }

    pub async fn set_speed_mode(&self, mode: u8) -> Result<(), PrinterError> {
        info!("[cmd] set_speed_mode mode={mode}");
        self.state.write().await.add_event(
            EventKind::CommandSpeedMode(mode),
            format!("Speed mode set to {}", mode),
        );
        self.ws_cmd_tx
            .send(Command {
                id: 0,
                method: METHOD_SET_SPEED_MODE,
                params: Some(serde_json::json!({ "mode": mode as i64 })),
            })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(())
    }

    pub async fn start_print(
        &self,
        filename: &str,
        storage_media: &str,
        plate: &str,
        tray_id: Option<i64>,
        timelapse: bool,
        bedlevel_force: bool,
    ) -> Result<(), PrinterError> {
        info!("[cmd] start_print filename={filename} plate={plate} tray={tray_id:?}");
        self.state.write().await.add_event(
            EventKind::CommandStartPrint,
            format!("Start print: {}", filename),
        );
        let print_layout = if plate == "smooth" { "B" } else { "A" };
        let tray_id = tray_id.unwrap_or(0);
        self.raw_cmd_tx
            .send(Command {
                id: 0,
                method: METHOD_START_PRINT,
                params: Some(serde_json::json!({
                    "filename": filename,
                    "storage_media": storage_media,
                    "config": {
                        "bedlevel_force": bedlevel_force,
                        "delay_video": timelapse,
                        "print_layout": print_layout,
                        "printer_check": true,
                        "slot_map": [{"canvas_id": 0, "t": 0, "tray_id": tray_id}]
                    }
                })),
            })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(())
    }

    pub async fn get_file_list(&self, storage: &str, offset: i64, limit: i64) -> Result<serde_json::Value, PrinterError> {
        self.ws_cmd_tx
            .send(Command {
                id: 0,
                method: super::models::METHOD_GET_FILE_LIST,
                params: Some(serde_json::json!({
                    "storage_media": storage,
                    "offset": offset,
                    "limit": limit,
                })),
            })
            .map_err(|_| PrinterError::NotConnected)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    pub async fn get_print_history(&self) -> Result<serde_json::Value, PrinterError> {
        info!("[cmd] get_print_history");
        self.rpc_call(METHOD_GET_PRINT_HISTORY, None, 10).await
    }

    pub async fn canvas_refresh(&self) -> Result<serde_json::Value, PrinterError> {
        info!("[cmd] canvas_refresh");
        self.rpc_call(METHOD_GET_AMS_INFO, Some(serde_json::json!({})), 10).await
    }

    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        *self.raw_connected_rx.borrow() && *self.ws_connected_rx.borrow()
    }

    pub fn printer_id(&self) -> &str {
        &self.printer_id
    }

    pub fn printer_ip(&self) -> &str {
        &self.config.printer.ip
    }

    pub fn event_rx(&self) -> broadcast::Receiver<PrinterEvent> {
        self.event_tx.subscribe()
    }

    pub fn state_changed_rx(&self) -> broadcast::Receiver<()> {
        self.state_changed_tx.subscribe()
    }

    pub fn state_changed_sender(&self) -> broadcast::Sender<()> {
        self.state_changed_tx.clone()
    }
}

fn rand_digits(n: usize) -> String {
    use rand::Rng;
    (0..n).map(|_| rand::thread_rng().gen_range(0..10u8).to_string()).collect()
}

fn rand_hex(n: usize) -> String {
    use rand::Rng;
    (0..n).map(|_| format!("{:x}", rand::thread_rng().gen_range(0..16u8))).collect()
}
