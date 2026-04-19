<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import cc2Image from '../cc2.png';
  import { scanNetwork, verifyPrinter, saveConfig } from '../../api';
  import { toErrorMessage } from '../errors';

  const dispatch = createEventDispatcher<{ complete: void }>();

  type Substep = 'intro' | 'scan' | 'configure' | 'verify';
  let substep: Substep = 'intro';
  let error = '';

  let scanning = false;
  let scanError = '';
  let printers: Array<{ ip: string }> = [];
  let selectedIp = '';
  let manualIp = '';
  let hasPincode = false;
  let pincode = '';
  let verifying = false;
  let verifyProgress = '';
  let savingPrinter = false;

  async function doScan() {
    scanning = true;
    scanError = '';
    printers = [];
    substep = 'scan';
    try {
      const result = await scanNetwork();
      printers = result.printers;
      if (printers.length === 0) scanError = "No CC2 printers found. Make sure it's on the same network.";
    } catch (e) {
      scanError = toErrorMessage(e) || 'Network scan failed. Enter the IP manually.';
    }
    scanning = false;
  }

  function selectPrinter(ip: string) {
    selectedIp = ip;
    manualIp = '';
    substep = 'configure';
  }

  function useManualEntry() {
    selectedIp = '';
    substep = 'configure';
  }

  async function doVerify() {
    verifying = true;
    error = '';
    verifyProgress = '';
    substep = 'verify';
    const ip = (selectedIp || manualIp).trim();
    try {
      verifyProgress = 'Connecting to ' + ip + '…';
      await new Promise((r) => setTimeout(r, 300));
      verifyProgress = 'Identifying printer…';
      const result = await verifyPrinter(ip, hasPincode ? pincode.toUpperCase() : '');
      if (!result.success) throw new Error('Device responded but is not a CC2.');
      verifyProgress = 'Saving configuration…';
      savingPrinter = true;
      await saveConfig(ip, result.printer_id, hasPincode ? pincode.toUpperCase() : '');
      savingPrinter = false;
      dispatch('complete');
    } catch (e) {
      const msg = toErrorMessage(e).toLowerCase();
      if (msg.includes('pincode') || msg.includes('invalid')) {
        error = 'Invalid pincode. Check the 6-character code in your printer settings.';
      } else if (msg.includes('timeout') || msg.includes('no response')) {
        error = `No response from ${ip}. Check the IP and that the printer is on.`;
      } else if (msg.includes('refused') || msg.includes('connection')) {
        error = `Could not connect to ${ip}. Verify the address.`;
      } else {
        error = toErrorMessage(e) || 'Verification failed.';
      }
      substep = 'configure';
    }
    verifying = false;
  }

  function handlePincodeInput(e: Event) {
    const t = e.target as HTMLInputElement;
    pincode = t.value.toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 6);
  }
</script>

