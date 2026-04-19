use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub printer: PrinterConfig,
    pub detection: DetectionConfig,
    pub notifications: NotificationsConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub onboarding_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub ip: String,
    pub printer_id: String,
    pub pincode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcludeZone {
    /// Normalized coordinate (0..1).
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl ExcludeZone {
    pub fn contains_center(&self, det_x1: f64, det_y1: f64, det_x2: f64, det_y2: f64) -> bool {
        let cx = (det_x1 + det_x2) / 2.0;
        let cy = (det_y1 + det_y2) / 2.0;
        cx >= self.x1 && cx <= self.x2 && cy >= self.y1 && cy <= self.y2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    pub enabled: bool,
    pub interval_secs: u32,
    /// Rolling-average threshold that triggers a notification.
    #[serde(default = "default_notify_threshold")]
    pub notify_threshold: f64,
    /// Rolling-average threshold that triggers a notification + auto-pause.
    /// Must be >= notify_threshold. Old `threshold` field migrates here.
    #[serde(alias = "threshold", default = "default_pause_threshold")]
    pub pause_threshold: f64,
    pub confirmation_frames: u32,
    pub obico_url: String,
    #[serde(default)]
    pub exclude_zones: Vec<ExcludeZone>,
}

fn default_notify_threshold() -> f64 { 0.5 }
fn default_pause_threshold() -> f64 { 0.7 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DestinationKind {
    Ntfy,
    Discord,
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventToggles {
    #[serde(default = "default_true")]
    pub print_started: bool,
    #[serde(default = "default_true")]
    pub print_finished: bool,
    #[serde(default)]
    pub print_paused: bool,
    #[serde(default = "default_true")]
    pub failure_notify: bool,
    #[serde(default = "default_true")]
    pub failure_pause: bool,
    #[serde(default = "default_true")]
    pub auto_paused: bool,
}

fn default_true() -> bool { true }

impl Default for EventToggles {
    fn default() -> Self {
        Self {
            print_started: true,
            print_finished: true,
            print_paused: false,
            failure_notify: true,
            failure_pause: true,
            auto_paused: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDestination {
    pub id: String,
    pub kind: DestinationKind,
    pub enabled: bool,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntfy_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntfy_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub toggles: EventToggles,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationsConfig {
    #[serde(default)]
    pub destinations: Vec<NotificationDestination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

impl AppConfig {
    pub fn load(path: Option<&str>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        if let Some(p) = path {
            builder = builder.add_source(File::with_name(p).required(false));
        } else {
            builder = builder.add_source(File::with_name("config").required(false));
        }

        builder = builder.add_source(Environment::with_prefix("CC2").separator("_"));

        let config = builder.build().map_err(ConfigError::Load)?;
        let app: Self = config.try_deserialize().map_err(ConfigError::Load)?;

        app.validate()?;
        Ok(app)
    }

    pub fn load_or_default() -> (Self, bool) {
        match Self::load(None) {
            Ok(c) => {
                let configured = !c.printer.ip.is_empty();
                (c, configured)
            }
            Err(e) => {
                eprintln!("warn: config parse/validation error, starting with defaults: {e}");
                (Self::default(), false)
            }
        }
    }

    pub fn save(&self, path: &str) -> Result<(), ConfigError> {
        let toml = toml::to_string_pretty(self).map_err(|e| {
            ConfigError::Load(config::ConfigError::Message(e.to_string()))
        })?;
        std::fs::write(path, toml).map_err(|e| {
            ConfigError::Load(config::ConfigError::Message(e.to_string()))
        })?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if !self.printer.ip.is_empty() {
            validate_ip(&self.printer.ip)?;
        }
        if !self.printer.pincode.is_empty() {
            validate_pincode(&self.printer.pincode)?;
        }
        if !(0.0..=1.0).contains(&self.detection.notify_threshold) {
            return Err(ConfigError::InvalidThreshold);
        }
        if !(0.0..=1.0).contains(&self.detection.pause_threshold) {
            return Err(ConfigError::InvalidThreshold);
        }
        if self.detection.pause_threshold < self.detection.notify_threshold {
            return Err(ConfigError::InvalidThreshold);
        }
        Ok(())
    }

    pub fn printer_password(&self) -> &str {
        if !self.printer.pincode.is_empty() {
            &self.printer.pincode
        } else {
            "123456"
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            printer: PrinterConfig {
                ip: String::new(),
                printer_id: String::new(),
                pincode: String::new(),
            },
            detection: DetectionConfig {
                enabled: true,
                interval_secs: 15,
                notify_threshold: 0.5,
                pause_threshold: 0.7,
                confirmation_frames: 2,
                obico_url: "http://localhost:3333".to_string(),
                exclude_zones: Vec::new(),
            },
            notifications: NotificationsConfig {
                destinations: Vec::new(),
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8484,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
            onboarding_complete: false,
        }
    }
}

fn validate_ip(ip: &str) -> Result<(), ConfigError> {
    ip.parse::<std::net::Ipv4Addr>()
        .map_err(|_| ConfigError::InvalidIp(ip.to_string()))?;
    Ok(())
}

/// Pincode: exactly 6 ASCII alphanumeric chars, case-sensitive.
pub fn validate_pincode(p: &str) -> Result<(), ConfigError> {
    if p.len() != 6 || !p.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ConfigError::InvalidPincode);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn settings_round_trip_toml() {
        let mut cfg = base_config();
        cfg.printer.ip = "192.168.1.50".to_string();
        cfg.printer.printer_id = "TESTID".to_string();
        cfg.detection.notify_threshold = 0.4;
        cfg.detection.pause_threshold = 0.6;
        cfg.onboarding_complete = true;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        cfg.save(path.to_str().unwrap()).unwrap();

        let loaded = AppConfig::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(loaded.printer.ip, "192.168.1.50");
        assert_eq!(loaded.printer.printer_id, "TESTID");
        assert!((loaded.detection.notify_threshold - 0.4).abs() < 1e-9);
        assert!((loaded.detection.pause_threshold - 0.6).abs() < 1e-9);
        assert!(loaded.onboarding_complete);
    }

    #[test]
    fn settings_round_trip_notification_destination() {
        let mut cfg = base_config();
        cfg.notifications.destinations.push(NotificationDestination {
            id: "dest1".to_string(),
            kind: DestinationKind::Ntfy,
            enabled: true,
            label: "My Phone".to_string(),
            ntfy_server: Some("https://ntfy.sh".to_string()),
            ntfy_topic: Some("cc2-alerts".to_string()),
            discord_webhook_url: None,
            webhook_url: None,
            toggles: EventToggles::default(),
        });

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        cfg.save(path.to_str().unwrap()).unwrap();

        let loaded = AppConfig::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(loaded.notifications.destinations.len(), 1);
        let dest = &loaded.notifications.destinations[0];
        assert_eq!(dest.label, "My Phone");
        assert_eq!(dest.kind, DestinationKind::Ntfy);
        assert_eq!(dest.ntfy_topic.as_deref(), Some("cc2-alerts"));
    }

    #[test]
    fn validate_pincode_accepts_valid() {
        assert!(validate_pincode("ABC123").is_ok());
        assert!(validate_pincode("000000").is_ok());
        assert!(validate_pincode("ZZZZZZ").is_ok());
    }

    #[test]
    fn validate_pincode_rejects_short_or_invalid() {
        assert!(validate_pincode("ABC12").is_err());
        assert!(validate_pincode("ABC1234").is_err());
        assert!(validate_pincode("AB-C12").is_err());
        assert!(validate_pincode("AB C12").is_err());
        assert!(validate_pincode("ABC12!").is_err());
    }

    #[test]
    fn validate_thresholds_rejects_inverted() {
        let mut cfg = base_config();
        cfg.printer.ip = String::new();
        cfg.detection.notify_threshold = 0.8;
        cfg.detection.pause_threshold = 0.6;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        cfg.save(path.to_str().unwrap()).unwrap();

        let result = AppConfig::load(Some(path.to_str().unwrap()));
        assert!(result.is_err(), "inverted thresholds must be rejected");
    }

    #[test]
    fn printer_password_uses_pincode_when_set() {
        let mut cfg = base_config();
        cfg.printer.pincode = "ABC123".to_string();
        assert_eq!(cfg.printer_password(), "ABC123");
    }

    #[test]
    fn printer_password_defaults_to_123456() {
        let cfg = base_config();
        assert_eq!(cfg.printer_password(), "123456");
    }

    #[test]
    fn exclude_zone_contains_center() {
        let zone = ExcludeZone { x1: 0.2, y1: 0.2, x2: 0.8, y2: 0.8 };
        assert!(zone.contains_center(0.3, 0.3, 0.7, 0.7));
        assert!(!zone.contains_center(0.0, 0.0, 0.1, 0.1));
        assert!(!zone.contains_center(0.85, 0.85, 0.95, 0.95));
    }
}
