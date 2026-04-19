<script lang="ts">
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Modal from '../Modal.svelte';
  import ConfirmModal from '../ConfirmModal.svelte';
  import {
    runDetection, cameraSnapshotUrl, listSnapshots, deleteAllSnapshots, deleteSnapshot, snapshotUrl,
    type RunDetectionResult, type SnapshotEntry,
  } from '../../api';
  import { toErrorMessage } from '../errors';

  export let detection: {
    obico_url: string;
    notify_threshold: number;
    pause_threshold: number;
    interval_secs: number;
    confirmation_frames: number;
  };

  let snapshots: SnapshotEntry[] = [];
  let snapshotTotal = 0;
  let snapshotTotalBytes = 0;
  let snapshotsLoading = false;
  let purgeConfirmOpen = false;
  let purging = false;
  let snapLightbox: SnapshotEntry | null = null;

  let detTestOpen = false;
  let detTestRunning = false;
  let detTestResult: RunDetectionResult | null = null;
  let detTestError = '';
  let detTestFrameUrl = '';

  onMount(() => { loadSnapshots(); });

  async function loadSnapshots() {
    snapshotsLoading = true;
    try {
      const res = await listSnapshots(0, 50);
      snapshots = res.snapshots;
      snapshotTotal = res.total;
      snapshotTotalBytes = res.total_bytes;
    } catch {
      snapshots = [];
    } finally {
      snapshotsLoading = false;
    }
  }

  async function doPurgeSnapshots() {
    purging = true;
    try {
      await deleteAllSnapshots();
      snapshots = [];
      snapshotTotal = 0;
      snapshotTotalBytes = 0;
    } catch {
    } finally {
      purging = false;
      purgeConfirmOpen = false;
    }
  }

  async function doDeleteSnapshot(filename: string) {
    await deleteSnapshot(filename);
    snapshots = snapshots.filter((s) => s.filename !== filename);
    snapshotTotal = Math.max(0, snapshotTotal - 1);
  }

  function openDetTest() {
    detTestOpen = true;
    detTestResult = null;
    detTestError = '';
    detTestRunning = false;
    detTestFrameUrl = cameraSnapshotUrl() + '?t=' + Date.now();
  }

  async function doDetTest() {
    detTestRunning = true;
    detTestError = '';
    detTestResult = null;
    try {
      detTestResult = await runDetection();
    } catch (e) {
      detTestError = toErrorMessage(e) || 'Detection failed';
    }
    detTestRunning = false;
  }

  function formatBytes(b: number): string {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
    return `${(b / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatSnapDate(ts: number): string {
    if (!ts) return '--';
    const d = new Date(ts * 1000);
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) + ' ' +
      d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false });
  }
</script>

<div class="group">
  <div class="row">
    <div class="row-label">
      <div class="row-title">Obico ML URL</div>
      <div class="row-sub">Endpoint of the local Obico detection container.</div>
    </div>
    <input id="obico" class="input mono row-input" type="text" bind:value={detection.obico_url} />
  </div>
  <div class="row col">
    <div class="row-label">
      <div class="row-title">
        Notify threshold
        <span class="pill-val mono">{(detection.notify_threshold * 100).toFixed(0)}%</span>
      </div>
      <div class="row-sub">Score above which a push notification is sent.</div>
    </div>
    <div class="slider-block">
      <div class="slider-ticks"><span>Sensitive</span><span>Strict</span></div>
      <input type="range" min="0" max="1" step="0.01" bind:value={detection.notify_threshold} class="range" />
    </div>
  </div>
  <div class="row col">
    <div class="row-label">
      <div class="row-title">
        Pause threshold
        <span class="pill-val mono">{(detection.pause_threshold * 100).toFixed(0)}%</span>
      </div>
      <div class="row-sub">Score above which the print is automatically paused.</div>
      {#if detection.pause_threshold < detection.notify_threshold}
        <div class="row-warn">Pause threshold must be ≥ notify threshold.</div>
      {/if}
    </div>
    <div class="slider-block">
      <div class="slider-ticks"><span>Sensitive</span><span>Strict</span></div>
      <input type="range" min="0" max="1" step="0.01" bind:value={detection.pause_threshold} class="range range-danger" />
    </div>
  </div>
  <div class="row">
    <div class="row-label">
      <div class="row-title">Check interval</div>
      <div class="row-sub">Seconds between frame analyses.</div>
    </div>
    <div class="input-suffix">
      <input id="interval" class="input mono row-input short" type="number" bind:value={detection.interval_secs} min="5" max="60" />
      <span class="suffix">seconds</span>
    </div>
  </div>
</div>

<div class="group">
  <div class="row">
    <div class="row-label">
      <div class="row-title">Test detection</div>
      <div class="row-sub">Run the ML model on the current camera frame and see the result.</div>
    </div>
    <button class="btn sm" on:click={openDetTest}>
      <svg width="12" height="12" viewBox="0 0 16 16" fill="none"><path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z" stroke="currentColor" stroke-width="1.3"/><circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.3"/></svg>
      Run test
    </button>
  </div>
</div>

<div class="group snaps-group">
  <div class="snaps-header">
    <div>
      <div class="snaps-title">Detection snapshots</div>
      <div class="snaps-meta">
        {#if snapshotsLoading}
          <span class="muted-text">Loading…</span>
        {:else}
          <span class="muted-text">{snapshotTotal} snapshot{snapshotTotal !== 1 ? 's' : ''} · {formatBytes(snapshotTotalBytes)}</span>
        {/if}
      </div>
    </div>
    <div class="snaps-acts">
      <button class="icon-btn" on:click={loadSnapshots} disabled={snapshotsLoading} title="Refresh">
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" class:spin={snapshotsLoading}>
          <path d="M13.5 4.5A6 6 0 1 0 14 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" fill="none"/>
          <path d="M10 4.5h3.5V1" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      {#if snapshotTotal > 0}
        <button class="btn xs danger" on:click={() => (purgeConfirmOpen = true)}>
          <svg width="11" height="11" viewBox="0 0 16 16" fill="none">
            <path d="M3 4h10M6.5 4V2.5h3V4M5 4l.5 9h5l.5-9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          Purge all
        </button>
      {/if}
    </div>
  </div>

  {#if !snapshotsLoading && snapshots.length === 0}
    <div class="snaps-empty">No detection snapshots yet.</div>
  {:else}
    <div class="snaps-grid">
      {#each snapshots as snap}
        <div class="snap-card" on:click={() => (snapLightbox = snap)} role="button" tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && (snapLightbox = snap)}>
          <div class="snap-thumb-wrap">
            <img src={snapshotUrl(snap.filename)} alt="Detection snapshot" class="snap-thumb" loading="lazy" />
            {#if snap.score_pct !== null}
              <span class="snap-score" class:high={snap.score_pct >= 50}>{snap.score_pct}%</span>
            {/if}
            <button class="snap-del-btn" on:click|stopPropagation={() => doDeleteSnapshot(snap.filename)} title="Delete this snapshot">
              <svg width="11" height="11" viewBox="0 0 14 14" fill="none"><path d="M2 3.5h10M4.5 3.5V2.5h5v1M5.5 6v4.5M8.5 6v4.5M3 3.5l.8 8h6.4l.8-8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
          <div class="snap-info">{formatSnapDate(snap.mtime)}</div>
        </div>
      {/each}
    </div>
    {#if snapshotTotal > snapshots.length}
      <div class="snaps-more">Showing {snapshots.length} of {snapshotTotal}</div>
    {/if}
  {/if}
</div>

{#if detTestOpen}
  <Modal open={detTestOpen} onClose={() => (detTestOpen = false)} zIndex={110}>
    <div class="modal-sheet det-sheet" role="dialog" aria-modal="true" in:fly={{ y: 10, duration: 200, easing: cubicOut }}>
      <div class="modal-head">
        <span class="modal-title">Test detection</span>
        <button class="modal-close" on:click={() => (detTestOpen = false)} aria-label="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M3 3l8 8M11 3L3 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <div class="det-body">
        <div class="det-frame-wrap">
          <img src={detTestFrameUrl} alt="Camera frame" class="det-frame" />
          {#if detTestResult}
            <svg class="det-overlay" viewBox="0 0 1 1" preserveAspectRatio="none">
              {#each detTestResult.detections as d}
                <rect x={d.x1} y={d.y1} width={d.x2 - d.x1} height={d.y2 - d.y1}
                  fill="none" stroke="var(--danger)" stroke-width="0.003"/>
                <rect x={d.x1} y={Math.max(0, d.y1 - 0.035)} width="0.08" height="0.03"
                  fill="var(--danger)" rx="0.004"/>
                <text x={d.x1 + 0.005} y={Math.max(0.02, d.y1 - 0.012)}
                  fill="#fff" font-size="0.022" font-weight="600">{(d.confidence * 100).toFixed(0)}%</text>
              {/each}
            </svg>
          {/if}
        </div>
        <div class="det-controls">
          {#if detTestResult}
            <div class="det-score" class:high={detTestResult.score >= 0.5} class:low={detTestResult.score < 0.5 && detTestResult.score > 0}>
              Score: <span class="mono">{(detTestResult.score * 100).toFixed(1)}%</span>
              - {detTestResult.detections.length} detection{detTestResult.detections.length !== 1 ? 's' : ''}
            </div>
          {/if}
          {#if detTestError}
            <div class="det-err">{detTestError}</div>
          {/if}
          <div class="det-actions">
            <button class="btn" on:click={() => { detTestFrameUrl = cameraSnapshotUrl() + '?t=' + Date.now(); detTestResult = null; }}>
              Refresh frame
            </button>
            <button class="btn primary" on:click={doDetTest} disabled={detTestRunning}>
              {#if detTestRunning}<span class="spinner sm"></span> Running…{:else}Run detection{/if}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Modal>
{/if}

<ConfirmModal
  open={purgeConfirmOpen}
  onClose={() => (purgeConfirmOpen = false)}
  onConfirm={doPurgeSnapshots}
  label="Purge all snapshots"
  title="Purge all snapshots"
  description="This will permanently delete all {snapshotTotal} detection snapshot{snapshotTotal !== 1 ? 's' : ''} ({formatBytes(snapshotTotalBytes)}). This cannot be undone."
  confirmLabel={purging ? 'Purging…' : 'Purge all'}
  variant="danger"
  disabled={purging}
>
  <svelte:fragment slot="icon">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M3 4h10M6.5 4V2.5h3V4M5 4l.5 9h5l.5-9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </svelte:fragment>
</ConfirmModal>

{#if snapLightbox}
  <Modal open={true} onClose={() => (snapLightbox = null)} zIndex={120}>
    <div class="snap-lb" role="dialog" aria-modal="true" in:fly={{ y: 8, duration: 200, easing: cubicOut }}>
      <div class="snap-lb-head">
        <div class="snap-lb-meta">
          {#if snapLightbox.score_pct !== null}
            <span class="snap-lb-score" class:danger={snapLightbox.score_pct >= 50}>{snapLightbox.score_pct}%</span>
          {/if}
          <span class="snap-lb-date">{formatSnapDate(snapLightbox.mtime)}</span>
          <span class="snap-lb-size">{formatBytes(snapLightbox.size)}</span>
        </div>
        <button class="modal-close" on:click={() => (snapLightbox = null)} aria-label="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M3 3l8 8M11 3L3 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <div class="snap-lb-img-wrap">
        <img src={snapshotUrl(snapLightbox.filename)} alt="Detection snapshot" class="snap-lb-img" />
        {#if snapLightbox.boxes && snapLightbox.boxes.length > 0}
          <svg class="snap-lb-overlay" viewBox="0 0 1 1" preserveAspectRatio="none">
            {#each snapLightbox.boxes as b}
              <rect x={b.x1} y={b.y1} width={b.x2 - b.x1} height={b.y2 - b.y1}
                fill="none" stroke="var(--danger)" stroke-width="0.003"/>
              <rect x={b.x1} y={Math.max(0, b.y1 - 0.04)} width="0.09" height="0.032"
                fill="var(--danger)" rx="0.004"/>
              <text x={b.x1 + 0.007} y={Math.max(0.02, b.y1 - 0.015)}
                fill="#fff" font-size="0.023" font-weight="bold" font-family="monospace">
                {(b.confidence * 100).toFixed(0)}%
              </text>
            {/each}
          </svg>
        {/if}
      </div>
      {#if snapLightbox.boxes && snapLightbox.boxes.length > 0}
        <div class="snap-lb-footer">
          {snapLightbox.boxes.length} detection{snapLightbox.boxes.length !== 1 ? 's' : ''}
          · Max {(Math.max(...snapLightbox.boxes.map(b => b.confidence)) * 100).toFixed(0)}% confidence
        </div>
      {/if}
    </div>
  </Modal>
{/if}

<style>
  .group {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
  }
  .snaps-group { padding: 0; }

  .row {
    display: grid;
    grid-template-columns: 1fr 220px;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
  .row:first-child { border-top: none; }
  .row.col { grid-template-columns: 1fr; gap: 10px; align-items: stretch; }
  .row-label { min-width: 0; }
  .row-title {
    font-size: 12.5px; font-weight: 500; color: var(--text);
    display: flex; align-items: center; gap: 8px;
  }
  .row-sub { font-size: 11.5px; color: var(--muted); margin-top: 2px; line-height: 1.45; }
  .row-warn { margin-top: 4px; font-size: 11px; color: var(--danger); }
  .row-input { width: 100%; }
  .row-input.short { max-width: 140px; justify-self: end; }

  .pill-val {
    display: inline-flex; align-items: center;
    padding: 2px 7px; font-size: 10.5px;
    color: var(--accent); background: var(--accent-dim);
    border: 1px solid rgba(45,135,240,0.35);
    border-radius: var(--radius-pill);
  }
  .input-suffix { display: flex; align-items: center; gap: 8px; justify-self: end; }
  .suffix { font-size: 11.5px; color: var(--muted); }

  .slider-block { display: flex; flex-direction: column; gap: 4px; }
  .slider-ticks {
    display: flex; justify-content: space-between;
    font-size: 10px; color: var(--muted2); letter-spacing: 0.05em; text-transform: uppercase;
  }
  .range { width: 100%; accent-color: var(--accent); height: 4px; }
  .range-danger { accent-color: var(--danger); }

  .icon-btn {
    width: 26px; height: 26px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-md); border: 1px solid var(--border);
    background: var(--surface); color: var(--muted);
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .icon-btn:hover:not(:disabled) { color: var(--text); border-color: var(--border2); background: var(--surface3); }
  .icon-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .spin { animation: spin 0.9s linear infinite; transform-origin: center; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .muted-text { font-size: 12.5px; color: var(--muted); }

  .snaps-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 11px 16px; border-bottom: 1px solid var(--border);
    background: var(--surface2);
  }
  .snaps-title { font-size: 12.5px; font-weight: 500; color: var(--text); }
  .snaps-meta { margin-top: 2px; }
  .snaps-acts { display: flex; align-items: center; gap: 8px; }
  .snaps-empty { padding: 20px 16px; font-size: 12.5px; color: var(--muted); text-align: center; }
  .snaps-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 8px; padding: 12px 14px;
  }
  .snap-card { display: flex; flex-direction: column; gap: 4px; cursor: pointer; }
  .snap-card:hover .snap-thumb-wrap { border-color: var(--border2); }
  .snap-card:focus { outline: none; }
  .snap-card:focus .snap-thumb-wrap { border-color: var(--accent); }
  .snap-thumb-wrap {
    position: relative; border-radius: 5px; overflow: hidden;
    background: var(--bg-deep); border: 1px solid var(--border); aspect-ratio: 4/3;
  }
  .snap-thumb { display: block; width: 100%; height: 100%; object-fit: cover; }
  .snap-score {
    position: absolute; bottom: 4px; right: 4px;
    font-size: 10px; font-weight: 700; padding: 1px 5px; border-radius: 3px;
    background: rgba(0,0,0,0.65); color: var(--warning); font-variant-numeric: tabular-nums;
  }
  .snap-score.high { color: var(--danger); }
  .snap-del-btn {
    position: absolute; top: 4px; right: 4px;
    width: 20px; height: 20px;
    display: none; align-items: center; justify-content: center;
    background: rgba(0,0,0,0.55); border: none; border-radius: 4px;
    color: #fff; cursor: pointer; padding: 0;
  }
  .snap-del-btn:hover { background: var(--danger); }
  .snap-thumb-wrap:hover .snap-del-btn { display: inline-flex; }
  .snap-info { font-size: 10px; color: var(--muted2); text-align: center; line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .snaps-more { padding: 8px 14px; font-size: 11px; color: var(--muted2); border-top: 1px solid var(--border); text-align: center; }

  .btn.xs { font-size: 10.5px; padding: 3px 8px; border-radius: var(--radius-sm); }
  .btn.danger { color: var(--danger); border-color: rgba(192,57,74,0.4); background: var(--danger-dim); }

  /* Detection test modal */
  .det-sheet { width: min(640px, calc(100vw - 40px)); border-radius: 12px; border: 1px solid var(--border2); box-shadow: 0 24px 80px -20px rgba(0,0,0,0.65), 0 4px 14px rgba(0,0,0,0.3); }
  .det-body { padding: 16px 18px 18px; display: flex; flex-direction: column; gap: 14px; }
  .det-frame-wrap { position: relative; border-radius: var(--radius-md); overflow: hidden; background: var(--bg-deep); border: 1px solid var(--border); }
  .det-frame { display: block; width: 100%; height: auto; min-height: 200px; object-fit: contain; background: #000; }
  .det-overlay { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  .det-controls { display: flex; flex-direction: column; gap: 10px; }
  .det-score { font-size: 13px; font-weight: 500; color: var(--muted); padding: 8px 12px; background: var(--surface2); border-radius: var(--radius); border: 1px solid var(--border); }
  .det-score.high { color: var(--danger); border-color: rgba(192,57,74,0.35); background: var(--danger-dim); }
  .det-score.low { color: var(--warning); border-color: rgba(192,120,40,0.35); background: var(--warning-dim); }
  .det-err { font-size: 12px; color: var(--danger); padding: 8px 12px; background: var(--danger-dim); border-radius: var(--radius); }
  .det-actions { display: flex; justify-content: flex-end; gap: 8px; }

  .spinner.sm { width: 12px; height: 12px; border: 2px solid rgba(255,255,255,0.3); border-top-color: #fff; border-radius: 50%; animation: spin 0.8s linear infinite; display: inline-block; }

  /* Lightbox */
  .snap-lb { background: var(--surface); border: 1px solid var(--border2); border-radius: 10px; width: min(520px, calc(100vw - 32px)); box-shadow: 0 24px 80px rgba(0,0,0,0.65); overflow: hidden; }
  .snap-lb-head { display: flex; align-items: center; justify-content: space-between; padding: 11px 14px; border-bottom: 1px solid var(--border); gap: 8px; }
  .snap-lb-meta { display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0; }
  .snap-lb-score { font-size: 12px; font-weight: 700; font-family: var(--font-mono); padding: 2px 8px; border-radius: var(--radius-pill); background: var(--warning-dim); color: var(--warning); border: 1px solid rgba(192,120,40,0.35); }
  .snap-lb-score.danger { background: var(--danger-dim); color: var(--danger); border-color: rgba(192,57,74,0.35); }
  .snap-lb-date { font-size: 11.5px; color: var(--muted); }
  .snap-lb-size { font-size: 11px; color: var(--muted2); }
  .snap-lb-img-wrap { position: relative; background: #000; line-height: 0; }
  .snap-lb-img { display: block; width: 100%; height: auto; max-height: calc(100vh - 200px); object-fit: contain; }
  .snap-lb-overlay { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  .snap-lb-footer { padding: 8px 14px; font-size: 11.5px; color: var(--muted); text-align: center; border-top: 1px solid var(--border); background: var(--surface2); }

  @media (max-width: 700px) {
    .row { grid-template-columns: 1fr; gap: 8px; }
    .row-input.short { max-width: none; justify-self: stretch; }
    .input-suffix { justify-self: stretch; }
  }
</style>
