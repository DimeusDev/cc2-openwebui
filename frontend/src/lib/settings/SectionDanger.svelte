<script lang="ts">
  import { fade } from 'svelte/transition';
  import { resetAll } from '../../api';
  import { toErrorMessage } from '../errors';

  let resetConfirmOpen = false;
  let resetPhrase = '';
  let resetState: 'idle' | 'resetting' | 'error' = 'idle';
  let resetError = '';
  const RESET_KEYWORD = 'RESET';

  function openResetConfirm() {
    resetPhrase = '';
    resetError = '';
    resetState = 'idle';
    resetConfirmOpen = true;
  }

  async function confirmReset() {
    if (resetPhrase.trim().toUpperCase() !== RESET_KEYWORD) {
      resetError = `Type ${RESET_KEYWORD} to confirm.`;
      return;
    }
    resetState = 'resetting';
    resetError = '';
    try {
      await resetAll();
      window.location.reload();
    } catch (e) {
      resetState = 'error';
      resetError = toErrorMessage(e) || 'Reset failed';
    }
  }
</script>

<div class="danger-card">
  <div class="danger-head">
    <span class="danger-badge">
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
        <path d="M8 2l6 11H2L8 2z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>
        <path d="M8 6.5v3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        <circle cx="8" cy="11.3" r="0.8" fill="currentColor"/>
      </svg>
    </span>
    <div>
      <div class="danger-title">Reset all settings</div>
      <div class="danger-sub">Clears printer connection, detection config, notifications, and returns to the onboarding flow. Cannot be undone.</div>
    </div>
  </div>
  <ul class="danger-list">
    <li>Printer IP and pincode are wiped.</li>
    <li>Detection threshold and Obico URL revert to defaults.</li>
    <li>ntfy credentials are cleared.</li>
    <li>Active MQTT connection is closed.</li>
  </ul>
  {#if !resetConfirmOpen}
    <button class="btn danger" on:click={openResetConfirm}>
      <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
        <path d="M3 4h10M6.5 4V2.5h3V4M5 4l.5 9h5l.5-9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      Reset all settings
    </button>
  {:else}
    <div class="danger-confirm" in:fade={{ duration: 140 }}>
      <div class="confirm-label">
        Type <span class="mono keyword">{RESET_KEYWORD}</span> to confirm.
      </div>
      <input
        class="input mono confirm-input"
        type="text"
        bind:value={resetPhrase}
        placeholder={RESET_KEYWORD}
        disabled={resetState === 'resetting'}
        autocomplete="off"
      />
      {#if resetError}
        <div class="confirm-err">{resetError}</div>
      {/if}
      <div class="confirm-actions">
        <button class="btn" on:click={() => (resetConfirmOpen = false)} disabled={resetState === 'resetting'}>Cancel</button>
        <button
          class="btn danger"
          on:click={confirmReset}
          disabled={resetState === 'resetting' || resetPhrase.trim().toUpperCase() !== RESET_KEYWORD}
        >
          {#if resetState === 'resetting'}
            <span class="spinner sm"></span>
            Resetting…
          {:else}
            Yes, reset everything
          {/if}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .danger-card {
    border: 1px solid rgba(192,57,74,0.35);
    border-radius: 8px;
    background: linear-gradient(180deg, rgba(192,57,74,0.06), rgba(192,57,74,0.02));
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .danger-head { display: flex; align-items: flex-start; gap: 12px; }
  .danger-badge {
    display: inline-flex;
    width: 30px; height: 30px;
    align-items: center; justify-content: center;
    border-radius: 8px;
    background: var(--danger-dim);
    color: var(--danger);
    border: 1px solid rgba(192,57,74,0.4);
    flex-shrink: 0;
  }
  .danger-title { font-size: 13.5px; font-weight: 600; color: var(--text); }
  .danger-sub { font-size: 11.8px; color: var(--muted); line-height: 1.5; margin-top: 3px; max-width: 58ch; }
  .danger-list {
    margin: 0; padding: 0 0 0 18px;
    font-size: 11.8px; color: var(--muted); line-height: 1.6; list-style: disc;
  }
  .danger-list li::marker { color: rgba(192,57,74,0.55); }

  .btn.danger {
    border-color: rgba(192,57,74,0.4);
    background: var(--danger-dim);
    color: var(--danger);
    align-self: flex-start;
    display: inline-flex; align-items: center; gap: 7px;
  }
  .btn.danger:hover:not(:disabled) { background: rgba(192,57,74,0.18); border-color: rgba(192,57,74,0.6); }
  .btn.danger:disabled { opacity: 0.55; cursor: not-allowed; }

  .danger-confirm {
    display: flex; flex-direction: column; gap: 10px;
    padding: 12px 14px;
    background: var(--surface);
    border: 1px dashed rgba(192,57,74,0.45);
    border-radius: 7px;
  }
  .confirm-label { font-size: 12px; color: var(--muted); }
  .keyword { color: var(--danger); font-weight: 700; letter-spacing: 0.08em; }
  .confirm-input { width: 220px; max-width: 100%; letter-spacing: 0.14em; text-transform: uppercase; }
  .confirm-err { font-size: 11.5px; color: var(--danger); }
  .confirm-actions { display: flex; gap: 8px; margin-top: 2px; }

  .spinner.sm {
    width: 12px; height: 12px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    display: inline-block;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
