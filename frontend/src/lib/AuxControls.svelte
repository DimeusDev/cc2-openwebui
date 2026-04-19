<script lang="ts">
  import { Lightbulb } from 'lucide-svelte';
  import { printer } from '../stores';
  import { setLed, setFan, setSpeedMode } from '../api';
  import { toErrorMessage } from './errors';

  $: s = $printer.state;
  $: ledOn = s?.led?.status === 1;
  $: fanSpeed    = s?.fans?.fan?.speed ?? 0;
  $: auxFanSpeed = s?.fans?.aux_fan?.speed ?? 0;
  $: boxFanSpeed = s?.fans?.box_fan?.speed ?? 0;
  $: speedMode   = s?.gcode_move?.speed_mode ?? 1;

  let error = '';

  const SPEED_MODES = [
    { val: 0, label: 'Silent' },
    { val: 1, label: 'Balanced' },
    { val: 2, label: 'Sport' },
    { val: 3, label: 'Ludicrous' },
  ];

  function fanToPercent(speed: number): number { return Math.round((speed / 255) * 100); }
  function percentToFan(pct: number): number { return Math.round((pct / 100) * 255); }
  function isFanOn(rawSpeed: number): boolean { return rawSpeed > 0; }
  async function stepFan(name: string, rawSpeed: number, delta: number) {
    const pct = Math.min(100, Math.max(0, fanToPercent(rawSpeed) + delta));
    await handleFan(name, percentToFan(pct));
  }
  async function toggleFan(name: string, rawSpeed: number) {
    await handleFan(name, rawSpeed > 0 ? 0 : Math.round(255 * 0.5));
  }
  async function handleLed(on: boolean) {
    error = '';
    try { await setLed(on); } catch (e) { error = toErrorMessage(e); }
  }
  async function handleFan(name: string, rawSpeed: number) {
    error = '';
    try { await setFan(name, rawSpeed); } catch (e) { error = toErrorMessage(e); }
  }
  async function handleSpeedMode(e: Event) {
    const mode = parseInt((e.target as HTMLSelectElement).value);
    error = '';
    try { await setSpeedMode(mode); } catch (e) { error = toErrorMessage(e); }
  }
</script>

