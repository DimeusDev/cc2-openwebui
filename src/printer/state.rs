use std::collections::VecDeque;
use std::io::Write;
use std::time::UNIX_EPOCH;

use serde_json::Value;

use super::models::{DeviceInfo, FullStatus};
use crate::detection::obico::Detection;

pub const EVENTS_LOG_PATH: &str = "data/events.log";

#[derive(Debug, Clone, PartialEq)]
pub enum PrintState {
    Idle,
    Printing,
    Paused,
}

/// One data point in the detection history.
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
    /// True when both raw (port 1883) and ws (port 9001) clients are connected.
    pub connected: bool,
    pub connected_raw: bool,
    pub connected_ws: bool,
    pub detection_score: f64,
    pub detection_history: VecDeque<DetectionPoint>,
    /// Latest bounding-box detections from the Obico ML API (after zone filtering).
    pub latest_detections: Vec<Detection>,
    pub latest_detection_ts: u64,
    pub events: Vec<PrinterEvent>,
    pub files: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct PrinterEvent {
    pub timestamp: std::time::SystemTime,
    pub kind: EventKind,
    pub description: String,
    pub snapshot: Option<String>,
}

// EventKind values are logged via Debug names.
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
    ErrorOccurred(String),
    /// Event loaded from persisted log on startup; inner string is the original kind name.
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
            detection_score: 0.0,
            detection_history: VecDeque::with_capacity(30),
            latest_detections: Vec::new(),
            latest_detection_ts: 0,
            events: Vec::with_capacity(100),
            files: Vec::new(),
        }
    }

    pub fn seed(&mut self, status: FullStatus) {
        let old_state = self.print_state();
        // 1002 response never includes canvas_info; preserve it across seed calls
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
        let e = PrinterEvent {
            timestamp: std::time::SystemTime::now(),
            kind,
            description,
            snapshot: None,
        };
        Self::persist_event(&e);
        self.events.push(e);
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
        Self::persist_event(&e);
        self.events.push(e);
        if self.events.len() > 100 {
            self.events.remove(0);
        }
    }

    /// Load last `limit` events from the JSONL log file.
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
        format!("...{}", &name[name.len() - 37..])
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
