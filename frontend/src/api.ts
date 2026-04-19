import type { DetectionStatus } from './stores';

const BASE = '';

async function apiError(res: Response, context: string): Promise<never> {
  let message = context;
  try {
    const body = await res.json();
    if (body?.error) message = body.error;
  } catch {
    const text = await res.text().catch(() => '');
    if (text) message = text;
  }
  throw new Error(message);
}

async function postJson(url: string, body: unknown): Promise<Response> {
  return fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}


export async function checkSetup(): Promise<{ configured: boolean; onboarding_complete: boolean }> {
  const res = await fetch(`${BASE}/api/setup/check`);
  if (!res.ok) return { configured: false, onboarding_complete: false };
  return res.json();
}

export interface HostOs {
  os: 'linux' | 'macos' | 'windows' | string;
  arch: string;
  docker_command: string;
  gpu_supported: boolean;
}

export async function getHostOs(): Promise<HostOs> {
  const res = await fetch(`${BASE}/api/setup/host-os`);
  if (!res.ok) await apiError(res, 'Failed to get host OS');
  return res.json();
}

export interface EventToggles {
  print_started: boolean;
  print_finished: boolean;
  print_paused: boolean;
  failure_notify: boolean;
  failure_pause: boolean;
  auto_paused: boolean;
}

export type DestinationKind = 'ntfy' | 'discord' | 'webhook';

export interface NotificationDestination {
  id: string;
  kind: DestinationKind;
  enabled: boolean;
  label: string;
  ntfy_server?: string;
  ntfy_topic?: string;
  discord_webhook_url?: string;
  webhook_url?: string;
  toggles: EventToggles;
}

export function defaultToggles(): EventToggles {
  return {
    print_started: true,
    print_finished: true,
    print_paused: true,
    failure_notify: true,
    failure_pause: true,
    auto_paused: true,
  };
}

export interface OnboardingPayload {
  detection?: {
    obico_url?: string;
    notify_threshold?: number;
    pause_threshold?: number;
  };
  notifications?: {
    destinations?: NotificationDestination[];
  };
}

export async function completeOnboarding(payload: OnboardingPayload): Promise<void> {
  const res = await postJson(`${BASE}/api/setup/complete`, payload);
  if (!res.ok) await apiError(res, 'Failed to complete onboarding');
}

export async function scanNetwork(subnet?: string): Promise<{ printers: Array<{ ip: string }> }> {
  const res = await postJson(`${BASE}/api/setup/scan`, subnet ? { subnet } : {});
  if (!res.ok) await apiError(res, 'Network scan failed');
  return res.json();
}

export async function verifyPrinter(ip: string, pincode: string): Promise<{
  success: boolean;
  printer_id: string;
  model: string;
}> {
  const res = await postJson(`${BASE}/api/setup/verify`, {
    ip,
    pincode: pincode || undefined,
  });
  if (!res.ok) await apiError(res, 'Connection verification failed');
  return res.json();
}

export type ObicoStatus = 'unavailable' | 'not_created' | 'stopped' | 'running';

export async function getObicoStatus(): Promise<{ status: ObicoStatus }> {
  const res = await fetch(`${BASE}/api/setup/obico/status`);
  if (!res.ok) await apiError(res, 'Failed to get Obico status');
  return res.json();
}

export async function startObicoContainer(): Promise<{ success: boolean; url: string }> {
  const res = await postJson(`${BASE}/api/setup/obico/start`, null);
  if (!res.ok) await apiError(res, 'Failed to start Obico container');
  return res.json();
}

export async function stopObicoContainer(): Promise<void> {
  const res = await postJson(`${BASE}/api/setup/obico/stop`, null);
  if (!res.ok) await apiError(res, 'Failed to stop Obico container');
}

export async function testObicoContainer(): Promise<{ ok: boolean }> {
  const res = await fetch(`${BASE}/api/setup/obico/test`);
  if (!res.ok) await apiError(res, 'Failed to test Obico container');
  return res.json();
}

export async function testObicoUrl(url: string): Promise<{ ok: boolean; status: number }> {
  const res = await postJson(`${BASE}/api/setup/obico/test-url`, { url });
  if (!res.ok) await apiError(res, 'Failed to test Obico URL');
  return res.json();
}

export async function resetAll(): Promise<void> {
  const res = await postJson(`${BASE}/api/setup/reset`, null);
  if (!res.ok) await apiError(res, 'Failed to reset settings');
}

export async function saveConfig(ip: string, printer_id: string, pincode: string): Promise<{ success: boolean }> {
  const res = await postJson(`${BASE}/api/setup/save`, {
    ip,
    printer_id,
    pincode: pincode || undefined,
  });
  if (!res.ok) await apiError(res, 'Failed to save configuration');
  return res.json();
}