<div class="aux-col">
  <div class="row-inline">
    <span class="row-label">Print Speed</span>
    <div class="select-wrap">
      <select value={speedMode} on:change={handleSpeedMode}>
        {#each SPEED_MODES as m}
          <option value={m.val}>{m.label}</option>
        {/each}
      </select>
      <svg class="sel-caret" width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
        <path d="M2.5 4l2.5 2.5L7.5 4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>
  </div>

  <div class="hr"></div>

  {#each [
    { name: 'fan',     label: 'Model',  raw: fanSpeed },
    { name: 'aux_fan', label: 'Assist', raw: auxFanSpeed },
    { name: 'box_fan', label: 'Case',   raw: boxFanSpeed },
  ] as fan (fan.name)}
    <div class="fan-row">
      <div class="fan-left">
        <svg width="13" height="13" viewBox="0 0 15 15" fill="none" aria-hidden="true">
          <g>
            <path d="M7.08789,6.20209C6.9252,5.77666,6.71402,5.22445,6.71402,4.54545C6.71402,3.97394,6.86363,3.31261,7.00801,2.67439C7.32296,1.2822,7.61302,0,6.27105,0C4.31387,0,2.72727,1.62806,2.72727,3.63636C2.72727,5.64467,4.31387,7.27273,6.27105,7.27273C7.49732,7.27273,7.36084,6.91585,7.08789,6.20209Z" fill-rule="evenodd" fill="currentColor"/>
            <g transform="matrix(0,1,-1,0,17.72727,-12.27273)"><path d="M19.36062,8.92936C19.19793,8.50393,18.98675,7.95172,18.98675,7.27272C18.98675,6.70121,19.13636,6.03988,19.28074,5.40166C19.59569,4.00947,19.88575,2.72727,18.54378,2.72727C16.5866,2.72727,15,4.35533,15,6.36363C15,8.37194,16.5866,10,18.54378,10C19.77005,10,19.63357,9.64312,19.36062,8.92936Z" fill-rule="evenodd" fill="currentColor"/></g>
            <g transform="matrix(-1,0,0,-1,24.54546,30)"><path d="M16.63335,21.20209C16.47066,20.77666,16.25948,20.22445,16.25948,19.54545C16.25948,18.97394,16.40909,18.31261,16.55347,17.67439C16.86842,16.2822,17.15848,15,15.81651,15C13.85933,15,12.27273,16.62806,12.27273,18.63636C12.27273,20.64467,13.85933,22.27273,15.81651,22.27273C17.04278,22.27273,16.90630,21.91585,16.63335,21.20209Z" fill-rule="evenodd" fill="currentColor"/></g>
            <g transform="matrix(0,-1,1,0,-12.27270,12.27270)"><path d="M4.36062,18.47479C4.19793,18.04936,3.98675,17.49715,3.98675,16.81815C3.98675,16.24664,4.13636,15.58531,4.28074,14.94709C4.59569,13.55490,4.88575,12.27270,3.54378,12.27270C1.5866,12.27270,0,13.90076,0,15.90906C0,17.91737,1.5866,19.54543,3.54378,19.54543C4.77005,19.54543,4.63357,19.18855,4.36062,18.47479Z" fill-rule="evenodd" fill="currentColor"/></g>
          </g>
        </svg>
        <span class="fan-name">{fan.label}</span>
      </div>
      <div class="fan-right">
        <label class="switch">
          <input type="checkbox" checked={isFanOn(fan.raw)} on:change={() => toggleFan(fan.name, fan.raw)}/>
          <span class="slider"></span>
        </label>
        <div class="stepper" class:off={!isFanOn(fan.raw)}>
          <button on:click={() => stepFan(fan.name, fan.raw, -5)} disabled={!isFanOn(fan.raw) || fanToPercent(fan.raw) <= 0}>−</button>
          <span class="step-pct mono">{fanToPercent(fan.raw)}%</span>
          <button on:click={() => stepFan(fan.name, fan.raw, 5)} disabled={!isFanOn(fan.raw) || fanToPercent(fan.raw) >= 100}>+</button>
        </div>
      </div>
    </div>
  {/each}

  <div class="hr"></div>

  <div class="fan-row">
    <div class="fan-left">
      <Lightbulb size={13} strokeWidth={1.9} style={ledOn ? 'color: var(--warning);' : ''} aria-hidden="true" />
      <span class="fan-name">Chamber Light</span>
    </div>
    <div class="fan-right">
      <label class="switch">
        <input type="checkbox" checked={ledOn} on:change={(e) => handleLed(e.currentTarget.checked)}/>
        <span class="slider"></span>
      </label>
    </div>
  </div>

  {#if error}
    <div class="err">{error}</div>
  {/if}
</div>

<style>
  .aux-col {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }

  .row-inline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .row-label { font-size: 12px; color: var(--muted); }

  .select-wrap { position: relative; }
  .select-wrap select {
    appearance: none;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 12px;
    padding: 6px 26px 6px 10px;
    cursor: pointer;
    min-width: 120px;
  }
  .select-wrap select:focus { outline: none; border-color: var(--accent); }
  .sel-caret {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    pointer-events: none;
  }

  .hr { height: 1px; background: var(--border); }

  .fan-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 2px 0;
  }
  .fan-left { display: flex; align-items: center; gap: 8px; color: var(--muted); flex-shrink: 0; }
  .fan-name { font-size: 12px; color: var(--text); }
  .fan-right { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }

  .stepper {
    display: flex;
    align-items: center;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .stepper.off { opacity: 0.4; }
  .stepper button {
    width: 24px;
    height: 24px;
    color: var(--muted);
    font-size: 13px;
    transition: background 0.1s, color 0.1s;
  }
  .stepper button:hover:not(:disabled) { background: var(--surface3); color: var(--text); }
  .stepper button:disabled { opacity: 0.5; cursor: not-allowed; }
  .step-pct {
    font-size: 11px;
    min-width: 34px;
    text-align: center;
    color: var(--text);
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .err {
    font-size: 12px;
    color: var(--danger);
    padding: 6px 10px;
    background: var(--danger-dim);
    border-radius: var(--radius-sm);
  }
</style>
