<script lang="ts">
  import { fade } from 'svelte/transition';

  export let open = false;
  export let onClose: () => void;
  export let zIndex = 100;
</script>

<svelte:window on:keydown={(e) => open && e.key === 'Escape' && onClose()} />

{#if open}
  <div
    class="modal-backdrop"
    style="z-index: {zIndex}"
    role="none"
    on:click|self={onClose}
    transition:fade={{ duration: 150 }}
  >
    <slot />
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(10, 10, 14, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(2px);
  }
</style>
