<script lang="ts">
  export let printer: { ip: string; printer_id: string; pincode: string };
</script>

<div class="group">
  <div class="row">
    <div class="row-label">
      <div class="row-title">IP Address</div>
      <div class="row-sub">Found on your printer's network settings screen.</div>
    </div>
    <input id="ip" class="input mono row-input" type="text" bind:value={printer.ip} placeholder="192.168.1.100" />
  </div>
  <div class="row">
    <div class="row-label">
      <div class="row-title">Pincode</div>
      <div class="row-sub">Leave empty if pin-protection is disabled.</div>
    </div>
    <input id="pin" class="input mono row-input short" type="text" bind:value={printer.pincode} placeholder="A1bC2" maxlength="6" />
  </div>
</div>

<style>
  .group {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr 220px;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
  .row:first-child { border-top: none; }
  .row-label { min-width: 0; }
  .row-title { font-size: 12.5px; font-weight: 500; color: var(--text); }
  .row-sub { font-size: 11.5px; color: var(--muted); margin-top: 2px; line-height: 1.45; }
  .row-input { width: 100%; }
  .row-input.short { max-width: 140px; justify-self: end; }

  @media (max-width: 700px) {
    .row { grid-template-columns: 1fr; gap: 8px; }
    .row-input.short { max-width: none; justify-self: stretch; }
  }
</style>
