import { writable } from 'svelte/store';
import { printer, events } from './stores';
import type { FullStatus, AppEvent } from './stores';

export const wsConnected = writable(false);
export const wsError = writable<string | null>(null);

let ws: WebSocket | null = null;
let reconnectTimer: number | null = null;
let reconnectAttempts = 0;

export function connect() {
  if (ws) return;

  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const url = `${proto}//${window.location.host}/ws`;

  wsConnected.set(false);
  wsError.set(null);

  try {
    ws = new WebSocket(url);
  } catch {
    wsError.set('Failed to connect');
    return;
  }

  ws.onopen = () => {
    wsConnected.set(true);
    wsError.set(null);
    reconnectAttempts = 0;
  };

  ws.onmessage = (e) => {
    try {
      const msg = JSON.parse(e.data);
      if (msg.type === 'state' && msg.data) {
        const HISTORY_MAX = 60;
        printer.update((s) => {
          const newNozzle = msg.data?.extruder?.temperature ?? s.state?.extruder?.temperature;
          const newBed = msg.data?.heater_bed?.temperature ?? s.state?.heater_bed?.temperature;
          const nozzle_history = newNozzle != null
            ? [...s.nozzle_history, newNozzle].slice(-HISTORY_MAX)
            : s.nozzle_history;
          const bed_history = newBed != null
            ? [...s.bed_history, newBed].slice(-HISTORY_MAX)
            : s.bed_history;
          return {
            ...s,
            state: msg.data as FullStatus,
            connected: msg.connected === true,
            printer_ip: msg.printer_ip ?? s.printer_ip,
            detection_score: msg.detection_score ?? s.detection_score,
            detection_history: msg.detection_history ?? s.detection_history,
            files: msg.files ?? s.files,
            nozzle_history,
            bed_history,
          };
        });
      } else if (msg.type === 'event' && msg.data) {
        const evt = msg.data as AppEvent;
        events.update((evts) => [evt, ...evts].slice(0, 20));
      } else if (msg.type === 'pong') {
      }
    } catch (err) {
      wsError.set(`Malformed server payload: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  ws.onclose = () => {
    ws = null;
    wsConnected.set(false);
    scheduleReconnect();
  };

  ws.onerror = () => {
    wsError.set('Connection error');
  };
}

function scheduleReconnect() {
  if (reconnectTimer !== null) return;

  reconnectAttempts++;
  const delay = Math.min(1000 * Math.pow(1.5, reconnectAttempts), 30000);

  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null;
    connect();
  }, delay);
}

export function disconnect() {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  ws?.close();
  ws = null;
  wsConnected.set(false);
}

export function sendPing() {
  ws?.send(JSON.stringify({ type: 'ping' }));
}