export interface PrinterStatusResponse {
  connected: boolean;
  connected_raw: boolean;
  connected_ws: boolean;
  printer_id: string;
  printer_ip: string;
  state: Record<string, unknown> | null;
}

export async function getPrinterStatus(): Promise<PrinterStatusResponse> {
  const res = await fetch(`${BASE}/api/printer/status`);
  if (!res.ok) await apiError(res, 'Failed to get printer status');
  return res.json();
}

export async function pausePrint(): Promise<void> {
  const res = await postJson(`${BASE}/api/printer/pause`, null);
  if (!res.ok) await apiError(res, 'Failed to pause print');
}

export async function resumePrint(): Promise<void> {
  const res = await postJson(`${BASE}/api/printer/resume`, null);
  if (!res.ok) await apiError(res, 'Failed to resume print');
}

export async function stopPrint(): Promise<void> {
  const res = await postJson(`${BASE}/api/printer/stop`, null);
  if (!res.ok) await apiError(res, 'Failed to stop print');
}

export async function setLed(power: boolean): Promise<void> {
  const res = await postJson(`${BASE}/api/printer/led`, { power: power ? 1 : 0 });
  if (!res.ok) await apiError(res, 'Failed to set LED');
}

export async function setFan(name: string, speed: number): Promise<void> {
  const res = await postJson(`${BASE}/api/printer/fan`, { name, speed });
  if (!res.ok) await apiError(res, 'Failed to set fan speed');
}

export async function setSpeedMode(mode: number): Promise<void> {
  const res = await postJson(`${BASE}/api/printer/speed-mode`, { mode });
  if (!res.ok) await apiError(res, 'Failed to set speed mode');
}

export interface StartPrintOptions {
  plate?: 'textured' | 'smooth';
  tray_id?: number | null;
  timelapse?: boolean;
  bedlevel_force?: boolean;
}

export async function startPrint(filename: string, storage_media: string = 'local', opts: StartPrintOptions = {}): Promise<void> {
  const res = await postJson(`${BASE}/api/printer/print`, {
    filename,
    storage_media,
    plate: opts.plate ?? 'textured',
    tray_id: opts.tray_id ?? null,
    timelapse: opts.timelapse ?? false,
    bedlevel_force: opts.bedlevel_force ?? true,
  });
  if (!res.ok) await apiError(res, 'Failed to start print');
}

export async function getFiles(storage: string = 'local', offset: number = 0, limit: number = 20): Promise<void> {
  const res = await fetch(`${BASE}/api/printer/files?storage=${storage}&offset=${offset}&limit=${limit}`);
  if (!res.ok) await apiError(res, 'Failed to get file list');
}

export async function uploadGcode(file: File): Promise<{ ok: boolean; bytes: number }> {
  const buf = await file.arrayBuffer();
  const res = await fetch(`${BASE}/api/printer/upload`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/octet-stream' },
    body: buf,
  });
  if (!res.ok) await apiError(res, 'Upload failed');
  return res.json();
}

export interface HistoryTask {
  filename?: string;
  task_name?: string;
  name?: string;
  create_time?: number;
  begin_time?: number;
  size?: number;
  file_size?: number;
  total_layer?: number;
  [key: string]: unknown;
}

export async function getHistory(): Promise<{ history: HistoryTask[] }> {
  const res = await fetch(`${BASE}/api/printer/history`);
  if (!res.ok) await apiError(res, 'Failed to get print history');
  return res.json();
}

export async function refreshCanvas(): Promise<Record<string, unknown>> {
  const res = await postJson(`${BASE}/api/printer/canvas/refresh`, null);
  if (!res.ok) await apiError(res, 'Failed to refresh canvas');
  return res.json();
}


export async function getDetectionStatus(): Promise<DetectionStatus> {
  const res = await fetch(`${BASE}/api/detection/status`);
  if (!res.ok) await apiError(res, 'Failed to get detection status');
  return res.json();
}

export async function toggleDetection(): Promise<void> {
  const res = await postJson(`${BASE}/api/detection/toggle`, null);
  if (!res.ok) await apiError(res, 'Failed to toggle detection');
}

export async function updateDetectionConfig(config: Partial<DetectionStatus>): Promise<void> {
  const res = await postJson(`${BASE}/api/detection/config`, config);
  if (!res.ok) await apiError(res, 'Failed to update detection config');
}

export interface DetectionBox {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  confidence: number;
}

