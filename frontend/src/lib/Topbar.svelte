<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { Globe, Settings, Printer } from 'lucide-svelte';

  export let connected: boolean = false;
  export let printerIp: string = '';

  const dispatch = createEventDispatcher<{ openSettings: void }>();
</script>

<header class="topbar">
  <div class="left">
    <div class="brand">
      <span class="brand-icon"><Printer size={20} strokeWidth={1.9} /></span>
      <div class="brand-text">
        <span class="brand-name">CC2</span>
        <span class="brand-sub">Monitor</span>
      </div>
    </div>

    <span class="divider" aria-hidden="true"></span>

    <div class="status-pill" class:online={connected}>
      <span class="dot"></span>
      <span>{connected ? 'Connected' : 'Offline'}</span>
    </div>
  </div>

  <div class="right">
    {#if printerIp}
      <div class="ip-tag mono" title="Printer IP address">
        <Globe size={11} strokeWidth={1.9} />
        <span>{printerIp}</span>
      </div>
    {/if}

    <button class="icon-btn" on:click={() => dispatch('openSettings')} title="Settings" aria-label="Open settings">
      <Settings size={15} strokeWidth={1.9} />
    </button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 20px;
    height: 48px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, var(--surface), rgba(23,24,28,0.92));
    backdrop-filter: blur(6px);
    flex-shrink: 0;
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .left, .right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .brand-icon {
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .brand-text {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .brand-name {
    font-size: 13px;
    font-weight: 700;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .brand-sub {
    font-size: 10px;
    font-weight: 500;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }

  .divider {
    width: 1px;
    height: 16px;
    background: var(--border2);
    opacity: 0.7;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px 3px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    font-size: 11px;
    font-weight: 500;
    color: var(--muted);
    background: var(--surface2);
  }
  .status-pill .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
  }
  .status-pill.online {
    color: var(--success);
    border-color: rgba(74,140,92,0.35);
    background: var(--success-dim);
  }
  .status-pill.online .dot {
    background: var(--success);
    box-shadow: 0 0 0 3px rgba(74,140,92,0.2);
  }

  .ip-tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    font-size: 11px;
    color: var(--muted);
    background: var(--surface2);
  }

  .icon-btn {
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface2);
    color: var(--muted);
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .icon-btn:hover {
    color: var(--text);
    border-color: var(--border2);
    background: var(--surface3);
  }
</style>
