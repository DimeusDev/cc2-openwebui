use std::net::SocketAddr;
use std::process::Command;

use axum::extract::State;
use axum::Json;
use futures::StreamExt;
use rand::Rng;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

use super::router::AppState;
use crate::config::validate_pincode;
use crate::error::{AppError, SetupError};

const SCAN_TIMEOUT_MS: u64 = 500;

#[derive(Serialize)]
pub struct SetupCheckResponse {
    pub configured: bool,
    pub onboarding_complete: bool,
}

pub async fn check_setup(State(state): State<AppState>) -> Result<Json<SetupCheckResponse>, AppError> {
    let config = state.config.read().await;
    Ok(Json(SetupCheckResponse {
        configured: !config.printer.ip.is_empty(),
        onboarding_complete: config.onboarding_complete,
    }))
}

#[derive(Serialize)]
pub struct HostOsResponse {
    pub os: &'static str,
    pub arch: &'static str,
    pub docker_command: String,
    pub gpu_supported: bool,
}

pub async fn host_os() -> Json<HostOsResponse> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let docker_command = build_docker_command(os);
    Json(HostOsResponse {
        os,
        arch,
        docker_command,
        gpu_supported: false,
    })
}

fn build_docker_command(os: &str) -> String {
    match os {
        "linux" | "macos" => {
            "docker run -d --name obico-ml --restart unless-stopped \\\n  \
             --network host \\\n  \
             ghcr.io/thespaghettidetective/ml_api:latest".to_string()
        }
        _ => String::new(),
    }
}

#[derive(Deserialize)]
pub struct CompleteOnboardingRequest {
    pub detection: Option<OnboardingDetection>,
    pub notifications: Option<OnboardingNotifications>,
}

#[derive(Deserialize)]
pub struct OnboardingDetection {
    pub obico_url: Option<String>,
    pub notify_threshold: Option<f64>,
    pub pause_threshold: Option<f64>,
}

#[derive(Deserialize)]
pub struct OnboardingNotifications {
    pub destinations: Option<Vec<crate::config::NotificationDestination>>,
}

