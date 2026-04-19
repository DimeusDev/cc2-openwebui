//! MQTT wire structs for the CC2 protocol.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: u64,
    pub method: u16,
    pub result: RpcResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResult {
    pub error_code: u16,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub client_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub client_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub hardware_version: String,
    pub hostname: String,
    pub ip: String,
    pub machine_model: String,
    pub protocol_version: String,
    pub sn: String,
    pub software_version: SoftwareVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareVersion {
    pub mcu_version: String,
    pub ota_version: String,
    pub soc_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FullStatus {
    pub external_device: Option<ExternalDevice>,
    pub extruder: Extruder,
    pub fans: Fans,
    pub gcode_move: GcodeMove,
    pub heater_bed: HeaterBed,
    pub led: Led,
    pub machine_status: MachineStatus,
    pub print_status: PrintStatus,
    pub tool_head: ToolHead,
    pub ztemperature_sensor: ZTemperatureSensor,
    /// AMS tray info from method 6000.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canvas_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalDevice {
    pub camera: bool,
    #[serde(rename = "type")]
    pub device_type: String,
    pub u_disk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Extruder {
    pub filament_detect_enable: Option<i64>,
    pub filament_detected: Option<i64>,
    pub target: i64,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Fans {
    pub aux_fan: Option<FanSpeed>,
    pub box_fan: Option<FanSpeed>,
    pub controller_fan: Option<FanSpeed>,
    pub fan: Option<FanSpeed>,
    pub heater_fan: Option<FanSpeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FanSpeed {
    pub speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GcodeMove {
    pub extruder: Option<f64>,
    pub speed: Option<i64>,
    pub speed_mode: Option<i64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeaterBed {
    pub target: i64,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Led {
    pub status: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MachineStatus {
    pub exception_status: Option<Vec<serde_json::Value>>,
    pub progress: i64,
    pub status: i64,
    pub sub_status: i64,
    pub sub_status_reason_code: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrintStatus {
    pub bed_mesh_detect: Option<bool>,
    pub current_layer: Option<i64>,
    pub enable: Option<bool>,
    pub filament_detect: Option<bool>,
    pub filename: String,
    pub print_duration: Option<i64>,
    pub remaining_time_sec: Option<i64>,
    pub state: String,
    pub total_duration: Option<i64>,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolHead {
    pub homed_axes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZTemperatureSensor {
    pub measured_max_temperature: Option<f64>,
    pub measured_min_temperature: Option<f64>,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPrintParams {
    pub filename: String,
    pub storage_media: String,
    pub config: StartPrintConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPrintConfig {
    pub bedlevel_force: bool,
    pub delay_video: bool,
    pub print_layout: String,
    pub printer_check: bool,
    pub slot_map: Vec<SlotMapEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotMapEntry {
    pub canvas_id: i64,
    pub t: i64,
    pub tray_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLedParams {
    pub power: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetFanParams {
    #[serde(flatten)]
    pub fan_map: serde_json::Value,
}

impl SetFanParams {
    pub fn single(name: &str, speed: i64) -> Self {
        Self {
            fan_map: serde_json::json!({ name: speed }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSpeedModeParams {
    pub mode: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFileListParams {
    pub storage_media: String,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub error_code: u16,
    pub file_list: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub color_map: Option<Vec<ColorMapEntry>>,
    pub create_time: Option<i64>,
    pub filename: String,
    pub last_print_time: Option<i64>,
    pub layer: Option<i64>,
    pub print_time: Option<i64>,
    pub size: Option<i64>,
    pub total_filament_used: Option<f64>,
    pub total_print_times: Option<i64>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorMapEntry {
    pub color: String,
    pub name: String,
    pub t: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetFileInfoParams {
    pub storage_media: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintHistoryResponse {
    pub error_code: u16,
    pub history_task_list: Vec<HistoryTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryTask {
    pub begin_time: i64,
    pub end_time: i64,
    pub task_id: String,
    pub task_name: String,
    pub task_status: i64,
    pub time_lapse_video_duration: i64,
    pub time_lapse_video_size: i64,
    pub time_lapse_video_status: i64,
    pub time_lapse_video_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetAMSAutoRefillParams {
    pub auto_refill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMSInfoResponse {
    pub error_code: u16,
    pub canvas_info: CanvasInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasInfo {
    pub active_canvas_id: i64,
    pub active_tray_id: i64,
    pub auto_refill: bool,
    pub canvas_list: Vec<CanvasEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasEntry {
    pub canvas_id: i64,
    pub connected: i64,
    pub tray_list: Vec<TrayEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayEntry {
    pub brand: String,
    pub filament_code: String,
    pub filament_color: String,
    pub filament_name: String,
    pub filament_type: String,
    pub max_nozzle_temp: i64,
    pub min_nozzle_temp: i64,
    pub status: i64,
    pub tray_id: i64,
}

pub const METHOD_GET_DEVICE_INFO: u16 = 1001;
pub const METHOD_GET_FULL_STATUS: u16 = 1002;
pub const METHOD_START_PRINT: u16 = 1020;
pub const METHOD_PAUSE_PRINT: u16 = 1021;
pub const METHOD_STOP_PRINT: u16 = 1022;
pub const METHOD_RESUME_PRINT: u16 = 1023;
pub const METHOD_SET_LED: u16 = 1029;
pub const METHOD_SET_FAN: u16 = 1030;
pub const METHOD_SET_SPEED_MODE: u16 = 1031;
pub const METHOD_GET_PRINT_HISTORY: u16 = 1036;
pub const METHOD_GET_FILE_LIST: u16 = 1044;
pub const METHOD_GET_FILE_INFO: u16 = 1046;
pub const METHOD_SET_AMS_AUTO_REFILL: u16 = 2004;
pub const METHOD_GET_AMS_INFO: u16 = 2005;
pub const METHOD_STATUS_PUSH: u16 = 6000;

pub const ERROR_SUCCESS: u16 = 0;
pub const ERROR_LOCKED: u16 = 1009;
pub const ERROR_WRONG_STATE: u16 = 1010;
