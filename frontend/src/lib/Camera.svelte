<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Camera as CameraIcon } from 'lucide-svelte';
  import { printer, showToast } from '../stores';
  import { getLatestDetection, setExcludeZones } from '../api';
  import { toErrorMessage } from './errors';
  import type { DetectionBox, ExcludeZone } from '../api';

  let collapsed = false;
  let editMode = false;
  let savedMsg = false;

  let canvas: HTMLCanvasElement;
  let imgEl: HTMLImageElement;
  let canvasReady = false;
  let imgError = false;

  let detections: DetectionBox[] = [];
  let zones: ExcludeZone[] = [];

  let dragging = false;
  let dragStart = { x: 0, y: 0 };
  let dragCurrent = { x: 0, y: 0 };

  $: connected = $printer.connected;
  $: printerIp = $printer.printer_ip;
  $: streamUrl = printerIp ? `http://${printerIp}:8080/?action=stream` : '';
  $: if (connected && printerIp) imgError = false;

  let pollTimer: number;

  async function pollDetection() {
    try {
      const res = await getLatestDetection();
      detections = res.detections ?? [];
      redraw();
    } catch { /* detection may not be running yet */ }
  }

  onMount(() => {
    pollTimer = window.setInterval(pollDetection, 2000);
  });
  onDestroy(() => clearInterval(pollTimer));

  function onImgLoad() {
    if (!canvas || !imgEl) return;
    const rect = imgEl.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      canvas.width = rect.width * window.devicePixelRatio;
      canvas.height = rect.height * window.devicePixelRatio;
    }
    canvasReady = true;
    imgError = false;
    redraw();
  }

  function redraw() {
    if (!canvas || !canvasReady) return;
    draw(detections, zones, dragging, dragStart, dragCurrent, editMode);
  }

  function draw(
    dets: DetectionBox[],
    zs: ExcludeZone[],
    isDragging: boolean,
    start: { x: number; y: number },
    cur: { x: number; y: number },
    editing: boolean,
  ) {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const W = canvas.width;
    const H = canvas.height;
    ctx.clearRect(0, 0, W, H);

    for (const z of zs) {
      const x = z.x1 * W, y = z.y1 * H;
      const w = (z.x2 - z.x1) * W, h = (z.y2 - z.y1) * H;
      ctx.fillStyle = 'rgba(0,0,0,0.45)';
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = 'rgba(200,200,220,0.55)';
      ctx.lineWidth = 1;
      ctx.setLineDash([5, 5]);
      ctx.strokeRect(x, y, w, h);
      ctx.setLineDash([]);
      if (editing) {
        ctx.font = `bold ${Math.round(9 * window.devicePixelRatio)}px monospace`;
        ctx.fillStyle = 'rgba(200,200,220,0.7)';
        ctx.fillText('× click to remove', x + 5, y + 14 * window.devicePixelRatio);
      }
    }

    for (const d of dets) {
      const x = d.x1 * W, y = d.y1 * H;
      const w = (d.x2 - d.x1) * W, h = (d.y2 - d.y1) * H;
      const r = Math.round(220 + 35 * d.confidence);
      const g = Math.round(120 * (1 - d.confidence));
      ctx.strokeStyle = `rgb(${r},${g},30)`;
      ctx.lineWidth = 2;
      ctx.strokeRect(x, y, w, h);

      const label = `${(d.confidence * 100).toFixed(0)}%`;
      const fs = Math.round(10 * window.devicePixelRatio);
      ctx.font = `bold ${fs}px monospace`;
      const tw = ctx.measureText(label).width;
      const pad = 4;
      ctx.fillStyle = `rgba(${r},${g},30,0.88)`;
      ctx.fillRect(x, y - fs - pad * 2, tw + pad * 2, fs + pad * 2);
      ctx.fillStyle = '#fff';
      ctx.fillText(label, x + pad, y - pad - 1);
    }

    if (isDragging && editing) {
      const x = Math.min(start.x, cur.x);
      const y = Math.min(start.y, cur.y);
      const w = Math.abs(cur.x - start.x);
      const h = Math.abs(cur.y - start.y);
      ctx.fillStyle = 'rgba(0,0,0,0.3)';
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = 'rgba(210,210,240,0.85)';
      ctx.lineWidth = 1;
      ctx.setLineDash([5, 5]);
      ctx.strokeRect(x, y, w, h);
      ctx.setLineDash([]);
    }
  }


  function canvasCoords(e: MouseEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio;
    return {
      x: (e.clientX - rect.left) * dpr,
      y: (e.clientY - rect.top) * dpr,
    };
  }

  function onMouseDown(e: MouseEvent) {
    if (!editMode) return;
    e.preventDefault();
    const { x, y } = canvasCoords(e);
    const W = canvas.width, H = canvas.height;

    for (let i = zones.length - 1; i >= 0; i--) {
      const z = zones[i];
      if (x >= z.x1 * W && x <= z.x2 * W && y >= z.y1 * H && y <= z.y2 * H) {
        zones = zones.filter((_, idx) => idx !== i);
        saveZones();
        return;
      }
    }

    dragging = true;
    dragStart = { x, y };
    dragCurrent = { x, y };
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    dragCurrent = canvasCoords(e);
    redraw();
  }

  function finishDrag() {
    if (!dragging) return;
    dragging = false;
    const W = canvas.width, H = canvas.height;
    const x1 = Math.min(dragStart.x, dragCurrent.x) / W;
    const y1 = Math.min(dragStart.y, dragCurrent.y) / H;
    const x2 = Math.max(dragStart.x, dragCurrent.x) / W;
    const y2 = Math.max(dragStart.y, dragCurrent.y) / H;
    if (x2 - x1 < 0.02 || y2 - y1 < 0.02) { redraw(); return; }
    zones = [...zones, { x1, y1, x2, y2 }];
    saveZones();
  }

  async function saveZones() {
    try {
      await setExcludeZones(zones);
      savedMsg = true;
      setTimeout(() => (savedMsg = false), 1500);
    } catch (e) {
      showToast(toErrorMessage(e));
    }
    redraw();
  }

  function clearAllZones() {
    zones = [];
    saveZones();
  }

  $: if (canvasReady) redraw();