pub async fn complete_onboarding(
    State(state): State<AppState>,
    Json(req): Json<CompleteOnboardingRequest>,
) -> Result<Json<Value>, AppError> {
    let mut config = state.config.write().await;

    if let Some(d) = req.detection {
        if let Some(url) = d.obico_url {
            config.detection.obico_url = url;
        }
        if let Some(v) = d.notify_threshold {
            config.detection.notify_threshold = v.clamp(0.0, 1.0);
        }
        if let Some(v) = d.pause_threshold {
            config.detection.pause_threshold = v.clamp(0.0, 1.0);
        }
    }

    if let Some(n) = req.notifications {
        if let Some(dests) = n.destinations {
            config.notifications.destinations = dests;
        }
    }

    config.onboarding_complete = true;

    if let Err(e) = config.save("config.toml") {
        warn!("failed to persist onboarding config: {e}");
        return Err(AppError::Config(crate::error::ConfigError::Load(
            config::ConfigError::Message(format!("failed to save onboarding: {e}")),
        )));
    }

    info!("onboarding complete + config saved");
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn reset_setup(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    {
        let mut manager = state.manager.lock().await;
        manager.shutdown().await;
    }

    let defaults = crate::config::AppConfig::default();

    {
        let mut config = state.config.write().await;
        *config = defaults.clone();
        if let Err(e) = config.save("config.toml") {
            warn!("failed to persist reset config: {e}");
            return Err(AppError::Config(crate::error::ConfigError::Load(
                config::ConfigError::Message(format!("failed to save reset: {e}")),
            )));
        }
    }

    let _ = state.det_enabled_tx.send(defaults.detection.enabled);
    let _ = state.det_config_tx.send(defaults.detection.clone());

    {
        let mut manager = state.manager.lock().await;
        manager.update_config(defaults);
    }

    info!("all settings reset to defaults");
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub subnet: Option<String>,
}

#[derive(Serialize)]
pub struct ScanResponse {
    pub printers: Vec<DiscoveredPrinter>,
}

#[derive(Serialize)]
pub struct DiscoveredPrinter {
    pub ip: String,
}

pub async fn scan_network(
    _state: State<AppState>,
    Json(req): Json<Option<ScanRequest>>,
) -> Result<Json<ScanResponse>, AppError> {
    let subnets: Vec<String> = if let Some(subnet) = req.and_then(|r| r.subnet) {
        let parts: Vec<&str> = subnet.split('.').collect();
        if parts.len() >= 3 {
            vec![parts[0..3].join(".")]
        } else {
            detect_local_subnets()
        }
    } else {
        detect_local_subnets()
    };

    const MAX_CONCURRENT: usize = 80;
    const VERIFY_CONCURRENT: usize = 10;

    let ips: Vec<String> = subnets.iter()
        .flat_map(|prefix| (1..=254u32).map(move |i| format!("{}.{}", prefix, i)))
        .collect();

    info!("scanning {} addresses across {} subnet(s)", ips.len(), subnets.len());

    // phase 1: probe 1883/9001/80 and keep reachable hosts
    let candidates: Vec<String> = futures::stream::iter(ips)
        .map(|ip| async move {
            for port in [1883u16, 9001, 80] {
                let addr: SocketAddr = format!("{ip}:{port}").parse().ok()?;
                if let Ok(Ok(_)) = timeout(Duration::from_millis(SCAN_TIMEOUT_MS), TcpStream::connect(addr)).await {
                    return Some(ip);
                }
            }
            None
        })
        .buffer_unordered(MAX_CONCURRENT)
        .filter_map(|r| async move { r })
        .collect()
        .await;

    info!("port probe found {} candidates, running protocol verification", candidates.len());

    // phase 2: verify elegoo mqtt handshake
    let mut printers: Vec<DiscoveredPrinter> = futures::stream::iter(candidates)
        .map(|ip| async move {
            let verified = verify_elegoo_mqtt(&ip, 3).await;
            if verified {
                Some(DiscoveredPrinter { ip })
            } else {
                None
            }
        })
        .buffer_unordered(VERIFY_CONCURRENT)
        .filter_map(|r| async move { r })
        .collect()
        .await;

    printers.sort_by(|a, b| a.ip.cmp(&b.ip));
    info!("network scan confirmed {} printers", printers.len());
    Ok(Json(ScanResponse { printers }))
}

/// Attempts to connect to MQTT on port 1883 and listens for an elegoo topic message.
async fn verify_elegoo_mqtt(ip: &str, timeout_secs: u64) -> bool {
    let suffix: String = (0..4).map(|_| rand::thread_rng().gen_range(0..10u8).to_string()).collect();
    let client_id = format!("cc2_scan_{suffix}");

    let mut opts = MqttOptions::new(&client_id, ip, 1883);
    opts.set_credentials("elegoo", "123456");
    opts.set_keep_alive(Duration::from_secs(5));
    opts.set_clean_session(true);

    let (client, mut eventloop) = AsyncClient::new(opts, 4);
    if client.subscribe("elegoo/+/api_status", QoS::AtMostOnce).await.is_err() {
        return false;
    }

    let result = timeout(Duration::from_secs(timeout_secs), async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    if p.topic.starts_with("elegoo/") {
                        return true;
                    }
                }
                Ok(_) => {}
                Err(_) => return false,
            }
        }
    })
    .await;

    client.disconnect().await.ok();
    matches!(result, Ok(true))
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub ip: String,
    pub pincode: Option<String>,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub printer_id: String,
    pub model: String,
}

