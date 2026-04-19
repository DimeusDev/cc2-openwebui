<script lang="ts">
  import { printer } from '../stores';
  import { pausePrint, resumePrint, stopPrint } from '../api';
  import ConfirmModal from './ConfirmModal.svelte';
  import { toErrorMessage } from './errors';

  $: s = $printer.state;
  $: printStatus = s?.print_status;
  $: machineStatus = s?.machine_status;
  $: printState = printStatus?.state ?? '';
  $: progress = machineStatus?.progress ?? 0;
  $: filename = printStatus?.filename ?? '';
  $: layer = printStatus?.current_layer ?? 0;
  $: remainingSec = printStatus?.remaining_time_sec ?? 0;
  $: printDuration = printStatus?.print_duration ?? 0;
  $: isPrinting = printState === 'printing';
  $: isPaused = printState === 'paused';
  $: isActive = isPrinting || isPaused;

  let stopping = false;
  let error = '';
  let showPauseModal = false;
  let showStopModal = false;

  function formatTime(sec: number): string {
    if (!sec) return '--';
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    if (h > 0) return `-${h}h${m}m`;
    return `-${m}m`;
  }

  function formatElapsed(sec: number): string {
    if (!sec) return '--';
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  }

  function shortName(name: string): string {
    if (!name) return 'No active print';
    return name.replace(/\.gcode$/i, '');
  }

  async function confirmPause() {
    showPauseModal = false;
    error = '';
    try { await pausePrint(); } catch (e) { error = toErrorMessage(e); }
  }

  async function handleResume() {
    error = '';
    try { await resumePrint(); } catch (e) { error = toErrorMessage(e); }
  }

  async function confirmStop() {
    showStopModal = false;
    error = '';
    stopping = true;
    try { await stopPrint(); } catch (e) { error = toErrorMessage(e); }
    stopping = false;
  }
</script>

<ConfirmModal
  open={showPauseModal}
  onClose={() => (showPauseModal = false)}
  onConfirm={confirmPause}
  label="Confirm pause"
  title="Pause print?"
  description="The printer will pause."
  confirmLabel="Pause"
  variant="warn"
>
  <svelte:fragment slot="icon">
    <svg width="22" height="22" viewBox="0 0 22 22" fill="none">
      <rect x="4" y="3" width="5" height="16" rx="1.5" fill="currentColor"/>
      <rect x="13" y="3" width="5" height="16" rx="1.5" fill="currentColor"/>
    </svg>
  </svelte:fragment>
</ConfirmModal>

<ConfirmModal
  open={showStopModal}
  onClose={() => (showStopModal = false)}
  onConfirm={confirmStop}
  label="Confirm stop"
  title="Stop print?"
  description="This will cancel the current print job. <strong>This cannot be undone.</strong>"
  confirmLabel={stopping ? 'Stopping…' : 'Stop'}
  variant="danger"
  disabled={stopping}
>
  <svelte:fragment slot="icon">
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <rect x="2" y="2" width="16" height="16" rx="3" fill="currentColor"/>
    </svg>
  </svelte:fragment>
</ConfirmModal>

