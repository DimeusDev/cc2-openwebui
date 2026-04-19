<script lang="ts">
  import { onMount } from 'svelte';
  import { printer, detection, showToast } from '../stores';
  import type { DetectionPoint } from '../stores';
  import { toggleDetection, getDetectionStatus } from '../api';
  import { toErrorMessage } from './errors';
  import Modal from './Modal.svelte';
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  let collapsed = false;
  let gWidth = 0;

  let tooltip: { cx: number; cy: number; point: DetectionPoint } | null = null;
  let modalPoint: DetectionPoint | null = null;

  $: detEnabled = $detection.enabled;
  $: notifyT = $detection.notify_threshold;
  $: pauseT = $detection.pause_threshold;
  $: score = $printer.detection_score;
  $: ps = $printer.state?.print_status;
  $: isPrinting = ps?.state === 'printing' || ps?.state === 'paused';
  $: currentFile = ps?.filename ?? null;

  $: history = (() => {
    const all = ($printer.detection_history as DetectionPoint[]).filter(p => p.score > 0);
    if (isPrinting && currentFile) {
      const forPrint = all.filter(p => !p.print_filename || p.print_filename === currentFile);
      return forPrint;
    }
    return all;
  })();

  $: showCleanBanner = isPrinting && history.length === 0;
  $: peak = history.length ? Math.max(...history.map(p => p.score)) : 0;

  $: scoreColor = score >= pauseT ? 'var(--danger)'
                : score >= notifyT ? 'var(--warning)'
                : 'var(--success)';
  $: scoreLabel = score >= pauseT ? 'Failure Risk'
                : score >= notifyT ? 'Attention'
                : 'Normal';

  const GH = 90;
  const PT = 10;
  const PB = 8;
  const PH = GH - PT - PB;

  function mapX(i: number, n: number): number {
    if (n <= 1) return gWidth / 2;
    return (i / (n - 1)) * gWidth;
  }
  function mapY(v: number): number {
    return PT + (1 - Math.max(0, Math.min(1, v))) * PH;
  }

  function linePath(pts: DetectionPoint[]): string {
    if (pts.length < 2) return '';
    return pts.map((p, i) =>
      `${i === 0 ? 'M' : 'L'} ${mapX(i, pts.length).toFixed(1)},${mapY(p.score).toFixed(1)}`
    ).join(' ');
  }

  function areaPath(pts: DetectionPoint[]): string {
    if (pts.length < 2) return '';
    const lx = mapX(pts.length - 1, pts.length).toFixed(1);
    return `${linePath(pts)} L ${lx},${GH} L 0,${GH} Z`;
  }

  function dotColor(s: number): string {
    return s >= pauseT ? 'var(--danger)' : s >= notifyT ? 'var(--warning)' : 'var(--success)';
  }

  function fmtTime(ts: number): string {
    return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }
  function fmtDateTime(ts: number): string {
    const d = new Date(ts * 1000);
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) + ' · ' +
      d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
  }

  function handleDotEnter(e: MouseEvent, point: DetectionPoint) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    tooltip = { cx: r.left + r.width / 2, cy: r.top, point };
  }
  function handleDotLeave(e: MouseEvent) {
    tooltip = null;
  }
  function handleDotClick(point: DetectionPoint) {
    tooltip = null;
    if (point.snapshot) modalPoint = point;
  }

  async function handleToggle() {
    try { await toggleDetection(); } catch (e) { showToast(toErrorMessage(e) || 'Failed to toggle detection', 'error'); }
  }

  onMount(async () => {
    try { const s = await getDetectionStatus(); detection.set(s); } catch (e) { showToast(toErrorMessage(e) || 'Failed to load detection status', 'error'); }
  });
</script>


