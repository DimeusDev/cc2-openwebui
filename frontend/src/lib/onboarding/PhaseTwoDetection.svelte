<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { testObicoUrl } from '../../api';
  import { toErrorMessage } from '../errors';

  const dispatch = createEventDispatcher<{
    next: { obicoUrl: string; notifyThreshold: number; pauseThreshold: number };
    back: void;
  }>();

  let notifyThreshold = 0.6;
  let pauseThreshold = 0.7;
  let obicoUrl = 'http://localhost:3333/p/';
  const obicoGuideUrl = 'https://github.com/DimeusDev/cc2-opencloud/docs/obicolm.md';

  let testUrlState: 'idle' | 'testing' | 'ok' | 'fail' = 'idle';
  let testUrlError = '';


  async function doTestObicoUrl() {
    testUrlState = 'testing';
    testUrlError = '';
    try {
      await testObicoUrl(obicoUrl);
      testUrlState = 'ok';
    } catch (e) {
      testUrlState = 'fail';
      testUrlError = toErrorMessage(e);
    }
  }
</script>

<section class="card">
  <div class="card-head">
    <span class="eyebrow">Step 2 · Detection</span>
    <h2>AI failure detection</h2>
    <p>Set your Obico ML URL and test it before continuing.</p>
  </div>

  <div class="req-box">
    <div class="req-title">Obico ML connection</div>
    <p class="req-sub">This checks if the URL is reachable from the app.</p>

    <div class="obico-overlay">
      <div class="field-wrap">
        <label class="field-label" for="obico-url">Obico ML URL</label>
        <input id="obico-url" class="input mono" type="text" bind:value={obicoUrl} />
      </div>

      <button class="btn primary big-test" on:click={doTestObicoUrl} disabled={!obicoUrl || testUrlState === 'testing'}>
        {testUrlState === 'testing' ? 'Testing…' : 'Test Obico ML connection'}
      </button>

      {#if testUrlState === 'ok'}
        <div class="alert ok">Obico ML is reachable.</div>
      {/if}

      {#if testUrlState === 'fail'}
        <div class="alert err">
          <div>{testUrlError || 'Connection failed.'}</div>
          <a class="guide-link" href={obicoGuideUrl} target="_blank" rel="noreferrer">Open Obico guide</a>
        </div>
      {/if}
    </div>
  </div>

  <div class="options">
    <div class="opt-row col">
      <div class="opt-main">
        <div class="opt-title">Notify threshold <span class="opt-val mono">{notifyThreshold.toFixed(2)}</span></div>
        <div class="opt-sub">Score above which a notification is sent. Lower = more sensitive.</div>
      </div>
      <input type="range" min="0.2" max="0.9" step="0.05" bind:value={notifyThreshold} class="range" />
    </div>
    <div class="opt-row col">
      <div class="opt-main">
        <div class="opt-title">Pause threshold <span class="opt-val mono">{pauseThreshold.toFixed(2)}</span></div>
        <div class="opt-sub">Score above which print is auto-paused. Should be higher than notify.</div>
      </div>
      <input type="range" min="0.2" max="0.9" step="0.05" bind:value={pauseThreshold} class="range" />
    </div>
  </div>

  <div class="actions">
    <button class="btn ghost" on:click={() => dispatch('back')}>← Back</button>
    <button class="btn primary" on:click={() => dispatch('next', { obicoUrl, notifyThreshold, pauseThreshold })}>Continue</button>
  </div>
</section>

<style>
  .card { background: linear-gradient(180deg, var(--surface), var(--surface2)); border: 1px solid var(--border); border-radius: 12px; padding: 30px 32px 26px; box-shadow: 0 1px 0 rgba(255,255,255,0.03) inset, 0 16px 40px -20px rgba(0,0,0,0.5), 0 2px 8px rgba(0,0,0,0.2); }
  .card-head { margin-bottom: 22px; }
  .eyebrow { display: inline-block; font-size: 10px; letter-spacing: 0.14em; text-transform: uppercase; color: var(--accent); font-weight: 600; margin-bottom: 8px; }
  .card-head h2 { font-size: 22px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.25; }
  .card-head p { color: var(--muted); font-size: 13px; line-height: 1.6; margin-top: 8px; max-width: 62ch; }

  .req-box { border: 1px solid var(--border); background: var(--surface); border-radius: var(--radius-md); padding: 14px 16px; margin-bottom: 18px; }
  .req-title { font-size: 12px; font-weight: 600; }
  .req-sub { color: var(--muted); font-size: 12px; margin: 4px 0 10px; }
  .obico-overlay { border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--bg-deep); padding: 14px; }
  .field-wrap { display: flex; flex-direction: column; gap: 4px; }
  .field-label { font-size: 11.5px; color: var(--muted); }
  .big-test { width: 100%; margin-top: 12px; height: 44px; font-size: 14px; font-weight: 600; }

  .options { display: flex; flex-direction: column; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--surface); overflow: hidden; margin-bottom: 6px; }
  .opt-row { display: flex; align-items: center; justify-content: space-between; padding: 13px 16px; gap: 12px; border-top: 1px solid var(--border); }
  .opt-row:first-child { border-top: none; }
  .opt-row.col { flex-direction: column; align-items: stretch; gap: 10px; }
  .opt-main { min-width: 0; }
  .opt-title { font-size: 13px; font-weight: 500; display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .opt-val { font-size: 12px; color: var(--accent); }
  .opt-sub { font-size: 11.5px; color: var(--muted); margin-top: 2px; line-height: 1.4; }
  .range { width: 100%; accent-color: var(--accent); height: 4px; }

  .alert { margin-top: 12px; padding: 11px 13px; border-radius: var(--radius-md); font-size: 12.5px; line-height: 1.5; }
  .alert.err { color: var(--danger); border: 1px solid rgba(192,57,74,0.3); background: var(--danger-dim); }
  .alert.ok { color: var(--success); border: 1px solid rgba(74,140,92,0.35); background: var(--success-dim); }
  .guide-link { display: inline-block; margin-top: 6px; font-size: 12px; color: var(--accent); text-decoration: underline; }

  .actions { display: flex; justify-content: space-between; align-items: center; margin-top: 22px; gap: 10px; }
</style>
