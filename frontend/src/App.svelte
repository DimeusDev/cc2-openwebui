<script lang="ts">
  import { onMount } from 'svelte';
  import { connect, disconnect, sendPing } from './ws';
  import { printer } from './stores';
  import { checkSetup } from './api';
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

  let showSettings = false;
  let route: 'loading' | 'setup' | 'monitor' = 'loading';
  let pingInterval: number | undefined;

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
      printerIp={$printer.printer_ip}
      on:openSettings={() => showSettings = true}
    />

    <main class="grid">
      <div class="col-left">
        <PrintHeader />
        <Controls />
        <DetectionPanel />
        <FileList />
      </div>
      <div class="col-right">
        <Camera />
        <TempPanel />
        <AMSPanel />
      </div>
    </main>
  </div>

  {#if showSettings}
    <SettingsModal on:close={() => showSettings = false} />
  {/if}
{/if}

<Toast />

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
</style>