</script>

<div class="card">
  <div class="card-header" role="button" tabindex="0"
    on:click={() => (collapsed = !collapsed)}
    on:keydown={(e) => e.key === 'Enter' && (collapsed = !collapsed)}>
    <div class="header-left">
      <CameraIcon size={13} strokeWidth={1.9} />
      <span class="card-title">Camera</span>
    </div>
    <div class="header-right" role="none" on:click|stopPropagation on:keydown|stopPropagation>
      {#if !collapsed}
        {#if savedMsg}<span class="saved-lbl">Saved</span>{/if}
        {#if editMode && zones.length > 0}
          <button class="hdr-btn danger" on:click={clearAllZones}>Clear zones</button>
        {/if}
        <button class="hdr-btn {editMode ? 'active' : ''}" on:click={() => (editMode = !editMode)}>
          {editMode ? 'Done' : 'Edit Zones'}
        </button>
      {/if}
      <svg class="chevron {collapsed ? 'up' : ''}" width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M3 5l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
  </div>

  {#if !collapsed}
    <div class="cam-wrap">
      {#if connected && streamUrl && !imgError}
        <img
          bind:this={imgEl}
          src={streamUrl}
          alt="Camera stream"
          on:load={onImgLoad}
          on:error={() => (imgError = true)}
        />
        <canvas
          bind:this={canvas}
          class="overlay {editMode ? 'editing' : ''}"
          on:mousedown={onMouseDown}
          on:mousemove={onMouseMove}
          on:mouseup={finishDrag}
          on:mouseleave={finishDrag}
        ></canvas>
        {#if editMode}
          <div class="edit-hint">
            {zones.length > 0
              ? `${zones.length} zone${zones.length > 1 ? 's' : ''} · click to remove · drag to add`
              : 'Drag to draw exclusion zone'}
          </div>
        {/if}
      {:else}
        <div class="placeholder">
          <span class="placeholder-icon"><CameraIcon size={32} strokeWidth={1.7} /></span>
          <span>{connected && imgError ? 'Camera unavailable' : 'Waiting for printer…'}</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px 9px;
    cursor: pointer;
    user-select: none;
    border-bottom: 1px solid var(--border);
  }
  .card-header:hover { background: var(--surface2); }

  .header-left {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--muted);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .card-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted);
  }

  .chevron {
    color: var(--muted);
    transition: transform 0.2s;
    flex-shrink: 0;
  }
  .chevron.up { transform: rotate(-180deg); }

  .hdr-btn {
    padding: 3px 9px;
    font-size: 11px;
    font-weight: 500;
    background: var(--surface2);
    color: var(--muted);
    border: 1px solid var(--border2);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    white-space: nowrap;
  }
  .hdr-btn:hover { background: var(--border); color: var(--text); }
  .hdr-btn.active { color: var(--text); border-color: var(--border2); }
  .hdr-btn.danger { color: var(--danger); border-color: rgba(192,57,74,0.4); }
  .hdr-btn.danger:hover { background: var(--danger-dim); }

  .saved-lbl {
    font-size: 11px;
    color: var(--success);
    font-weight: 500;
  }

  .cam-wrap {
    aspect-ratio: 16 / 9;
    background: #09090c;
    position: relative;
    overflow: hidden;
    min-height: 140px;
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  /* Canvas over the img */
  .overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .overlay.editing {
    pointer-events: auto;
    cursor: crosshair;
  }

  .edit-hint {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(0,0,0,0.7);
    color: rgba(210,210,230,0.9);
    font-size: 10px;
    padding: 3px 10px;
    border-radius: 10px;
    white-space: nowrap;
    pointer-events: none;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--muted);
  }
  .placeholder-icon { color: var(--border2); display: inline-flex; }
  .placeholder span {
    font-size: 12px;
    opacity: 0.6;
    text-align: center;
  }
</style>