<div class="print-card">
  <div class="print-header">
    <div class="thumb">
      {#if isActive}
        <svg width="52" height="52" viewBox="0 0 52 52" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect width="52" height="52" rx="5" fill="#1a2740"/>
          <path d="M26 10L42 18V34L26 42L10 34V18L26 10Z" stroke="#2d87f0" stroke-width="2" fill="none"/>
          <path d="M26 10V42M10 18L42 18M10 34L42 34" stroke="#2d87f0" stroke-width="1" stroke-dasharray="3 3" opacity="0.5"/>
          <circle cx="26" cy="26" r="4" fill="#2d87f0" opacity="0.8"/>
        </svg>
      {:else}
        <div class="thumb-empty">
          <svg width="36" height="36" viewBox="0 0 36 36" fill="none">
            <rect x="5" y="9" width="26" height="18" rx="2.5" stroke="var(--border2)" stroke-width="1.5" fill="none"/>
            <path d="M11 9V8a7 7 0 0114 0v1" stroke="var(--border2)" stroke-width="1.5" fill="none"/>
          </svg>
        </div>
      {/if}
    </div>

    <div class="job-info">
      <div class="job-top-row">
        <span class="filename" title={filename}>{shortName(filename)}</span>
        <div class="job-badges">
          {#if isPrinting}
            <span class="badge printing">Printing</span>
          {:else if isPaused}
            <span class="badge paused">Paused</span>
          {:else}
            <span class="badge idle">Idle</span>
          {/if}
        </div>
      </div>

      {#if isActive}
        <div class="progress-row">
          <span class="progress-pct">{progress}%</span>
          <div class="progress-track">
            <div class="progress-fill" style="width:{progress}%"></div>
          </div>
        </div>
        <div class="stats-row">
          <span class="stat-item">
            <span class="stat-lbl">Layer</span>
            <span class="stat-val mono">{layer}</span>
          </span>
          <span class="stat-sep">|</span>
          <span class="stat-item">
            <span class="stat-lbl">Elapsed</span>
            <span class="stat-val mono">{formatElapsed(printDuration)}</span>
          </span>
          <span class="stat-sep">|</span>
          <span class="stat-item">
            <span class="stat-lbl">ETA</span>
            <span class="stat-val mono">{formatTime(remainingSec)}</span>
          </span>
        </div>
      {:else}
        <div class="idle-hint">No print job active</div>
      {/if}
    </div>

    {#if isActive}
      <div class="job-controls">
        {#if isPrinting}
          <button class="ctrl-btn pause" on:click={() => showPauseModal = true} title="Pause">
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <rect x="3" y="2.5" width="4.5" height="13" rx="1.5" fill="currentColor"/>
              <rect x="10.5" y="2.5" width="4.5" height="13" rx="1.5" fill="currentColor"/>
            </svg>
          </button>
        {/if}
        {#if isPaused}
          <button class="ctrl-btn resume" on:click={handleResume} title="Resume">
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <path d="M4 3l11 6-11 6V3z" fill="currentColor"/>
            </svg>
          </button>
        {/if}
        <button class="ctrl-btn stop" on:click={() => showStopModal = true} disabled={stopping} title="Stop">
          <svg width="15" height="15" viewBox="0 0 15 15" fill="none">
            <rect x="2" y="2" width="11" height="11" rx="2" fill="currentColor"/>
          </svg>
        </button>
      </div>
    {/if}
  </div>

  {#if error}
    <div class="err">{error}</div>
  {/if}
</div>

<style>
  .print-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .print-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 18px;
  }

  .thumb {
    flex-shrink: 0;
    width: 72px;
    height: 72px;
    border-radius: var(--radius);
    background: var(--surface2);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .thumb-empty {
    opacity: 0.3;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .job-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .job-top-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .filename {
    font-size: 15px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
    color: var(--text);
  }

  .job-badges { flex-shrink: 0; }

  .badge {
    font-size: 11px;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 20px;
    white-space: nowrap;
    letter-spacing: 0.02em;
  }

  .badge.printing {
    background: var(--surface2);
    color: var(--text);
    border: 1px solid var(--border2);
  }

  .badge.paused {
    background: var(--warning-dim);
    color: var(--warning);
    border: 1px solid rgba(240,160,48,0.3);
  }

  .badge.idle {
    background: var(--surface2);
    color: var(--muted);
    border: 1px solid var(--border);
  }

  .progress-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .progress-pct {
    font-size: 14px;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--text);
    min-width: 38px;
    flex-shrink: 0;
  }

  .progress-track {
    flex: 1;
    height: 6px;
    background: var(--surface2);
    border-radius: 3px;
    overflow: hidden;
    border: 1px solid var(--border);
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--accent-hi));
    border-radius: 3px;
    transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 0 8px -1px rgba(45,135,240,0.4);
  }

  .stats-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .stat-item {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .stat-lbl { font-size: 11px; color: var(--muted); }

  .stat-val {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  .stat-sep { color: var(--border2); font-size: 11px; }

  .idle-hint {
    font-size: 13px;
    color: var(--muted);
    font-style: italic;
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .job-controls {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
    align-items: center;
  }

  .ctrl-btn {
    width: 46px;
    height: 46px;
    border-radius: 8px;
    border: 1px solid;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: filter 0.15s, opacity 0.15s;
    cursor: pointer;
  }

  .ctrl-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .ctrl-btn:not(:disabled):hover { filter: brightness(1.2); }

  .ctrl-btn.pause {
    background: var(--warning-dim);
    border-color: rgba(240,160,48,0.4);
    color: var(--warning);
  }

  .ctrl-btn.resume {
    background: var(--surface2);
    border-color: var(--border2);
    color: var(--text);
  }

  .ctrl-btn.stop {
    background: var(--danger-dim);
    border-color: rgba(232,69,90,0.4);
    color: var(--danger);
  }

  .err {
    margin: 0 16px 12px;
    font-size: 12px;
    color: var(--danger);
    padding: 6px 10px;
    background: var(--danger-dim);
    border-radius: var(--radius-sm);
  }
</style>
