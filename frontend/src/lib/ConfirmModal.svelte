<script lang="ts">
  import Modal from './Modal.svelte';

  export let open: boolean;
  export let onClose: () => void;
  export let onConfirm: () => void;
  export let label: string;
  export let title: string;
  export let description: string;
  export let confirmLabel = 'Confirm';
  export let variant: 'warn' | 'danger' = 'danger';
  export let disabled = false;
</script>

<Modal {open} {onClose}>
  <div class="modal" role="dialog" aria-modal="true" aria-label={label}>
    <div class="modal-icon {variant}">
      <slot name="icon" />
    </div>
    <div class="modal-body">
      <h2 class="modal-title">{title}</h2>
      <p class="modal-desc">{@html description}</p>
    </div>
    <div class="modal-actions">
      <button class="modal-btn cancel" on:click={onClose}>Cancel</button>
      <button class="modal-btn confirm {variant}" on:click={onConfirm} {disabled}>
        {confirmLabel}
      </button>
    </div>
  </div>
</Modal>

<style>
  .modal {
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: 12px;
    padding: 28px 28px 24px;
    width: 340px;
    max-width: calc(100vw - 40px);
    display: flex;
    flex-direction: column;
    gap: 16px;
    box-shadow: 0 24px 64px rgba(0,0,0,0.5);
  }

  .modal-icon {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .modal-icon.warn { background: var(--warning-dim); color: var(--warning); }
  .modal-icon.danger { background: var(--danger-dim); color: var(--danger); }

  .modal-body { display: flex; flex-direction: column; gap: 6px; }

  .modal-title {
    font-size: 17px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
  }

  .modal-desc {
    font-size: 13px;
    color: var(--muted);
    line-height: 1.5;
    margin: 0;
  }

  .modal-actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
    margin-top: 4px;
  }

  .modal-btn {
    padding: 9px 20px;
    font-size: 13px;
    font-weight: 600;
    border-radius: 7px;
    border: 1px solid;
    cursor: pointer;
    transition: filter 0.15s;
  }

  .modal-btn:hover:not(:disabled) { filter: brightness(1.15); }
  .modal-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .modal-btn.cancel {
    background: var(--surface2);
    border-color: var(--border2);
    color: var(--text);
  }

  .modal-btn.confirm.warn {
    background: var(--warning-dim);
    border-color: rgba(240,160,48,0.5);
    color: var(--warning);
  }

  .modal-btn.confirm.danger {
    background: var(--danger-dim);
    border-color: rgba(232,69,90,0.5);
    color: var(--danger);
  }
</style>
