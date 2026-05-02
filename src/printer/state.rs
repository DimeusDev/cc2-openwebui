use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::time::UNIX_EPOCH;

use serde_json::Value;

use super::models::{DeviceInfo, FullStatus};
use crate::detection::obico::Detection;

pub const EVENTS_LOG_PATH: &str = "data/events.log";
pub const DETECTION_LOG_PATH: &str = "data/detection.log";

#[derive(Debug, Clone, PartialEq)]
pub enum PrintState {
    Idle,
    Printing,
    Paused,
}

/// detection point
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectionPoint {
    pub ts: u64,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_filename: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub boxes: Vec<crate::detection::obico::Detection>,
}

#[derive(Debug, Clone)]
pub struct PrinterState {
    pub full: FullStatus,
    pub device_info: Option<DeviceInfo>,
    pub printer_ip: String,
    /// raw+ws connected
    pub connected: bool,
    pub connected_raw: bool,
    pub connected_ws: bool,
    pub camera_connected: bool,
    pub detection_score: f64,
    pub detection_history: VecDeque<DetectionPoint>,
    /// latest detections
    pub latest_detections: Vec<Detection>,
    pub latest_detection_ts: u64,
    pub events: Vec<PrinterEvent>,
    /// event total mono
    pub events_total: u64,
    pub files: Vec<Value>,
    pub thumbnail_cache: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PrinterEvent {
    pub timestamp: std::time::SystemTime,
    pub kind: EventKind,
    pub description: String,
    pub snapshot: Option<String>,
}

// debug names
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum EventKind {
    Connected,
    Disconnected,
    PrintStarted,
    PrintPaused,
    PrintResumed,
    PrintStopped,
    PrintFinished,
    FailureDetected,
    FailureNotifyThreshold,
    FailurePauseThreshold,
    AutoPaused,
    NotificationSent,
    CommandPause,
    CommandResume,
    CommandStop,
    CommandLed(bool),
    CommandFan(String, u8),
    CommandSpeedMode(u8),
    CommandStartPrint,
    DetectionLogged,
    WsConnected,
    WsDisconnected,
    RawConnected,
    RawDisconnected,
    CameraLost,
    CameraRestored,
    ErrorOccurred(String),
    /// loaded event kind
    Loaded(String),
}

impl PrinterState {
    pub fn new() -> Self {
        Self {
            full: FullStatus::default(),
            device_info: None,
            printer_ip: String::new(),
            connected: false,
            connected_raw: false,
            connected_ws: false,
            camera_connected: false,
            detection_score: 0.0,
            detection_history: VecDeque::with_capacity(200),
            latest_detections: Vec::new(),
            latest_detection_ts: 0,
            events: Vec::with_capacity(100),
            events_total: 0,
            files: Vec::new(),
            thumbnail_cache: HashMap::new(),
        }
    }

    pub fn seed(&mut self, status: FullStatus) {
        let old_state = self.print_state();
        // keep canvas_info
        let saved_canvas = self.full.canvas_info.take();
        self.full = status;
        if self.full.canvas_info.is_none() {
            self.full.canvas_info = saved_canvas;
        }
        let new_state = self.print_state();
        if old_state != new_state {
            self.record_state_transition(old_state, new_state);
        }
    }

    pub fn merge_delta(&mut self, delta: &Value) {
        let old_state = self.print_state();

        if let Ok(current) = serde_json::to_value(&self.full) {
            let merged = recursive_merge(&current, delta);
            if let Ok(status) = serde_json::from_value::<FullStatus>(merged) {
                self.full = status;
            }
        }

        let new_state = self.print_state();
        if old_state != new_state {
            self.record_state_transition(old_state, new_state);
        }
    }

    pub fn print_state(&self) -> PrintState {
        match self.full.print_status.state.as_str() {
            "printing" => PrintState::Printing,
            "paused" => PrintState::Paused,
            _ => PrintState::Idle,
        }
    }