{#if substep === 'intro'}
  <section class="card hero">
    <div class="hero-copy">
      <span class="eyebrow">Step 1 · Printer</span>
      <h1>Connect your Elegoo CC2.</h1>
      <p>Enter the printer's IP address to get started. Find it in your printer's Settings → Network.</p>
      <div class="hero-actions">
        <button class="btn primary lg" on:click={useManualEntry}>Enter IP manually</button>
        <button class="btn lg ghost-soft" on:click={doScan}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.4"/>
            <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          </svg>
          Scan network
        </button>
      </div>
      <div class="hero-checks">
        <span><span class="dot-check"></span> Printer powered on</span>
        <span><span class="dot-check"></span> Same Wi-Fi or LAN</span>
      </div>
    </div>
    <div class="hero-visual">
      <div class="printer-frame">
        <div class="printer-halo"></div>
        <img src={cc2Image} alt="Elegoo CC2 printer" class="printer-img" />
      </div>
    </div>
  </section>

{:else if substep === 'scan'}
  <section class="card">
    <div class="card-head">
      <span class="eyebrow">Step 1 · Printer</span>
      <h2>Scanning your network</h2>
      <p>Looking for CC2 devices on your LAN…</p>
    </div>
    <div class="scan-stage">
      {#if scanning}
        <div class="radar-wrap" aria-hidden="true">
          <div class="radar-core"></div>
          <div class="radar-pulse"></div>
          <div class="radar-pulse d2"></div>
          <div class="radar-pulse d3"></div>
        </div>
        <div class="scan-line">Searching…</div>
      {:else if printers.length > 0}
        <div class="found">Found {printers.length} printer{printers.length === 1 ? '' : 's'}</div>
        <div class="list">
          {#each printers as p, i}
            <button class="list-row" on:click={() => selectPrinter(p.ip)} in:fly={{ y: 4, duration: 200, delay: i * 60, easing: cubicOut }}>
              <span class="list-icon">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                  <rect x="3" y="5" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.3"/>
                  <path d="M5 5V3.5A1.5 1.5 0 0 1 6.5 2h3A1.5 1.5 0 0 1 11 3.5V5" stroke="currentColor" stroke-width="1.3"/>
                  <circle cx="11" cy="8.5" r="0.7" fill="currentColor"/>
                </svg>
              </span>
              <span class="mono ip">{p.ip}</span>
              <span class="chip accent sm">CC2</span>
              <span class="list-arrow">→</span>
            </button>
          {/each}
        </div>
      {:else if scanError}
        <div class="alert err">{scanError}</div>
      {/if}
    </div>
    <div class="actions">
      <button class="btn ghost" on:click={() => (substep = 'intro')}>← Back</button>
      <div class="actions-right">
        {#if !scanning}
          <button class="btn" on:click={doScan}>Scan again</button>
        {/if}
        <button class="btn primary" on:click={useManualEntry}>Enter IP manually</button>
      </div>
    </div>
  </section>

{:else if substep === 'configure'}
  <section class="card">
    <div class="card-head">
      <span class="eyebrow">Step 1 · Printer</span>
      <h2>{selectedIp ? 'Confirm connection' : 'Enter your printer IP'}</h2>
      <p>{selectedIp ? 'Review the details below, then connect.' : 'Find the IP on your printer screen under Settings → Network.'}</p>
    </div>
    <div class="form">
      <div class="field">
        <label class="field-label" for="ip">Printer IP</label>
        {#if selectedIp}
          <input id="ip" class="input mono" type="text" bind:value={selectedIp} />
        {:else}
          <input id="ip" class="input mono" type="text" bind:value={manualIp} placeholder="192.168.1.100" />
        {/if}
      </div>
      <label class="inline-check">
        <input type="checkbox" bind:checked={hasPincode} />
        <span>My printer shows a 6-character pincode</span>
      </label>
      {#if hasPincode}
        <div class="field" in:fly={{ y: -6, duration: 180 }}>
          <label class="field-label" for="pin">Pincode</label>
          <input
            id="pin"
            class="input mono pin"
            class:valid={pincode.length === 6}
            type="text"
            value={pincode}
            on:input={handlePincodeInput}
            placeholder="ABCDEF"
            maxlength="6"
          />
          {#if pincode.length > 0 && pincode.length < 6}
            <span class="field-hint">{pincode.length}/6 characters</span>
          {/if}
        </div>
      {/if}
      {#if error}<div class="alert err">{error}</div>{/if}
      <div class="actions">
        <button class="btn ghost" on:click={() => { substep = selectedIp ? 'scan' : 'intro'; error = ''; }}>← Back</button>
        <button class="btn primary" disabled={!(selectedIp || manualIp).trim()} on:click={doVerify}>Connect</button>
      </div>
    </div>
  </section>

{:else}
  <section class="card center">
    <div class="verify-stack">
      {#if savingPrinter || verifying}<div class="big-spinner"></div>{/if}
      <h2>Verifying</h2>
      <p class="verify-line">{verifyProgress || 'Working…'}</p>
      {#if error}
        <div class="alert err">{error}</div>
        <div class="actions center-only">
          <button class="btn" on:click={() => { substep = 'configure'; error = ''; }}>Edit details</button>
          <button class="btn primary" on:click={doVerify}>Retry</button>
        </div>
      {/if}
    </div>
  </section>
{/if}

<style>
  .card {
    background: linear-gradient(180deg, var(--surface), var(--surface2));
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 30px 32px 26px;
    box-shadow: 0 1px 0 rgba(255,255,255,0.03) inset, 0 16px 40px -20px rgba(0,0,0,0.5), 0 2px 8px rgba(0,0,0,0.2);
  }
  .card.center { text-align: center; padding: 64px 32px; }
  .card-head { margin-bottom: 22px; }
  .eyebrow { display: inline-block; font-size: 10px; letter-spacing: 0.14em; text-transform: uppercase; color: var(--accent); font-weight: 600; margin-bottom: 8px; }
  .card-head h2 { font-size: 22px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.25; }
  .card-head p { color: var(--muted); font-size: 13px; line-height: 1.6; margin-top: 8px; max-width: 62ch; }

  /* Hero */
  .hero { display: grid; grid-template-columns: 1.3fr 1fr; gap: 32px; align-items: center; padding: 40px 40px 36px; }
  .hero-copy h1 { font-size: 28px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.2; margin-top: 8px; }
  .hero-copy p { color: var(--muted); font-size: 13.5px; line-height: 1.6; margin-top: 12px; max-width: 44ch; }
  .hero-actions { display: flex; gap: 10px; margin-top: 22px; flex-wrap: wrap; }
  .hero-checks { display: flex; gap: 18px; margin-top: 22px; font-size: 11.5px; color: var(--muted); }
  .hero-checks span { display: inline-flex; align-items: center; gap: 6px; }
  .dot-check { width: 5px; height: 5px; border-radius: 50%; background: var(--success); box-shadow: 0 0 0 3px var(--success-dim); }
  .btn.ghost-soft { background: var(--surface2); border-color: var(--border); color: var(--text); }
  .btn.ghost-soft:hover:not(:disabled) { background: var(--surface3); border-color: var(--border2); }
  .hero-visual { display: flex; justify-content: center; align-items: center; }
  .printer-frame { position: relative; width: 240px; height: 240px; display: flex; align-items: center; justify-content: center; border-radius: 16px; background: radial-gradient(circle at 50% 45%, var(--surface3), transparent 70%); }
  .printer-halo { position: absolute; width: 220px; height: 220px; border-radius: 50%; background: radial-gradient(circle, rgba(45,135,240,0.22), transparent 60%); filter: blur(20px); animation: halo 6s ease-in-out infinite; }
  @keyframes halo { 0%, 100% { opacity: 0.7; transform: scale(1); } 50% { opacity: 1; transform: scale(1.05); } }
  .printer-img { position: relative; width: 210px; height: auto; filter: drop-shadow(0 12px 20px rgba(0,0,0,0.4)); animation: float 5s ease-in-out infinite; }
  @keyframes float { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-4px); } }

  /* Scan */
  .scan-stage { display: flex; flex-direction: column; align-items: center; gap: 14px; min-height: 180px; justify-content: center; padding: 10px 0; }
  .radar-wrap { position: relative; width: 120px; height: 120px; display: flex; align-items: center; justify-content: center; }
  .radar-core { width: 14px; height: 14px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 14px var(--accent); }
  .radar-pulse { position: absolute; inset: 0; border-radius: 50%; border: 1px solid var(--accent); opacity: 0; animation: radar-pulse 2s infinite ease-out; }
  .radar-pulse.d2 { animation-delay: 0.66s; }
  .radar-pulse.d3 { animation-delay: 1.33s; }
  @keyframes radar-pulse { 0% { transform: scale(0.2); opacity: 0.9; } 100% { transform: scale(1); opacity: 0; } }
  .scan-line { color: var(--muted); font-size: 12px; }
  .found { font-size: 12px; color: var(--muted); letter-spacing: 0.02em; }
  .list { width: 100%; display: flex; flex-direction: column; gap: 8px; }
  .list-row { display: flex; align-items: center; gap: 12px; padding: 12px 14px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-md); transition: border-color 0.15s, background 0.15s, transform 0.15s; text-align: left; }
  .list-row:hover { border-color: var(--accent); background: var(--accent-dim); transform: translateY(-1px); }
  .list-icon { width: 28px; height: 28px; border-radius: var(--radius); background: var(--surface2); color: var(--muted); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
  .list-row:hover .list-icon { color: var(--accent); background: var(--surface3); }
  .list-row .ip { font-size: 13px; flex: 1; }
  .list-arrow { color: var(--muted); }

  /* Form */
  .form { display: flex; flex-direction: column; gap: 0; }
  .field { display: flex; flex-direction: column; gap: 5px; margin-bottom: 12px; }
  .field-label { font-size: 11px; font-weight: 500; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; }
  .field-hint { font-size: 11px; color: var(--muted); }
  .inline-check { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text); cursor: pointer; padding: 8px 0 4px; }
  .inline-check input { accent-color: var(--accent); }
  .pin { letter-spacing: 0.25em; text-transform: uppercase; font-size: 15px; }
  .pin.valid { border-color: var(--success); }

  /* Verify */
  .verify-stack { display: flex; flex-direction: column; align-items: center; gap: 14px; }
  .big-spinner { width: 36px; height: 36px; border: 2.5px solid var(--border2); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite; }
  .verify-line { color: var(--muted); font-size: 13px; }

  .alert { padding: 11px 13px; border-radius: var(--radius-md); font-size: 12.5px; line-height: 1.5; margin-top: 6px; }
  .alert.err { color: var(--danger); border: 1px solid rgba(192,57,74,0.3); background: var(--danger-dim); }

  .actions { display: flex; justify-content: space-between; align-items: center; margin-top: 22px; gap: 10px; }
  .actions-right { display: flex; gap: 8px; }
  .actions.center-only { justify-content: center; margin-top: 12px; }

  .chip.sm { padding: 2px 6px; font-size: 10px; }
  .chip.accent { color: var(--accent); border-color: rgba(45,135,240,0.35); background: var(--accent-dim); }

  @keyframes spin { to { transform: rotate(360deg); } }

  @media (max-width: 760px) {
    .hero { grid-template-columns: 1fr; gap: 16px; padding: 28px 24px; }
    .hero-visual { order: -1; }
    .printer-frame { width: 180px; height: 180px; }
    .printer-img { width: 160px; }
  }
</style>
