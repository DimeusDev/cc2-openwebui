<script lang="ts">
  import { onMount } from 'svelte';
  import { getLogs, deleteLogs, type LogEntry } from '../../api';

  let logs: LogEntry[] = [];
  let logsLoading = false;
  let logsClearing = false;
  let logFilter: 'all' | 'error' | 'print' | 'detection' = 'all';

  let hoveredSnapshot: string | null = null;
  let hoverPos = { x: 0, y: 0 };

  onMount(() => { loadLogs(); });

  async function loadLogs() {
    logsLoading = true;
    try {
      const result = await getLogs();
      logs = (result.logs ?? []).slice().reverse();
    } catch {
      logs = [];
    } finally {
      logsLoading = false;
    }
  }

  async function clearLogs() {
    logsClearing = true;
    try {
      await deleteLogs();
      logs = [];
    } catch {
    } finally {
      logsClearing = false;
    }
  }

  $: filteredLogs = logs.filter((l) => {
    if (logFilter === 'all') return true;
    if (logFilter === 'error') return l.kind.includes('Error') || l.kind.includes('Failure') || l.kind.includes('Disconnected');
    if (logFilter === 'print') return l.kind.startsWith('Print');
    if (logFilter === 'detection') return l.kind === 'DetectionLogged' || l.kind.includes('Failure') || l.kind === 'AutoPaused';
    return true;
  });

  function kindClass(kind: string): string {
    if (kind.includes('Connected')) return 'ok';
    if (kind.includes('Disconnected')) return 'warn';
    if (kind.includes('Error') || kind.includes('Failure')) return 'err';
    if (kind === 'DetectionLogged') return 'warn';
    if (kind === 'PrintStarted') return 'ok';
    if (kind.startsWith('Command')) return 'info';
    return 'muted';
  }

  function formatLogTime(ts: number): string {
    if (!ts) return '--';
    const d = new Date(ts * 1000);
    return d.toLocaleTimeString(undefined, { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  function handleLogHover(e: MouseEvent, snapshot: string | null) {
    if (!snapshot) { hoveredSnapshot = null; return; }
    hoveredSnapshot = snapshot;
    hoverPos = { x: e.clientX, y: e.clientY };
  }
</script>

<div class="logs-wrap">
  <div class="logs-toolbar">
    <div class="log-filters">
      <button class="filter-chip" class:active={logFilter === 'all'} on:click={() => (logFilter = 'all')}>All</button>
      <button class="filter-chip" class:active={logFilter === 'error'} on:click={() => (logFilter = 'error')}>Errors</button>
      <button class="filter-chip" class:active={logFilter === 'print'} on:click={() => (logFilter = 'print')}>Print events</button>
      <button class="filter-chip" class:active={logFilter === 'detection'} on:click={() => (logFilter = 'detection')}>Detection</button>
    </div>
    <div class="logs-meta">
      <span class="logs-count">{filteredLogs.length}</span>
      <button class="icon-btn" on:click={loadLogs} disabled={logsLoading} title="Refresh">
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none" class:spin={logsLoading}>
          <path d="M13.5 4.5A6 6 0 1 0 14 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" fill="none"/>
          <path d="M10 4.5h3.5V1" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <button class="icon-btn danger" on:click={clearLogs} disabled={logsClearing || logs.length === 0} title="Clear all logs">
        <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
          <path d="M3 4h10M5 4V3h6v1M6 7v5M10 7v5M4 4l1 9h6l1-9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>
  </div>

  {#if filteredLogs.length === 0 && !logsLoading}
    <div class="empty">
      <svg width="28" height="28" viewBox="0 0 24 24" fill="none">
        <rect x="4" y="4" width="16" height="16" rx="2" stroke="currentColor" stroke-width="1.3" opacity="0.5"/>
        <path d="M8 9h8M8 13h5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" opacity="0.5"/>
      </svg>
      <div>No events match this filter yet.</div>
    </div>
  {:else}
    <div class="logs-list" role="list" on:mouseleave={() => (hoveredSnapshot = null)}>
      {#each filteredLogs as entry}
        <div
          class="log-entry"
          class:has-snap={!!entry.snapshot}
          role="listitem"
          on:mouseenter={(e) => handleLogHover(e, entry.snapshot ?? null)}
          on:mousemove={(e) => { if (entry.snapshot) hoverPos = { x: e.clientX, y: e.clientY }; }}
          on:mouseleave={() => { if (hoveredSnapshot === entry.snapshot) hoveredSnapshot = null; }}
        >
          <span class="log-time mono">{formatLogTime(entry.timestamp)}</span>
          <span class="log-kind {kindClass(entry.kind)}">{entry.kind}</span>
          <span class="log-msg">{entry.message}</span>
          {#if entry.snapshot}
            <span class="log-snap-icon" title="Hover to preview snapshot">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="none"><rect x="2" y="3" width="12" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2"/><circle cx="6" cy="7" r="1.5" stroke="currentColor" stroke-width="1"/><path d="M2 11l3-3 2 2 3-3 4 4" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/></svg>
            </span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if hoveredSnapshot}
  <div class="snap-tooltip" style="left: {hoverPos.x + 12}px; top: {hoverPos.y - 100}px;">
    <img src="/snapshots/{hoveredSnapshot}" alt="Detection snapshot" />
  </div>
{/if}

<style>
  .logs-wrap {
    display: flex; flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
  }
  .logs-toolbar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 9px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface2);
    flex-shrink: 0;
  }
  .log-filters { display: flex; gap: 4px; }
  .filter-chip {
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    font-size: 11px;
    color: var(--muted);
    background: transparent;
    border: 1px solid transparent;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }
  .filter-chip:hover { color: var(--text); background: var(--surface3); }
  .filter-chip.active { color: var(--accent); background: var(--accent-dim); border-color: rgba(45,135,240,0.35); }

  .logs-meta { display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--muted); }
  .logs-count {
    font-variant-numeric: tabular-nums;
    padding: 2px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
  }
  .icon-btn {
    width: 26px; height: 26px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .icon-btn:hover:not(:disabled) { color: var(--text); border-color: var(--border2); background: var(--surface3); }
  .icon-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .icon-btn.danger:hover:not(:disabled) { color: var(--danger); border-color: var(--danger); background: var(--danger-dim); }

  .spin { animation: spin 0.9s linear infinite; transform-origin: center; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .logs-list { overflow-y: auto; max-height: 420px; }
  .log-entry {
    display: grid;
    grid-template-columns: 72px 160px 1fr auto;
    gap: 12px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    line-height: 1.4;
    align-items: center;
    transition: background 0.12s;
  }
  .log-entry:last-child { border-bottom: none; }
  .log-entry:hover { background: var(--surface2); }
  .log-entry.has-snap { cursor: default; }
  .log-time { color: var(--muted); font-size: 11px; }
  .log-kind {
    display: inline-flex; align-items: center;
    padding: 2px 8px;
    font-size: 10.5px; font-weight: 600; letter-spacing: 0.02em;
    border-radius: var(--radius-pill);
    background: var(--surface2); color: var(--muted); border: 1px solid var(--border);
    justify-self: start; white-space: nowrap;
  }
  .log-kind.ok { color: var(--success); background: var(--success-dim); border-color: rgba(74,140,92,0.35); }
  .log-kind.warn { color: var(--warning); background: var(--warning-dim); border-color: rgba(192,120,40,0.35); }
  .log-kind.err { color: var(--danger); background: var(--danger-dim); border-color: rgba(192,57,74,0.35); }
  .log-kind.info { color: var(--accent); background: var(--accent-dim); border-color: rgba(45,135,240,0.35); }
  .log-msg { color: var(--text); word-break: break-word; }
  .log-snap-icon { color: var(--accent); display: inline-flex; align-items: center; }

  .empty {
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    flex: 1; padding: 40px 20px;
    color: var(--muted); font-size: 12.5px; gap: 10px;
  }

  .snap-tooltip {
    position: fixed;
    z-index: 200;
    pointer-events: none;
    border-radius: var(--radius-md);
    border: 1px solid var(--border2);
    box-shadow: 0 8px 30px rgba(0,0,0,0.5);
    overflow: hidden;
    background: var(--bg-deep);
  }
  .snap-tooltip img { display: block; width: 240px; height: auto; }
</style>
