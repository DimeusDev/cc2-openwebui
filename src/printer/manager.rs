use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::{broadcast, oneshot, watch, RwLock};
use tracing::{error, info};

use super::client_raw::MqttRawClient;
use super::client_ws::MqttWsClient;
use super::commands::{Command, PendingRpcs};
use super::models::{
    METHOD_GET_AMS_INFO, METHOD_GET_FILE_INFO, METHOD_GET_FILE_LIST, METHOD_GET_FILE_THUMBNAIL,
    METHOD_GET_FULL_STATUS, METHOD_GET_PRINT_HISTORY, METHOD_HOME_AXES, METHOD_JOG_AXIS,
    METHOD_PAUSE_PRINT, METHOD_RESUME_PRINT, METHOD_SET_AMS_AUTO_REFILL, METHOD_SET_FAN,
    METHOD_SET_LED, METHOD_SET_SPEED_MODE, METHOD_START_PRINT, METHOD_STOP_PRINT,
};
use super::state::{EventKind, PrinterEvent, PrinterState};
use crate::config::AppConfig;
use crate::error::PrinterError;

// lifecycle config; mutate only in start/stop/update_config
struct LifecycleState {
    raw_shutdown: Option<watch::Sender<bool>>,
    ws_shutdown: Option<watch::Sender<bool>>,
    config: AppConfig,
    printer_id: String,
    raw_client_id: String,
}

pub struct PrinterManager {
    pub state: Arc<RwLock<PrinterState>>,
    pub event_tx: broadcast::Sender<PrinterEvent>,

    raw_connected_tx: watch::Sender<bool>,
    raw_connected_rx: watch::Receiver<bool>,
    ws_connected_tx: watch::Sender<bool>,
    ws_connected_rx: watch::Receiver<bool>,

    ws_cmd_tx: broadcast::Sender<Command>,
    raw_cmd_tx: broadcast::Sender<Command>,
    state_changed_tx: broadcast::Sender<()>,

    pending_rpcs: PendingRpcs,
    rpc_id_seq: Arc<AtomicU64>,

    // single in-flight 1044 avoids refresh stampede timeouts
    file_list_lock: tokio::sync::Mutex<()>,

    running: Arc<AtomicBool>,
    lifecycle: tokio::sync::Mutex<LifecycleState>,
}

impl PrinterManager {
    pub fn new(config: AppConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        let state = Arc::new(RwLock::new(PrinterState::new(event_tx.clone())));
        let (raw_connected_tx, raw_connected_rx) = watch::channel(false);
        let (ws_connected_tx, ws_connected_rx) = watch::channel(false);
        let (ws_cmd_tx, _) = broadcast::channel(32);
        let (raw_cmd_tx, _) = broadcast::channel(32);
        let (state_changed_tx, _) = broadcast::channel(64);

        let lifecycle = tokio::sync::Mutex::new(LifecycleState {
            printer_id: config.printer.printer_id.clone(),
            raw_client_id: format!("cc2_{}", rand_digits(4)),
            raw_shutdown: None,
            ws_shutdown: None,
            config,
        });

        Self {
            state,
            event_tx,
            raw_connected_tx,
            raw_connected_rx,
            ws_connected_tx,
            ws_connected_rx,
            ws_cmd_tx,
            raw_cmd_tx,
            state_changed_tx,
            pending_rpcs: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            rpc_id_seq: Arc::new(AtomicU64::new(10_000)),
            file_list_lock: tokio::sync::Mutex::new(()),
            running: Arc::new(AtomicBool::new(false)),
            lifecycle,
        }
    }

    pub async fn start(&self) -> Result<(), PrinterError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        // release lock before async network ops
        let (cfg_printer_id, cfg_ip, cfg_password) = {
            let lc = self.lifecycle.lock().await;
            (
                lc.config.printer.printer_id.clone(),
                lc.config.printer.ip.clone(),
                lc.config.printer_password().to_string(),
            )
        };

        let printer_id = if cfg_printer_id.is_empty() {
            let discovered = super::discovery::discover_printer_id(
                &cfg_ip,
                "elegoo",
                &cfg_password,
                3,
            )
            .await?;
            info!("discovered printer ID: {discovered}");
            discovered
        } else {
            cfg_printer_id
        };

        // create fresh shutdown channels for this run
        let (raw_shutdown_rx, ws_shutdown_rx, raw_client_id, ip, password) = {
            let mut lc = self.lifecycle.lock().await;
            lc.printer_id = printer_id.clone();

            let (raw_shutdown_tx, raw_shutdown_rx) = watch::channel(false);
            let (ws_shutdown_tx, ws_shutdown_rx) = watch::channel(false);
            lc.raw_shutdown = Some(raw_shutdown_tx);
            lc.ws_shutdown = Some(ws_shutdown_tx);

            (
                raw_shutdown_rx,
                ws_shutdown_rx,
                lc.raw_client_id.clone(),
                lc.config.printer.ip.clone(),
                lc.config.printer_password().to_string(),
            )
        };

