use axum::extract::{Path, State};
use axum::Json;
use serde_json::Value;
use tracing::warn;

use super::router::AppState;
use crate::config::{DestinationKind, NotificationDestination};
use crate::error::AppError;
use crate::notifications::{discord, ntfy};

fn gen_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:08x}", rng.gen::<u32>())
}

fn config_err(e: impl std::fmt::Display) -> AppError {
    AppError::Config(crate::error::ConfigError::Load(
        config::ConfigError::Message(e.to_string()),
    ))
}

pub async fn list_destinations(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let config = state.config.read().await;
    Ok(Json(
        serde_json::to_value(&config.notifications.destinations).unwrap_or(Value::Array(vec![])),
    ))
}

pub async fn create_destination(
    State(state): State<AppState>,
    Json(mut dest): Json<NotificationDestination>,
) -> Result<Json<Value>, AppError> {
    dest.id = gen_id();
    let mut config = state.config.write().await;
    let id = dest.id.clone();
    config.notifications.destinations.push(dest);
    if let Err(e) = config.save("config.toml") {
        warn!("failed to save config: {e}");
        return Err(config_err(e));
    }
    Ok(Json(serde_json::json!({ "success": true, "id": id })))
}

pub async fn update_destination(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<NotificationDestination>,
) -> Result<Json<Value>, AppError> {
    let mut config = state.config.write().await;
    let dest = config
        .notifications
        .destinations
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or_else(|| AppError::Validation(format!("destination {id} not found")))?;
    *dest = NotificationDestination { id: id.clone(), ..req };
    if let Err(e) = config.save("config.toml") {
        warn!("failed to save config: {e}");
        return Err(config_err(e));
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn delete_destination(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let mut config = state.config.write().await;
    let before = config.notifications.destinations.len();
    config.notifications.destinations.retain(|d| d.id != id);
    if config.notifications.destinations.len() == before {
        return Err(AppError::Validation(format!("destination {id} not found")));
    }
    if let Err(e) = config.save("config.toml") {
        warn!("failed to save config: {e}");
        return Err(config_err(e));
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn test_destination(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let config = state.config.read().await;
    let dest = config
        .notifications
        .destinations
        .iter()
        .find(|d| d.id == id)
        .cloned()
        .ok_or_else(|| AppError::Validation(format!("destination {id} not found")))?;
    drop(config);

    match dest.kind {
        DestinationKind::Ntfy => ntfy::send_test(&dest).await?,
        DestinationKind::Discord => discord::send_test(&dest).await?,
        DestinationKind::Webhook => {
            return Err(AppError::Validation(
                "webhook test not supported yet".to_string(),
            ));
        }
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
