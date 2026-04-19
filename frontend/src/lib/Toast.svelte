<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { toasts } from '../stores';

  function icon(type: string) {
    if (type === 'error') return '✕';
    if (type === 'warn') return '!';
    return 'i';
  }
</script>

<div class="toast-stack" aria-live="polite" aria-atomic="false">
  {#each $toasts as t (t.id)}
    <div
      class="toast toast-{t.type}"
      role="alert"
      in:fly={{ y: 12, duration: 180 }}
      out:fade={{ duration: 150 }}
    >
      <span class="toast-icon">{icon(t.type)}</span>
      <span class="toast-msg">{t.message}</span>
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 9000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 16px;
    border-radius: var(--radius);
    border: 1px solid;
    font-size: 13px;
    font-weight: 500;
    max-width: 340px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.4);
    pointer-events: auto;
  }

  .toast-error {
    background: var(--danger-dim);
    border-color: rgba(232,69,90,0.4);
    color: var(--danger);
  }

  .toast-warn {
    background: var(--warning-dim);
    border-color: rgba(240,160,48,0.4);
    color: var(--warning);
  }

  .toast-info {
    background: var(--surface2);
    border-color: var(--border2);
    color: var(--text);
  }

  .toast-icon {
    font-size: 11px;
    font-weight: 700;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid currentColor;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-family: var(--font-mono);
  }

  .toast-msg {
    flex: 1;
    line-height: 1.4;
  }
</style>
