use bollard::container::{Config, CreateContainerOptions, StartContainerOptions};
use bollard::image::CreateImageOptions;
use bollard::models::{ContainerStateStatusEnum, HostConfig, PortBinding};
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

const OBICO_IMAGE: &str = "ghcr.io/thespaghettidetective/ml_api:latest";
const OBICO_CONTAINER: &str = "obico-ml";
pub const OBICO_PORT: u16 = 3333;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObicoStatus {
    /// Docker daemon not available on this host.
    Unavailable,
    /// Container does not exist or has never been created.
    NotCreated,
    /// Container exists but is not running.
    Stopped,
    /// Container is running.
    Running,
}

fn connect() -> Option<Docker> {
    Docker::connect_with_local_defaults().ok()
}

pub async fn status() -> ObicoStatus {
    let docker = match connect() {
        Some(d) => d,
        None => return ObicoStatus::Unavailable,
    };

    match docker.inspect_container(OBICO_CONTAINER, None).await {
        Ok(info) => {
            let running = info
                .state
                .as_ref()
                .and_then(|s| s.status.as_ref())
                .map(|s| *s == ContainerStateStatusEnum::RUNNING)
                .unwrap_or(false);
            if running {
                ObicoStatus::Running
            } else {
                ObicoStatus::Stopped
            }
        }
        Err(_) => ObicoStatus::NotCreated,
    }
}

/// Pull image (if needed), create container (if not exists), start it.
/// Returns once the container is running or on error.
pub async fn start() -> Result<(), String> {
    let docker = connect().ok_or("Docker daemon not available")?;

    pull_image_if_needed(&docker).await?;

    // Create container if it doesn't exist
    if let Err(_) = docker.inspect_container(OBICO_CONTAINER, None).await {
        create_container(&docker).await?;
    }

    docker
        .start_container(OBICO_CONTAINER, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| format!("Failed to start container: {e}"))?;

    info!("[docker] obico-ml container started");
    Ok(())
}

pub async fn stop() -> Result<(), String> {
    let docker = connect().ok_or("Docker daemon not available")?;

    docker
        .stop_container(OBICO_CONTAINER, None)
        .await
        .map_err(|e| format!("Failed to stop container: {e}"))?;

    info!("[docker] obico-ml container stopped");
    Ok(())
}

/// Poll until the ML container's port accepts TCP connections or timeout expires.
pub async fn wait_for_ready(timeout_secs: u64) -> bool {
    let addr = format!("127.0.0.1:{OBICO_PORT}");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    false
}

/// Start Docker container with real-time progress sent over a channel.
/// Channel items are (event_type, message): event_type is "log", "ready", or "fail".
pub async fn start_with_progress_channel(tx: tokio::sync::mpsc::Sender<(String, String)>) -> Result<(), String> {
    let docker = connect().ok_or_else(|| "Docker daemon not available".to_string())?;

    // Pull image if not present
    if docker.inspect_image(OBICO_IMAGE).await.is_err() {
        let _ = tx.send(("log".into(), format!("Pulling {}…", OBICO_IMAGE))).await;
        let options = CreateImageOptions { from_image: OBICO_IMAGE, ..Default::default() };
        let mut stream = docker.create_image(Some(options), None, None);
        let mut last_key = String::new();
        while let Some(item) = stream.next().await {
            match item {
                Ok(info) => {
                    if let Some(status) = &info.status {
                        if status == "Downloading" || status == "Extracting" {
                            continue;
                        }
                        let key = format!("{}:{}", info.id.as_deref().unwrap_or(""), status);
                        if key != last_key {
                            last_key = key;
                            let msg = match &info.id {
                                Some(id) => format!("{}: {}", id.get(..12).unwrap_or(id), status),
                                None => status.clone(),
                            };
                            let _ = tx.send(("log".into(), msg)).await;
                        }
                    }
                }
                Err(e) => {
                    warn!("[docker] pull error: {e}");
                    return Err(format!("Image pull failed: {e}"));
                }
            }
        }
        let _ = tx.send(("log".into(), "Image ready.".into())).await;
    } else {
        let _ = tx.send(("log".into(), "Image already present, skipping pull.".into())).await;
    }

    // Create container if absent
    if docker.inspect_container(OBICO_CONTAINER, None).await.is_err() {
        let _ = tx.send(("log".into(), "Creating container…".into())).await;
        create_container(&docker).await?;
    }

    // Start container (ignore "already started")
    let _ = tx.send(("log".into(), "Starting container…".into())).await;
    match docker.start_container(OBICO_CONTAINER, None::<StartContainerOptions<String>>).await {
        Ok(()) => {}
        Err(e) if e.to_string().to_lowercase().contains("already") => {}
        Err(e) => return Err(format!("Failed to start container: {e}")),
    }

    info!("[docker] obico-ml container started via progress channel");
    Ok(())
}

async fn pull_image_if_needed(docker: &Docker) -> Result<(), String> {
    // Check if image exists locally
    if docker.inspect_image(OBICO_IMAGE).await.is_ok() {
        return Ok(());
    }

    info!("[docker] pulling obico-ml image...");
    let options = CreateImageOptions {
        from_image: OBICO_IMAGE,
        ..Default::default()
    };

    let mut stream = docker.create_image(Some(options), None, None);
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => {
                warn!("[docker] pull error: {e}");
                return Err(format!("Image pull failed: {e}"));
            }
        }
    }
    info!("[docker] obico-ml image ready");
    Ok(())
}

async fn create_container(docker: &Docker) -> Result<(), String> {
    let port_str = format!("{}/tcp", OBICO_PORT);
    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        port_str.clone(),
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some(OBICO_PORT.to_string()),
        }]),
    );

    let mut exposed_ports: HashMap<&str, HashMap<(), ()>> = HashMap::new();
    exposed_ports.insert(port_str.as_str(), HashMap::new());

    let config = Config {
        image: Some(OBICO_IMAGE),
        exposed_ports: Some(exposed_ports),
        host_config: Some(HostConfig {
            port_bindings: Some(port_bindings),
            restart_policy: Some(bollard::models::RestartPolicy {
                name: Some(bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED),
                maximum_retry_count: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: OBICO_CONTAINER,
                platform: None,
            }),
            config,
        )
        .await
        .map_err(|e| format!("Failed to create container: {e}"))?;

    info!("[docker] obico-ml container created");
    Ok(())
}