    pub fn add_event(&mut self, kind: EventKind, description: String) {
        // dedup same kind+msg
        if let Some(last) = self.events.last() {
            if std::mem::discriminant(&last.kind) == std::mem::discriminant(&kind)
                && last.description == description
            {
                return;
            }
        }
        let e = PrinterEvent {
            timestamp: std::time::SystemTime::now(),
            kind,
            description,
            snapshot: None,
        };
        #[cfg(not(test))]
        Self::persist_event(&e);
        self.events.push(e);
        self.events_total += 1;
        if self.events.len() > 100 {
            self.events.remove(0);
        }
    }

    pub fn add_event_with_snapshot(
        &mut self,
        kind: EventKind,
        description: String,
        snapshot: Option<String>,
    ) {
        let e = PrinterEvent {
            timestamp: std::time::SystemTime::now(),
            kind,
            description,
            snapshot,
        };
        #[cfg(not(test))]
        Self::persist_event(&e);
        self.events.push(e);
        self.events_total += 1;
        if self.events.len() > 100 {
            self.events.remove(0);
        }
    }

    /// append detection log
    pub fn persist_detection_point(pt: &DetectionPoint) {
        let Ok(line) = serde_json::to_string(pt) else { return };
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(DETECTION_LOG_PATH)
        {
            let _ = writeln!(f, "{line}");
        }
    }

    /// load detection history
    pub fn load_detection_history(limit: usize) -> VecDeque<DetectionPoint> {
        let Ok(data) = std::fs::read_to_string(DETECTION_LOG_PATH) else {
            return VecDeque::new();
        };
        let points: Vec<DetectionPoint> = data.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        let skip = points.len().saturating_sub(limit);
        points.into_iter().skip(skip).collect()
    }

    /// count all persisted events without loading them
    pub fn count_events_log() -> u64 {
        use std::io::BufRead;
        let Ok(f) = std::fs::File::open(EVENTS_LOG_PATH) else { return 0; };
        std::io::BufReader::new(f)
            .lines()
            .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
            .count() as u64
    }