export interface ExcludeZone {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export interface LatestDetection {
  score: number;
  detections: DetectionBox[];
  timestamp: number;
}

export async function getLatestDetection(): Promise<LatestDetection> {
  const res = await fetch(`${BASE}/api/detection/latest`);
  if (!res.ok) await apiError(res, 'Failed to get latest detection');
  return res.json();
}

export async function setExcludeZones(zones: ExcludeZone[]): Promise<void> {
  const res = await fetch(`${BASE}/api/detection/zones`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(zones),
  });
  if (!res.ok) await apiError(res, 'Failed to save exclusion zones');
}

export function cameraSnapshotUrl(): string {
  return `${BASE}/api/camera/snapshot`;
}


export async function listDestinations(): Promise<NotificationDestination[]> {
  const res = await fetch(`${BASE}/api/notifications/destinations`);
  if (!res.ok) await apiError(res, 'Failed to list notification destinations');
  return res.json();
}

export async function createDestination(
  dest: Omit<NotificationDestination, 'id'>,
): Promise<string> {
  const res = await postJson(`${BASE}/api/notifications/destinations`, { ...dest, id: '' });
  if (!res.ok) await apiError(res, 'Failed to create notification destination');
  const body = await res.json();
  return body.id as string;
}

export async function updateDestination(
  id: string,
  dest: NotificationDestination,
): Promise<void> {
  const res = await fetch(`${BASE}/api/notifications/destinations/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dest),
  });
  if (!res.ok) await apiError(res, 'Failed to update notification destination');
}

export async function deleteDestination(id: string): Promise<void> {
  const res = await fetch(`${BASE}/api/notifications/destinations/${id}`, {
    method: 'DELETE',
  });
  if (!res.ok) await apiError(res, 'Failed to delete notification destination');
}

export async function testDestination(id: string): Promise<void> {
  const res = await postJson(`${BASE}/api/notifications/destinations/${id}/test`, null);
  if (!res.ok) await apiError(res, 'Test notification failed');
}


export interface RunDetectionResult {
  score: number;
  detections: DetectionBox[];
}

export async function runDetection(): Promise<RunDetectionResult> {
  const res = await postJson(`${BASE}/api/detection/run`, null);
  if (!res.ok) await apiError(res, 'Detection run failed');
  return res.json();
}


export interface LogEntry {
  timestamp: number;
  kind: string;
  message: string;
  snapshot?: string;
}

export interface LogsResponse {
  logs: LogEntry[];
}

export async function getLogs(): Promise<LogsResponse> {
  const res = await fetch(`${BASE}/api/logs`);
  if (!res.ok) await apiError(res, 'Failed to get logs');
  return res.json();
}

export async function deleteLogs(): Promise<void> {
  const res = await fetch(`${BASE}/api/logs`, { method: 'DELETE' });
  if (!res.ok) await apiError(res, 'Failed to delete logs');
}

export async function deleteSnapshot(filename: string): Promise<void> {
  const res = await fetch(`${BASE}/api/snapshots/${encodeURIComponent(filename)}`, { method: 'DELETE' });
  if (!res.ok) await apiError(res, 'Failed to delete snapshot');
}


export interface AppSettings {
  printer: { ip: string; printer_id: string; pincode: string };
  detection: {
    enabled: boolean;
    notify_threshold: number;
    pause_threshold: number;
    interval_secs: number;
    confirmation_frames: number;
    obico_url: string;
  };
  notifications: { destinations: NotificationDestination[] };
  server: { host: string; port: number };
  logging: { level: string };
}

export async function getSettings(): Promise<AppSettings> {
  const res = await fetch(`${BASE}/api/settings`);
  if (!res.ok) await apiError(res, 'Failed to get settings');
  return res.json();
}

export async function updateSettings(settings: Partial<AppSettings> | Record<string, unknown>): Promise<void> {
  const res = await postJson(`${BASE}/api/settings`, settings);
  if (!res.ok) await apiError(res, 'Failed to save settings');
}


export interface SnapshotEntry {
  filename: string;
  size: number;
  mtime: number;
  score_pct: number | null;
  boxes: Array<{ x1: number; y1: number; x2: number; y2: number; confidence: number }>;
}

export interface SnapshotListResponse {
  snapshots: SnapshotEntry[];
  total: number;
  total_bytes: number;
}

export async function listSnapshots(offset = 0, limit = 50): Promise<SnapshotListResponse> {
  const res = await fetch(`${BASE}/api/snapshots?offset=${offset}&limit=${limit}`);
  if (!res.ok) await apiError(res, 'Failed to list snapshots');
  return res.json();
}

export async function deleteAllSnapshots(): Promise<void> {
  const res = await fetch(`${BASE}/api/snapshots`, { method: 'DELETE' });
  if (!res.ok) await apiError(res, 'Failed to delete snapshots');
}

export function snapshotUrl(filename: string): string {
  return `${BASE}/snapshots/${encodeURIComponent(filename)}`;
}