<div class="card">
  <div class="card-header" role="button" tabindex="0"
    on:click={() => (collapsed = !collapsed)}
    on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && (collapsed = !collapsed)}
  >
    <span class="card-title">Failure Detection</span>
    <div class="header-right">
      <div class="toggle-wrap" on:click|stopPropagation on:keydown|stopPropagation role="none">
        <label class="toggle">
          <input type="checkbox" checked={detEnabled} on:change={handleToggle} />
          <span class="knob"></span>
        </label>
      </div>
      <svg class="chevron" class:up={collapsed} width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M3 5l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
  </div>

  {#if !collapsed}
    {#if detEnabled}
      <div class="body">

        <div class="score-strip">
          <div class="score-badge" style="--c:{scoreColor}">
            <span class="score-val">{(score * 100).toFixed(0)}<span class="score-unit">%</span></span>
          </div>
          <div class="score-right">
            <span class="score-label" style="color:{scoreColor}">{scoreLabel}</span>
            <div class="thr-row">
              <span class="thr-chip thr-notify">
                <span class="thr-dot"></span>{(notifyT * 100).toFixed(0)}% notify
              </span>
              <span class="thr-chip thr-pause">
                <span class="thr-dot"></span>{(pauseT * 100).toFixed(0)}% pause
              </span>
            </div>
          </div>
        </div>

        <div class="graph-outer">
          {#if showCleanBanner}
            <div class="clean-banner" in:fade={{ duration: 200 }}>
              <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="6.5" stroke="currentColor" stroke-width="1.3" fill="none"/>
                <path d="M4.5 8.5l2.5 2.5 4.5-5.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              No Print Failure detected so far.
            </div>
          {:else}
            <div class="graph-wrap" bind:clientWidth={gWidth}>
              {#if gWidth > 0}
                <svg width={gWidth} height={GH} class="graph-svg">
                  {#each [0, 0.5, 1] as gridV}
                    <line x1="0" y1={mapY(gridV)} x2={gWidth} y2={mapY(gridV)}
                      stroke="var(--border)" stroke-width={gridV === 0 || gridV === 1 ? 0.5 : 0.4} opacity="0.5"/>
                  {/each}

                  {#if notifyT > 0 && notifyT < 1}
                    <line x1="0" y1={mapY(notifyT)} x2={gWidth} y2={mapY(notifyT)}
                      stroke="var(--warning)" stroke-width="1" stroke-dasharray="4 3" opacity="0.8"/>
                    <text x={gWidth - 3} y={mapY(notifyT) - 3}
                      font-size="8.5" fill="var(--warning)" text-anchor="end" opacity="0.9"
                      font-family="var(--font-mono)">
                      {(notifyT * 100).toFixed(0)}%
                    </text>
                  {/if}

                  {#if pauseT > 0 && pauseT < 1}
                    <line x1="0" y1={mapY(pauseT)} x2={gWidth} y2={mapY(pauseT)}
                      stroke="var(--danger)" stroke-width="1" stroke-dasharray="4 3" opacity="0.8"/>
                    <text x={gWidth - 3} y={mapY(pauseT) - 3}
                      font-size="8.5" fill="var(--danger)" text-anchor="end" opacity="0.9"
                      font-family="var(--font-mono)">
                      {(pauseT * 100).toFixed(0)}%
                    </text>
                  {/if}

                  {#if history.length >= 2}
                    {@const lineCol = peak >= pauseT ? 'var(--danger)' : peak >= notifyT ? 'var(--warning)' : 'var(--success)'}
                    <path d={areaPath(history)} fill={lineCol} opacity="0.08"/>
                    <path d={linePath(history)} fill="none" stroke={lineCol}
                      stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                  {/if}

                  {#each history as pt, i}
                    {@const cx = mapX(i, history.length)}
                    {@const cy = mapY(pt.score)}
                    {@const dc = dotColor(pt.score)}
                    <circle
                      cx={cx} cy={cy} r="8"
                      fill="transparent"
                      style="cursor:{pt.snapshot ? 'pointer' : 'default'}"
                      on:mouseenter={(e) => handleDotEnter(e, pt)}
                      on:mouseleave={handleDotLeave}
                      on:click={() => handleDotClick(pt)}
                      on:keydown={(e) => e.key === 'Enter' && handleDotClick(pt)}
                      role="button" tabindex="0" aria-label="Detection {(pt.score*100).toFixed(0)}%"
                    />
                    <circle cx={cx} cy={cy} r="3.5" fill={dc} stroke="var(--surface)" stroke-width="1.5" pointer-events="none"/>
                  {/each}

                  {#if history.length === 1}
                    {@const cx = mapX(0, 1)}
                    {@const cy = mapY(history[0].score)}
                    {@const dc = dotColor(history[0].score)}
                    <circle cx={cx} cy={cy} r="8" fill="transparent"
                      style="cursor:{history[0].snapshot ? 'pointer' : 'default'}"
                      on:mouseenter={(e) => handleDotEnter(e, history[0])}
                      on:mouseleave={handleDotLeave}
                      on:click={() => handleDotClick(history[0])}
                      on:keydown={(e) => e.key === 'Enter' && handleDotClick(history[0])}
                      role="button" tabindex="0" aria-label="Detection {(history[0].score*100).toFixed(0)}%"
                    />
                    <circle cx={cx} cy={cy} r="3.5" fill={dc} stroke="var(--surface)" stroke-width="1.5" pointer-events="none"/>
                  {/if}
                </svg>
              {/if}
            </div>
            {#if history.length > 0}
              <div class="graph-footer">
                <span class="gf-count">
                  {history.length} detection{history.length !== 1 ? 's' : ''}
                  {#if isPrinting && currentFile}<span class="gf-muted"> this print</span>{/if}
                </span>
                <span class="gf-peak" style="color:{peak >= pauseT ? 'var(--danger)' : peak >= notifyT ? 'var(--warning)' : 'var(--text2)'}">
                  Peak {(peak * 100).toFixed(0)}%
                </span>
              </div>
            {/if}
          {/if}
        </div>

      </div>
    {:else}
      <div class="disabled-msg">Detection paused</div>
    {/if}
  {/if}
</div>

{#if tooltip}
  <div
    class="g-tooltip"
    style="left:{tooltip.cx}px; top:{tooltip.cy - 10}px; transform:translate(-50%,-100%)"
    role="tooltip"
    in:fade={{ duration: 100 }}
  >
    <div class="tt-head">
      <span class="tt-time">{fmtTime(tooltip.point.ts)}</span>
      <span class="tt-score" style="color:{dotColor(tooltip.point.score)}">
        {(tooltip.point.score * 100).toFixed(1)}%
      </span>
    </div>
    {#if tooltip.point.snapshot}
      <div class="tt-img-wrap">
        <img src="/snapshots/{tooltip.point.snapshot}" alt="Detection" class="tt-img" />
      </div>
      {#if tooltip.point.snapshot}
        <div class="tt-hint">Click to open</div>
      {/if}
    {/if}
  </div>
{/if}

{#if modalPoint}
  <Modal open={true} onClose={() => (modalPoint = null)}>
    <div
      class="det-modal"
      role="dialog" aria-modal="true" aria-label="Detection snapshot"
      in:fly={{ y: 8, duration: 200, easing: cubicOut }}
      out:fade={{ duration: 100 }}
    >
      <div class="det-head">
        <div class="det-meta">
          <span class="det-score-pill"
            style="color:{dotColor(modalPoint.score)};
                   background:color-mix(in srgb,{dotColor(modalPoint.score)} 12%,transparent);
                   border-color:color-mix(in srgb,{dotColor(modalPoint.score)} 30%,transparent)">
            {(modalPoint.score * 100).toFixed(1)}%
          </span>
          <span class="det-time">{fmtDateTime(modalPoint.ts)}</span>
          {#if modalPoint.print_filename}
            <span class="det-file" title={modalPoint.print_filename}>
              {modalPoint.print_filename.replace(/\.gcode$/i, '')}
            </span>
          {/if}
        </div>
        <button class="det-close" on:click={() => (modalPoint = null)} aria-label="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M2 2l10 10M12 2L2 12" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="det-img-wrap">
        <img src="/snapshots/{modalPoint.snapshot}" alt="Detection snapshot" class="det-img" />
        {#if modalPoint.boxes && modalPoint.boxes.length > 0}
          <svg class="det-overlay" viewBox="0 0 1 1" preserveAspectRatio="none">
            {#each modalPoint.boxes as b}
              <rect x={b.x1} y={b.y1} width={b.x2 - b.x1} height={b.y2 - b.y1}
                fill="none" stroke="var(--danger)" stroke-width="0.003"/>
              <rect x={b.x1} y={Math.max(0, b.y1 - 0.04)} width="0.09" height="0.032"
                fill="var(--danger)" rx="0.004"/>
              <text
                x={b.x1 + 0.007}
                y={Math.max(0.02, b.y1 - 0.015)}
                fill="#fff" font-size="0.023" font-weight="bold" font-family="monospace">
                {(b.confidence * 100).toFixed(0)}%
              </text>
            {/each}
          </svg>
        {/if}
      </div>

      {#if modalPoint.boxes && modalPoint.boxes.length > 0}
        <div class="det-footer">
          <span>
            {modalPoint.boxes.length} detection{modalPoint.boxes.length !== 1 ? 's' : ''}
          </span>
          <span>
            Max confidence
            <strong style="color:var(--danger)">
              {(Math.max(...modalPoint.boxes.map(b => b.confidence)) * 100).toFixed(0)}%
            </strong>
          </span>
        </div>
      {/if}
    </div>
  </Modal>
{/if}

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

  .card-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .chevron {
    color: var(--muted);
    transition: transform 0.2s;
    flex-shrink: 0;
  }
  .chevron.up { transform: rotate(-180deg); }

  /* Toggle */
  .toggle-wrap { display: flex; }
  .toggle { position: relative; display: block; width: 36px; height: 20px; cursor: pointer; }
  .toggle input { opacity: 0; width: 0; height: 0; position: absolute; }
  .knob {
    position: absolute;
    inset: 0;
    background: var(--surface2);
    border: 1px solid var(--border2);
    border-radius: 20px;
    transition: background 0.2s;
  }
  .knob::before {
    content: '';
    position: absolute;
    width: 14px;
    height: 14px;
    left: 2px;
    top: 2px;
    background: var(--muted);
    border-radius: 50%;
    transition: transform 0.2s, background 0.2s;
  }
  input:checked + .knob::before { transform: translateX(16px); background: var(--text); }

  /* Body */
  .body { padding: 12px 14px; display: flex; flex-direction: column; gap: 10px; }

  /* Score strip */
  .score-strip {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .score-badge {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    border: 2px solid var(--c, var(--muted));
    background: color-mix(in srgb, var(--c, var(--muted)) 10%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .score-val {
    font-size: 17px;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--c, var(--text));
    line-height: 1;
  }
  .score-unit { font-size: 10px; font-weight: 500; }
  .score-right { flex: 1; display: flex; flex-direction: column; gap: 5px; min-width: 0; }
  .score-label { font-size: 13px; font-weight: 600; }

  .thr-row { display: flex; gap: 8px; flex-wrap: wrap; }
  .thr-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    color: var(--muted);
    padding: 2px 7px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border);
    background: var(--surface2);
    white-space: nowrap;
  }
  .thr-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .thr-notify .thr-dot { background: var(--warning); }
  .thr-pause .thr-dot { background: var(--danger); }

  /* Graph */
  .graph-outer {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface2);
    overflow: hidden;
  }

  .clean-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 22px 14px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--success);
  }

  .graph-wrap { position: relative; }
  .graph-svg { display: block; }

  .graph-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 5px 10px;
    border-top: 1px solid var(--border);
    font-size: 10.5px;
    color: var(--muted);
  }
  .gf-count { display: flex; align-items: center; gap: 3px; }
  .gf-muted { color: var(--muted2); }
  .gf-peak { font-weight: 600; font-family: var(--font-mono); }

  .disabled-msg {
    padding: 20px 14px;
    font-size: 13px;
    color: var(--muted);
    text-align: center;
    font-style: italic;
  }

  /* Tooltip */
  .g-tooltip {
    position: fixed;
    z-index: 9999;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: var(--radius-sm);
    box-shadow: 0 6px 24px rgba(0,0,0,0.4);
    padding: 8px 10px;
    min-width: 120px;
    pointer-events: none;
  }
  .tt-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 6px;
  }
  .tt-time { font-size: 10px; color: var(--muted); white-space: nowrap; }
  .tt-score { font-size: 13px; font-weight: 700; font-family: var(--font-mono); }
  .tt-img-wrap { position: relative; border-radius: 4px; overflow: hidden; border: 1px solid var(--border); }
  .tt-img { display: block; width: 140px; height: 79px; object-fit: cover; }
  .tt-hint {
    margin-top: 5px;
    font-size: 9.5px;
    color: var(--muted2);
    text-align: center;
  }

  /* Modal */
  .det-modal {
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 10px;
    width: min(480px, calc(100vw - 32px));
    box-shadow: 0 24px 80px rgba(0,0,0,0.6);
    overflow: hidden;
  }
  .det-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
  }
  .det-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    min-width: 0;
    flex: 1;
  }
  .det-score-pill {
    font-size: 12px;
    font-weight: 700;
    font-family: var(--font-mono);
    padding: 3px 9px;
    border-radius: var(--radius-pill);
    border: 1px solid;
    flex-shrink: 0;
  }
  .det-time { font-size: 11px; color: var(--muted); white-space: nowrap; }
  .det-file {
    font-size: 10.5px;
    color: var(--muted2);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 180px;
  }
  .det-close {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--surface2);
    border: 1px solid var(--border);
    color: var(--text);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
  }
  .det-close:hover { background: var(--border); }

  .det-img-wrap {
    position: relative;
    background: #000;
    line-height: 0;
  }
  .det-img {
    display: block;
    width: 100%;
    height: auto;
    max-height: calc(100vh - 200px);
    object-fit: contain;
  }
  .det-overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .det-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    font-size: 11.5px;
    color: var(--muted);
    border-top: 1px solid var(--border);
    background: var(--surface2);
  }
</style>