pub async fn verify_printer(
    _state: State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    let password = match req.pincode.as_deref() {
        Some(p) if !p.is_empty() => {
            validate_pincode(p).map_err(|_| AppError::Setup(SetupError::InvalidPincode))?;
            p.to_string()
        }
        _ => "123456".to_string(),
    };

    let printer_id = crate::printer::discovery::discover_printer_id(&req.ip, "elegoo", &password, 5)
        .await
        .map_err(|e| {
            if matches!(e, crate::error::PrinterError::DiscoveryTimeout(_)) {
                AppError::Setup(SetupError::VerificationFailed(
                    "No response from printer. Check the IP address and that the printer is on.".to_string(),
                ))
            } else {
                AppError::Setup(SetupError::VerificationFailed(format!("Connection failed: {e}")))
            }
        })?;

    info!("verified printer {printer_id} at {}", req.ip);
    Ok(Json(VerifyResponse {
        success: true,
        printer_id,
        model: "Centauri Carbon 2".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct SaveConfigRequest {
    pub ip: String,
    pub printer_id: String,
    pub pincode: Option<String>,
}

pub async fn save_config(
    State(state): State<AppState>,
    Json(req): Json<SaveConfigRequest>,
) -> Result<Json<Value>, AppError> {
    if req.ip.is_empty() {
        return Err(AppError::Setup(SetupError::VerificationFailed(
            "IP address is required".to_string(),
        )));
    }
    if req.printer_id.is_empty() {
        return Err(AppError::Setup(SetupError::VerificationFailed(
            "Printer ID is required".to_string(),
        )));
    }

    let pincode = req.pincode.unwrap_or_default();
    if !pincode.is_empty() {
        validate_pincode(&pincode).map_err(|_| AppError::Setup(SetupError::InvalidPincode))?;
    }

    {
        let mut config = state.config.write().await;
        config.printer.ip = req.ip.clone();
        config.printer.printer_id = req.printer_id.clone();
        config.printer.pincode = pincode;
        if let Err(e) = config.save("config.toml") {
            error!("failed to persist config to disk: {e}");
            return Err(AppError::Config(crate::error::ConfigError::Load(
                config::ConfigError::Message(format!("failed to save config: {e}")),
            )));
        }
        info!("config saved to config.toml");
    }

    let config_snapshot = state.config.read().await.clone();
    let mut manager = state.manager.lock().await;
    manager.update_config(config_snapshot);

    if let Err(e) = manager.start().await {
        error!("failed to start printer manager after setup: {e}");
        return Err(AppError::Setup(SetupError::VerificationFailed(
            format!("Failed to start printer connection: {e}"),
        )));
    }

    info!("printer manager started after setup for {}", req.ip);
    Ok(Json(serde_json::json!({ "success": true })))
}

fn detect_local_subnets() -> Vec<String> {
    let mut subnets = Vec::new();

    // Parse `ip addr` to get all assigned interface addresses (more reliable than ip route).
    if let Ok(output) = Command::new("ip").args(["addr"]).output() {
        if let Ok(text) = String::from_utf8(output.stdout) {
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("inet ") && !trimmed.contains("127.0.0.1") {
                    if let Some(cidr) = trimmed.split_whitespace().nth(1) {
                        let ip_part = cidr.split('/').next().unwrap_or("");
                        let parts: Vec<&str> = ip_part.split('.').collect();
                        if parts.len() == 4 {
                            let subnet = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
                            if !subnets.contains(&subnet) {
                                subnets.push(subnet);
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: parse `ip route` for the route associated with the default gateway's interface.
    if subnets.is_empty() {
        if let Ok(output) = Command::new("ip").args(["route"]).output() {
            if let Ok(routes) = String::from_utf8(output.stdout) {
                for line in routes.lines() {
                    if !line.starts_with("default") {
                        if let Some(prefix) = line.split_whitespace().next() {
                            let ip_str = prefix.split('/').next().unwrap_or("");
                            let parts: Vec<&str> = ip_str.split('.').collect();
                            if parts.len() == 4 {
                                let subnet = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
                                if !subnets.contains(&subnet) {
                                    subnets.push(subnet);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if subnets.is_empty() {
        subnets.extend_from_slice(&[
            "192.168.1".to_string(),
            "192.168.0".to_string(),
            "192.168.129".to_string(),
            "10.0.0".to_string(),
        ]);
    }

    info!("detected local subnets: {:?}", subnets);
    subnets
}
