import { writable } from 'svelte/store';

export interface Toast {
  id: number;
  message: string;
  type: 'error' | 'warn' | 'info';
}

let _toastId = 0;
export const toasts = writable<Toast[]>([]);

export function showToast(message: string, type: Toast['type'] = 'error', duration = 4000) {
  const id = ++_toastId;
  toasts.update(t => [...t, { id, message, type }]);
  setTimeout(() => toasts.update(t => t.filter(x => x.id !== id)), duration);
}

export interface DetectionBox {
  x1: number; y1: number; x2: number; y2: number; confidence: number;
}

export interface DetectionPoint {
  ts: number;
  score: number;
  snapshot?: string;
  print_filename?: string;
  boxes?: DetectionBox[];
}

export interface ColorMapEntry {
  color: string;
  name: string;
  t: number;
}

export interface PrinterFile {
  filename?: string;
  name?: string;
  size?: number;
  file_size?: number;
  total_layer?: number;
  layers?: number;
  layer?: number;
  create_time?: number;
  created?: number;
  print_time?: number;
  total_filament_used?: number;
  color_map?: ColorMapEntry[];
  [key: string]: unknown;
}

export interface PrinterState {
  connected: boolean;
  printer_id: string;
  printer_ip: string;
  state: FullStatus | null;
  detection_score: number;
  detection_history: DetectionPoint[];
  nozzle_history: number[];
  bed_history: number[];
  files: PrinterFile[];
  events: AppEvent[];
}

export interface FullStatus {
  extruder: Extruder;
  fans: Fans;
  gcode_move: GcodeMove;
  heater_bed: HeaterBed;
  led: Led;
  machine_status: MachineStatus;
  print_status: PrintStatus;
  tool_head: ToolHead;
  ztemperature_sensor: ZTemperatureSensor;
  canvas_info?: CanvasInfo;
  external_device?: ExternalDevice;
}

export interface Extruder {
  target: number;
  temperature: number;
  filament_detected?: number;
  filament_detect_enable?: number;
}

export interface TrayEntry {
  brand: string;
  filament_code: string;
  filament_color: string;
  filament_name: string;
  filament_type: string;
  tray_type?: string;
  max_nozzle_temp: number;
  min_nozzle_temp: number;
  status: number;
  tray_id: number;
}

export interface CanvasEntry {
  canvas_id: number;
  connected: number;
  tray_list: TrayEntry[];
}

export interface CanvasInfo {
  active_canvas_id: number;
  active_tray_id: number;
  auto_refill: boolean;
  canvas_list: CanvasEntry[];
}

export interface ExternalDevice {
  camera: boolean;
  type: string;
  u_disk: boolean;
}

export interface Fans {
  aux_fan?: FanSpeed;
  box_fan?: FanSpeed;
  controller_fan?: FanSpeed;
  fan?: FanSpeed;
  heater_fan?: FanSpeed;
}

export interface FanSpeed {
  speed: number;
}

export interface GcodeMove {
  x?: number;
  y?: number;
  z?: number;
  speed_mode?: number;
}

export interface HeaterBed {
  target: number;
  temperature: number;
}

export interface Led {
  status: number;
}

export interface MachineStatus {
  progress: number;
  status: number;
  sub_status: number;
}

export interface PrintStatus {
  current_layer?: number;
  filename: string;
  print_duration?: number;
  remaining_time_sec?: number;
  state: string;
  total_duration?: number;
}

export interface ToolHead {
  homed_axes: string;
}

export interface ZTemperatureSensor {
  temperature: number;
}

export interface AppEvent {
  kind: string;
  description: string;
}

export interface DetectionStatus {
  enabled: boolean;
  notify_threshold: number;
  pause_threshold: number;
  interval_secs: number;
  confirmation_frames: number;
}

export const printer = writable<PrinterState>({
  connected: false,
  printer_id: '',
  printer_ip: '',
  state: null,
  detection_score: 0,
  detection_history: [],
  nozzle_history: [],
  bed_history: [],
  files: [],
  events: [],
});

export const events = writable<AppEvent[]>([]);

export const detection = writable<DetectionStatus>({
  enabled: true,
  notify_threshold: 0.5,
  pause_threshold: 0.7,
  interval_secs: 15,
  confirmation_frames: 2,
});

