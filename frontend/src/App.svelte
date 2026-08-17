<script lang="ts">
  import { onMount } from 'svelte';
  import { connect, disconnect, sendPing, wsConnected } from './ws';
  import { printer, requestOpenSettings, ui_settings } from './stores';
  import { checkSetup, getVersion } from './api';
  import Onboarding from './lib/Onboarding.svelte';
  import SettingsModal from './lib/SettingsModal.svelte';
  import Topbar from './lib/Topbar.svelte';
  import Camera from './lib/Camera.svelte';
  import PrintHeader from './lib/PrintHeader.svelte';
  import TempPanel from './lib/TempPanel.svelte';
  import AMSPanel from './lib/AMSPanel.svelte';
  import DetectionPanel from './lib/DetectionPanel.svelte';
  import Controls from './lib/Controls.svelte';
  import FileList from './lib/FileList.svelte';
  import Toast from './lib/Toast.svelte';
  import { fly, fade } from 'svelte/transition';

  let showSettings = false;
  let settingsInitialSection = 'general';
  let route: 'loading' | 'setup' | 'monitor' = 'loading';
  let pingInterval: number | undefined;

  let updateAvailable = false;
  let updateDismissed = false;
  let updateChecked = false;
  let latestVersion: string | null = null;

  const UPDATE_DISMISS_KEY = 'cc2_update_dismissed';

  function isDismissedToday(): boolean {
    const today = new Date().toISOString().slice(0, 10);
    return localStorage.getItem(UPDATE_DISMISS_KEY) === today;
  }

  function dismissUpdate() {
    localStorage.setItem(UPDATE_DISMISS_KEY, new Date().toISOString().slice(0, 10));
    updateDismissed = true;
  }

  async function checkUpdate() {
    if (updateChecked) return;
    updateChecked = true;
    try {
      const v = await getVersion();
      if (!v.up_to_date && !isDismissedToday()) {
        latestVersion = v.latest_version;
        updateAvailable = true;
      }
    } catch { /* network error, skip silently */ }
  }

  $: if (route === 'monitor') checkUpdate();

  $: if ($requestOpenSettings !== null) {
    settingsInitialSection = $requestOpenSettings;
    showSettings = true;
    requestOpenSettings.set(null);
  }

  function currentPath(): string {
    if (window.location.hash.startsWith('#/')) {
      return window.location.hash.slice(1);
    }
    return window.location.pathname;
  }

  function navigate(path: string, replace = false) {
    if (window.location.pathname === path) return;
    if (replace) history.replaceState({}, '', path);
    else history.pushState({}, '', path);
    resolveRoute();
  }

  async function resolveRoute() {
    const path = currentPath();
    try {
      const result = await checkSetup();
      const onboarded = result.configured && result.onboarding_complete;

      if (!onboarded) {
        if (path !== '/setup') {
          history.replaceState({}, '', '/setup');
        }
        route = 'setup';
        return;
      }

      if (path !== '/') {
        history.replaceState({}, '', '/');
      }
      route = 'monitor';
      if (!pingInterval) {
        connect();
        pingInterval = window.setInterval(sendPing, 25000);
      }
    } catch {
      route = 'setup';
      history.replaceState({}, '', '/setup');
    }
  }

  function onSetupComplete() {
    navigate('/', true);
    resolveRoute();
  }

  onMount(() => {
    resolveRoute();
    const onPop = () => resolveRoute();
    window.addEventListener('popstate', onPop);
    return () => {
      window.removeEventListener('popstate', onPop);
      if (pingInterval) clearInterval(pingInterval);
      disconnect();
    };
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') showSettings = false;
  }

  $: showJobInfo   = $ui_settings.find(s => s.id === 'job-info')?.checked   ?? true;
  $: showControl   = $ui_settings.find(s => s.id === 'control')?.checked    ?? true;
  $: showDetection = $ui_settings.find(s => s.id === 'detection')?.checked  ?? true;
  $: showFiles     = $ui_settings.find(s => s.id === 'files')?.checked      ?? true;
  $: showCamera    = $ui_settings.find(s => s.id === 'camera')?.checked     ?? true;
  $: showTemp      = $ui_settings.find(s => s.id === 'temperature')?.checked ?? true;
  $: showCanvas    = $ui_settings.find(s => s.id === 'canvas')?.checked     ?? true;

</script>

<svelte:window on:keydown={onKeydown} />

{#if route === 'loading'}
  <div class="splash">
    <div class="spinner"></div>
  </div>
{:else if route === 'setup'}
  <Onboarding on:complete={onSetupComplete} />
{:else}
  <div class="shell">
    <Topbar
      connected={$printer.connected}
      serverConnected={$wsConnected}
      printerIp={$printer.printer_ip}
      on:openSettings={() => showSettings = true}
    />

    <main class="grid">
      <div class="col-left">
        {#if showJobInfo}<PrintHeader />{/if}
        {#if showControl}<Controls />{/if}
        {#if showDetection}<DetectionPanel />{/if}
        {#if showFiles}<FileList />{/if}
      </div>
      <div class="col-right">
        {#if showCamera}<Camera />{/if}
        {#if showTemp}<TempPanel />{/if}
        {#if showCanvas}<AMSPanel />{/if}
      </div>
    </main>
  </div>

  {#if showSettings}
    <SettingsModal initialSection={settingsInitialSection} on:close={() => showSettings = false} />
  {/if}
{/if}

<Toast />

{#if updateAvailable && !updateDismissed}
  <div
    class="update-banner"
    role="status"
    in:fly={{ y: 12, duration: 200 }}
    out:fade={{ duration: 150 }}
  >
    <span class="update-icon" aria-hidden="true">
      <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
        <path d="M7 12V3M3 6.5l4-4 4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </span>
    <span class="update-text">{latestVersion ? `v${latestVersion} is available` : 'A new version is available'}</span>
    <a
      class="update-link"
      href="https://github.com/DimeusDev/cc2-openwebui"
      target="_blank"
      rel="noopener noreferrer"
    >View on GitHub →</a>
    <button class="update-dismiss" on:click={dismissUpdate} aria-label="Dismiss update notice">
      <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
        <path d="M2 2l8 8M10 2L2 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
    </button>
  </div>
{/if}

<style>
  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
  }

  :global(.splash .spinner) {
    width: 28px;
    height: 28px;
  }

  .shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(0, 65fr) minmax(0, 35fr);
    align-items: start;
    padding: 18px 20px 28px;
    gap: 14px;
    max-width: 1600px;
    width: 100%;
    margin: 0 auto;
  }

  .col-left, .col-right {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }

  @media (max-width: 960px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }

  .update-banner {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 8999;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px 10px 14px;
    background: var(--surface);
    border: 1px solid var(--border2);
    border-radius: var(--radius);
    box-shadow: 0 4px 24px rgba(0,0,0,0.45);
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    max-width: 340px;
  }

  .update-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent-dim);
    color: var(--accent);
    flex-shrink: 0;
  }

  .update-text { flex: 1; white-space: nowrap; }

  .update-link {
    font-size: 12px;
    color: var(--accent);
    text-decoration: none;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .update-link:hover { text-decoration: underline; }

  .update-dismiss {
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--surface2);
    border: 1px solid var(--border);
    color: var(--muted);
    cursor: pointer;
    flex-shrink: 0;
  }
  .update-dismiss:hover { background: var(--border); color: var(--text); }

  @media (max-width: 480px) {
    .update-banner { left: 16px; right: 16px; bottom: 16px; max-width: none; }
    .update-text { font-size: 12px; }
  }
</style>