        self.state.write().await.printer_ip = ip.clone();

        // connection watcher
        {
            let state = self.state.clone();
            let state_changed_tx = self.state_changed_tx.clone();
            let ws_cmd_tx_watcher = self.ws_cmd_tx.clone();
            let rpc_id_watcher = self.rpc_id_seq.clone();
            let mut raw_rx = self.raw_connected_rx.clone();
            let mut ws_rx = self.ws_connected_rx.clone();
            tokio::spawn(async move {
                let mut was_connected = false;
                loop {
                    let raw = *raw_rx.borrow();
                    let ws = *ws_rx.borrow();
                    let new_connected = ws;
                    {
                        let mut s = state.write().await;
                        s.connected = new_connected;
                        s.connected_raw = raw;
                        s.connected_ws = ws;
                        if was_connected && !new_connected {
                            s.clear_on_disconnect();
                            s.add_event(EventKind::Disconnected, "Printer disconnected".to_string());
                        } else if !was_connected && new_connected {
                            s.add_event(EventKind::Connected, "Printer connected".to_string());
                        }
                    }
                    let reconnect = !was_connected && new_connected;
                    was_connected = new_connected;
                    state_changed_tx.send(()).ok();
                    // re-seed immediately: clear_on_disconnect sets machine=-1;
                    // WS only re-fetches on its own reconnect, not on raw-only reconnect
                    if reconnect {
                        let id = rpc_id_watcher.fetch_add(1, Ordering::SeqCst);
                        ws_cmd_tx_watcher.send(Command { id, method: METHOD_GET_FULL_STATUS, params: None }).ok();
                    }
                    tokio::select! {
                        _ = raw_rx.changed() => {}
                        _ = ws_rx.changed() => {}
                    }
                }
            });
        }

        self.running.store(true, Ordering::SeqCst);
        self.spawn_raw_client(&ip, &printer_id, &raw_client_id, &password, raw_shutdown_rx);
        self.spawn_ws_client(&ip, &printer_id, &password, ws_shutdown_rx);