    /// load events log
    pub fn load_events_from_log(limit: usize) -> Vec<PrinterEvent> {
        let Ok(data) = std::fs::read_to_string(EVENTS_LOG_PATH) else { return Vec::new(); };
        data.lines()
            .filter_map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).ok()?;
                let ts = v["ts"].as_u64()?;
                let kind = v["kind"].as_str()?.to_string();
                let msg = v["msg"].as_str()?.to_string();
                let snap = v["snap"].as_str().map(|s| s.to_string());
                let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts);
                Some(PrinterEvent {
                    timestamp,
                    kind: EventKind::Loaded(kind),
                    description: msg,
                    snapshot: snap,
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(limit)
            .rev()
            .collect()
    }

    fn persist_event(e: &PrinterEvent) {
        let ts = e.timestamp.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let kind = match &e.kind {
            EventKind::Loaded(s) => s.clone(),
            other => format!("{other:?}"),
        };
        let line = match &e.snapshot {
            Some(snap) => serde_json::json!({"ts":ts,"kind":kind,"msg":e.description,"snap":snap}),
            None => serde_json::json!({"ts":ts,"kind":kind,"msg":e.description}),
        };
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(EVENTS_LOG_PATH) {
            let _ = writeln!(f, "{line}");
        }
    }

    fn record_state_transition(&mut self, from: PrintState, to: PrintState) {
        match (&from, &to) {
            (PrintState::Idle, PrintState::Printing) => {
                let filename = self.full.print_status.filename.clone();
                self.add_event(
                    EventKind::PrintStarted,
                    format!("Print started: {}", truncate_filename(&filename)),
                );
            }
            (PrintState::Printing, PrintState::Paused) => {
                self.add_event(EventKind::PrintPaused, "Print paused".to_string());
            }
            (PrintState::Paused, PrintState::Printing) => {
                self.add_event(EventKind::PrintResumed, "Print resumed".to_string());
            }
            (PrintState::Printing, PrintState::Idle) => {
                self.add_event(EventKind::PrintFinished, "Print finished".to_string());
            }
            (PrintState::Paused, PrintState::Idle) => {
                self.add_event(EventKind::PrintStopped, "Print stopped".to_string());
            }
            _ => {}
        }
    }
}

fn recursive_merge(base: &Value, delta: &Value) -> Value {
    match (base, delta) {
        (Value::Object(base_map), Value::Object(delta_map)) => {
            let mut merged = base_map.clone();
            for (key, delta_value) in delta_map {
                let merged_value = match base_map.get(key) {
                    Some(base_value) => recursive_merge(base_value, delta_value),
                    None => delta_value.clone(),
                };
                merged.insert(key.clone(), merged_value);
            }
            Value::Object(merged)
        }
        (_, delta_value) => delta_value.clone(),
    }
}

fn truncate_filename(name: &str) -> String {
    if name.len() <= 40 {
        name.to_string()
    } else {
        let mut cut = name.len() - 37;
        while !name.is_char_boundary(cut) { cut += 1; }
        format!("...{}", &name[cut..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_recursive_merge_preserves_nested_fields() {
        let base = json!({
            "fans": {
                "aux_fan": {"speed": 100.0},
                "fan": {"speed": 50.0},
                "box_fan": {"speed": 0.0}
            },
            "extruder": {
                "temperature": 200.0,
                "target": 210
            }
        });
        let delta = json!({ "fans": { "fan": {"speed": 255.0} } });
        let merged = recursive_merge(&base, &delta);
        assert_eq!(merged["fans"]["fan"]["speed"], 255.0);
        assert_eq!(merged["fans"]["aux_fan"]["speed"], 100.0);
        assert_eq!(merged["fans"]["box_fan"]["speed"], 0.0);
        assert_eq!(merged["extruder"]["temperature"], 200.0);
    }

    #[test]
    fn test_recursive_merge_adds_new_keys() {
        let base = json!({ "fans": { "fan": {"speed": 50.0} } });
        let delta = json!({
            "fans": { "aux_fan": {"speed": 100.0} },
            "led": {"status": 1}
        });
        let merged = recursive_merge(&base, &delta);
        assert_eq!(merged["fans"]["fan"]["speed"], 50.0);
        assert_eq!(merged["fans"]["aux_fan"]["speed"], 100.0);
        assert_eq!(merged["led"]["status"], 1);
    }

    #[test]
    fn test_recursive_merge_overwrites_scalars() {
        let base = json!({ "machine_status": { "status": 1, "progress": 50 } });
        let delta = json!({ "machine_status": { "status": 2, "progress": 75 } });
        let merged = recursive_merge(&base, &delta);
        assert_eq!(merged["machine_status"]["status"], 2);
        assert_eq!(merged["machine_status"]["progress"], 75);
    }

    #[test]
    fn test_print_state_idle() {
        let mut state = PrinterState::new();
        state.full.print_status.state = String::new();
        assert!(matches!(state.print_state(), PrintState::Idle));
    }

    #[test]
    fn test_print_state_printing() {
        let mut state = PrinterState::new();
        state.full.print_status.state = "printing".to_string();
        assert!(matches!(state.print_state(), PrintState::Printing));
    }

    #[test]
    fn test_print_state_paused() {
        let mut state = PrinterState::new();
        state.full.print_status.state = "paused".to_string();
        assert!(matches!(state.print_state(), PrintState::Paused));
    }

    #[test]
    fn test_merge_delta_updates_state() {
        let mut state = PrinterState::new();
        state.full.print_status.state = "printing".to_string();
        state.full.print_status.filename = "test.gcode".to_string();
        state.merge_delta(&json!({ "print_status": { "state": "paused" } }));
        assert!(matches!(state.print_state(), PrintState::Paused));
    }

    #[test]
    fn test_events_capped_at_100() {
        let mut state = PrinterState::new();
        for i in 0..105 {
            state.add_event(EventKind::Connected, format!("event {}", i));
        }
        assert_eq!(state.events.len(), 100);
        assert_eq!(state.events[0].description, "event 5");
    }
}
