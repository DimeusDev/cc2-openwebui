use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("printer error: {0}")]
    Printer(#[from] PrinterError),

    #[error("detection error: {0}")]
    Detection(#[from] DetectionError),

    #[error("notification error: {0}")]
    Notification(#[from] NotificationError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("setup error: {0}")]
    Setup(#[from] SetupError),

    #[error("validation error: {0}")]
    Validation(String),
}

#[derive(Error, Debug)]
pub enum SetupError {
    #[error("connection verification failed: {0}")]
    VerificationFailed(String),

    #[error("invalid pincode: must be 6 uppercase alphanumeric characters")]
    InvalidPincode,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::Printer(PrinterError::NotConnected) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Printer(PrinterError::RpcTimeout) => StatusCode::GATEWAY_TIMEOUT,
            AppError::Printer(PrinterError::CommandFailed { .. }) => StatusCode::BAD_REQUEST,
            AppError::Config(_) => StatusCode::BAD_REQUEST,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Setup(SetupError::InvalidPincode) => StatusCode::BAD_REQUEST,
            AppError::Setup(SetupError::VerificationFailed(_)) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(serde_json::json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to load config: {0}")]
    Load(#[source] config::ConfigError),

    #[error("invalid printer IP address: {0}")]
    InvalidIp(String),

    #[error("invalid pincode: must be 6 uppercase characters")]
    InvalidPincode,

    #[error("invalid detection threshold: must be between 0.0 and 1.0")]
    InvalidThreshold,
}

#[derive(Error, Debug)]
pub enum PrinterError {
    #[error("MQTT error: {0}")]
    Mqtt(#[from] rumqttc::ConnectionError),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("registration failed: {0}")]
    Registration(String),

    #[error("printer not connected")]
    NotConnected,

    // Returned when the printer acknowledges a command with a non-zero error code.
    #[allow(dead_code)]
    #[error("command failed: method {method}, error_code {error_code}")]
    CommandFailed { method: u16, error_code: u16 },

    #[error("discovery timed out after {0}s")]
    DiscoveryTimeout(u64),

    #[error("printer RPC timed out")]
    RpcTimeout,

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum DetectionError {
    #[error("Obico ML request failed: {0}")]
    ObicoFailed(String),

    #[error("detection engine not running")]
    NotRunning,
}

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("ntfy notification failed: {0}")]
    NtfyFailed(String),
    #[error("discord notification failed: {0}")]
    DiscordFailed(String),
}