        info!("printer manager started for {printer_id}");
        Ok(())
    }

    fn spawn_raw_client(
        &self,
        ip: &str,
        printer_id: &str,
        raw_client_id: &str,
        password: &str,
        shutdown_rx: watch::Receiver<bool>,
    ) {
        let state = self.state.clone();
        let connected_tx = self.raw_connected_tx.clone();
        let state_changed_tx = self.state_changed_tx.clone();
        let raw_cmd_tx = self.raw_cmd_tx.clone();
        let running = self.running.clone();

        let ip = ip.to_string();
        let pid = printer_id.to_string();
        let raw_client_id = raw_client_id.to_string();
        let password = password.to_string();

        tokio::spawn(async move {
            let mut backoff = 2u64;
            let mut attempt: u32 = 1;
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
                    shutdown_rx.clone(),
                )
                .await;

                let clean = result.is_ok();
                let was_long = attempt_start.elapsed().as_secs() >= 10;
                match result {
                    Ok(()) => info!("[raw] disconnected cleanly"),
                    Err(e) => {
                        error!("[raw] disconnected (attempt {attempt}): {e}");
                        connected_tx.send(false).ok();
                    }
                }

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if was_long { backoff = 2; attempt = 1; }
        // ws client
                let sleep_secs = if clean && was_long { 1 } else { backoff };
                info!("[raw] reconnecting in {sleep_secs}s (attempt {attempt})");
                tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
                if !clean || !was_long { backoff = (backoff * 2).min(30); }
                attempt += 1;
            }
        });
    }

    fn spawn_ws_client(
        &self,
        ip: &str,
        printer_id: &str,
        password: &str,
        shutdown_rx: watch::Receiver<bool>,
    ) {
        let state = self.state.clone();
        let connected_tx = self.ws_connected_tx.clone();
        let state_changed_tx = self.state_changed_tx.clone();
        let ws_cmd_tx = self.ws_cmd_tx.clone();
        let running = self.running.clone();
        let pending_rpcs = self.pending_rpcs.clone();

        let ip = ip.to_string();
        let pid = printer_id.to_string();
        let password = password.to_string();

        tokio::spawn(async move {
            let mut backoff = 2u64;
            let mut attempt: u32 = 1;
            loop {
                // rotating client_id avoids stale "already registered" sessions
                let ws_client_id = format!("0clid{}", rand_hex(8));
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
                    shutdown_rx.clone(),
                    pending_rpcs.clone(),
                )
                .await;

                let clean = result.is_ok();
                let was_long = attempt_start.elapsed().as_secs() >= 10;
                match result {
                    Ok(()) => info!("[ws] disconnected cleanly"),
                    Err(e) => {
                        error!("[ws] disconnected (attempt {attempt}): {e}");
                        connected_tx.send(false).ok();
                    }
                }

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if was_long { backoff = 2; attempt = 1; }
                let sleep_secs = if clean && was_long { 1 } else { backoff };
                info!("[ws] reconnecting in {sleep_secs}s (attempt {attempt})");
                tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
                if !clean || !was_long { backoff = (backoff * 2).min(30); }
                attempt += 1;
            }
        });
    }

    pub async fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        let mut lc = self.lifecycle.lock().await;
        if let Some(tx) = lc.raw_shutdown.take() {
            tx.send(true).ok();
        }
        if let Some(tx) = lc.ws_shutdown.take() {
            tx.send(true).ok();
        }
        self.raw_connected_tx.send(false).ok();
        self.ws_connected_tx.send(false).ok();
        info!("printer manager stopped");
    }

    pub async fn update_config(&self, new_config: AppConfig) {
        if self.running.load(Ordering::SeqCst) {
            self.running.store(false, Ordering::SeqCst);
            let mut lc = self.lifecycle.lock().await;
            if let Some(tx) = lc.raw_shutdown.take() {
                tx.send(true).ok();
            }
            if let Some(tx) = lc.ws_shutdown.take() {
                tx.send(true).ok();
            }
        }
        let mut lc = self.lifecycle.lock().await;
        lc.config = new_config;
        lc.raw_client_id = format!("cc2_{}", rand_digits(4));
        lc.printer_id = lc.config.printer.printer_id.clone();
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
            Ok(Err(_)) => {
                // sender drop means ws died mid-rpc
                self.pending_rpcs.lock().await.remove(&id);
                Err(PrinterError::NotConnected)
            }
            Err(_) => {
                self.pending_rpcs.lock().await.remove(&id);
                Err(PrinterError::RpcTimeout)
            }
        }
    }

    // send command and check error_code from response
    async fn rpc_cmd(&self, method: u16, params: Option<serde_json::Value>, timeout_secs: u64) -> Result<(), PrinterError> {
        let val = self.rpc_call(method, params, timeout_secs).await?;
        let code = val.get("error_code").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        if code != 0 {
            return Err(PrinterError::CommandFailed { method, error_code: code });
        }
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), PrinterError> {
        info!("[cmd] pause_print");
        self.rpc_cmd(METHOD_PAUSE_PRINT, None, 8).await?;
        self.state.write().await.add_event(EventKind::CommandPause, "Print paused".to_string());
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), PrinterError> {
        info!("[cmd] resume_print");
        self.rpc_cmd(METHOD_RESUME_PRINT, None, 8).await?;
        self.state.write().await.add_event(EventKind::CommandResume, "Print resumed".to_string());
        Ok(())
    }

    pub async fn stop_print(&self) -> Result<(), PrinterError> {
        info!("[cmd] stop_print");
        self.rpc_cmd(METHOD_STOP_PRINT, None, 8).await?;
        self.state.write().await.add_event(EventKind::CommandStop, "Print stopped".to_string());
        Ok(())
    }

    pub async fn set_led(&self, power: bool) -> Result<(), PrinterError> {
        info!("[cmd] set_led power={power}");
        self.rpc_cmd(METHOD_SET_LED, Some(serde_json::json!({ "power": if power { 1i64 } else { 0i64 } })), 5).await?;
        self.state.write().await.add_event(
            EventKind::CommandLed,
            format!("LED {}", if power { "on" } else { "off" }),
        );
        Ok(())
    }

    pub async fn set_fan(&self, name: &str, speed: u8) -> Result<(), PrinterError> {
        info!("[cmd] set_fan name={name} speed={speed}");
        self.rpc_cmd(METHOD_SET_FAN, Some(serde_json::json!({ name: speed as i64 })), 5).await?;
        self.state.write().await.add_event(
            EventKind::CommandFan,
            format!("Fan '{}' → {}", name, speed),
        );
        Ok(())
    }

    pub async fn set_speed_mode(&self, mode: u8) -> Result<(), PrinterError> {
        info!("[cmd] set_speed_mode mode={mode}");
        self.rpc_cmd(METHOD_SET_SPEED_MODE, Some(serde_json::json!({ "mode": mode as i64 })), 5).await?;
        self.state.write().await.add_event(
            EventKind::CommandSpeedMode,
            format!("Speed mode → {}", mode),
        );
        Ok(())
    }

    pub async fn home_axes(&self, axes: &str) -> Result<(), PrinterError> {
        info!("[cmd] home_axes axes={axes}");
        self.rpc_cmd(METHOD_HOME_AXES, Some(serde_json::json!({ "homed_axes": axes })), 15).await
    }

    pub async fn jog_axis(&self, axis: &str, distance: f64) -> Result<(), PrinterError> {
        info!("[cmd] jog_axis axis={axis} distance={distance}");
        self.rpc_cmd(METHOD_JOG_AXIS, Some(serde_json::json!({ "axes": axis, "distance": distance })), 10).await
    }

    pub async fn start_print(
        &self,
        filename: &str,
        storage_media: &str,
        plate: &str,
        tray_id: Option<i64>,
        tray_slot: Option<i64>,
        canvas_id: i64,
        timelapse: bool,
        bedlevel_force: bool,
    ) -> Result<(), PrinterError> {
        let tray_id = tray_id.unwrap_or(0);
        let t = tray_slot.unwrap_or(0);
        info!("[cmd] start_print filename={filename} plate={plate} canvas={canvas_id} t={t} tray={tray_id}");
        let print_layout = if plate == "smooth" { "B" } else { "A" };
        self.rpc_cmd(
            METHOD_START_PRINT,
            Some(serde_json::json!({
                "filename": filename,
                "storage_media": storage_media,
                "config": {
                    "bedlevel_force": bedlevel_force,
                    "delay_video": timelapse,
                    "print_layout": print_layout,
                    "printer_check": true,
                    "slot_map": [{"canvas_id": canvas_id, "t": t, "tray_id": tray_id}]
                }
            })),
            15,
        ).await?;
        self.state.write().await.add_event(
            EventKind::CommandStartPrint,
            format!("Start print: {}", filename),
        );
        Ok(())
    }

    pub async fn set_ams_auto_refill(&self, enabled: bool) -> Result<(), PrinterError> {
        info!("[cmd] set_ams_auto_refill enabled={enabled}");
        self.rpc_cmd(METHOD_SET_AMS_AUTO_REFILL, Some(serde_json::json!({ "auto_refill": enabled })), 5).await
    }

    pub async fn get_file_list(&self, storage: &str, page_number: i64, page_size: i64) -> Result<serde_json::Value, PrinterError> {
        // drop concurrent request  -  caller gets current cached list
        let _guard = match self.file_list_lock.try_lock() {
            Ok(g) => g,
            Err(_) => {
                let files = self.state.read().await.files.clone();
                return Ok(serde_json::json!(files));
            }
        };

        let t0 = std::time::Instant::now();
        info!("[cmd] get_file_list storage={storage} pageNumber={page_number} pageSize={page_size}");

        let result = self.rpc_call(
            METHOD_GET_FILE_LIST,
            Some(serde_json::json!({
                "storage_media": storage,
                "pageNumber": page_number,
                "pageSize": page_size,
            })),
            15,
        ).await;

        let elapsed = t0.elapsed().as_millis();

        let data = match result {
            Ok(d) => d,
            Err(e) => {
                info!("[cmd] get_file_list failed after {elapsed}ms: {e}");
                return Err(e);
            }
        };

        let code = data.get("error_code").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        if code != 0 {
            info!("[cmd] get_file_list error_code={code} after {elapsed}ms");
            return Err(PrinterError::CommandFailed { method: METHOD_GET_FILE_LIST, error_code: code });
        }

        let files = data.get("file_list")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        info!("[cmd] get_file_list ok: {} files in {elapsed}ms", files.len());
        Ok(serde_json::json!(files))
    }

    pub async fn get_file_info(&self, storage_media: &str, file_name: &str) -> Result<serde_json::Value, PrinterError> {
        info!("[cmd] get_file_info filename={file_name}");
        self.rpc_call(
            METHOD_GET_FILE_INFO,
            Some(serde_json::json!({ "storage_media": storage_media, "file_name": file_name })),
            10,
        ).await
    }

    pub async fn get_print_history(&self) -> Result<serde_json::Value, PrinterError> {
        info!("[cmd] get_print_history");
        self.rpc_call(METHOD_GET_PRINT_HISTORY, None, 10).await
    }

    pub async fn get_file_thumbnail(&self, storage_media: &str, file_name: &str) -> Result<serde_json::Value, PrinterError> {
        info!("[cmd] get_file_thumbnail filename={file_name}");
        self.rpc_call(
            METHOD_GET_FILE_THUMBNAIL,
            Some(serde_json::json!({ "storage_media": storage_media, "file_name": file_name })),
            10,
        ).await
    }

    pub async fn canvas_refresh(&self) -> Result<serde_json::Value, PrinterError> {
        info!("[cmd] canvas_refresh");
        self.rpc_call(METHOD_GET_AMS_INFO, Some(serde_json::json!({})), 10).await
    }

    pub async fn printer_id(&self) -> String {
        self.lifecycle.lock().await.printer_id.clone()
    }

    pub async fn printer_ip(&self) -> String {
        self.lifecycle.lock().await.config.printer.ip.clone()
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
