<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Modal from './Modal.svelte';
  import { getSettings, updateSettings, type AppSettings } from '../api';
  import { toErrorMessage } from './errors';
  import SectionGeneral from './settings/SectionGeneral.svelte';
  import SectionDetection from './settings/SectionDetection.svelte';
  import SectionNotifications from './settings/SectionNotifications.svelte';
  import SectionLogs from './settings/SectionLogs.svelte';
  import SectionDanger from './settings/SectionDanger.svelte';

  const dispatch = createEventDispatcher<{ close: void }>();

  type Section = 'general' | 'detection' | 'notifications' | 'logs' | 'danger';
  let activeSection: Section = 'general';

  let settings: AppSettings = {
    printer: { ip: '', printer_id: '', pincode: '' },
    detection: { enabled: true, notify_threshold: 0.5, pause_threshold: 0.7, interval_secs: 15, confirmation_frames: 2, obico_url: 'http://localhost:3333' },
    notifications: { destinations: [] },
    server: { host: '0.0.0.0', port: 8484 },
    logging: { level: 'info' },
  };

  let saveState: 'idle' | 'saving' | 'saved' | 'error' = 'idle';
  let errorMsg = '';

  onMount(async () => {
    try {
      settings = await getSettings();
    } catch { /* fallback to defaults */ }
  });

  async function saveSettings() {
    saveState = 'saving';
    errorMsg = '';
    try {
      await updateSettings(settings);
      saveState = 'saved';
      setTimeout(() => { if (saveState === 'saved') saveState = 'idle'; }, 2000);
    } catch (e) {
      saveState = 'error';
      errorMsg = toErrorMessage(e) || 'Save failed';
    }
  }

  function close() { dispatch('close'); }

  type NavItem = { id: Section; label: string; desc: string; icon: string };
  const navGroups: Array<{ title: string; items: NavItem[] }> = [
    {
      title: 'Setup',
      items: [{ id: 'general', label: 'Printer', desc: 'Connection', icon: 'printer' }],
    },
    {
      title: 'Features',
      items: [
        { id: 'detection', label: 'AI Detection', desc: 'Failure alerts', icon: 'eye' },
        { id: 'notifications', label: 'Notifications', desc: 'ntfy, Discord', icon: 'bell' },
      ],
    },
    {
      title: 'Diagnostics',
      items: [{ id: 'logs', label: 'Activity Logs', desc: 'Events & errors', icon: 'log' }],
    },
    {
      title: 'Advanced',
      items: [{ id: 'danger', label: 'Danger Zone', desc: 'Reset everything', icon: 'warn' }],
    },
  ];

  const sectionMeta: Record<Section, { title: string; desc: string }> = {
    general: { title: 'Printer Connection', desc: 'The IP your CC2 exposes on the LAN.' },
    detection: { title: 'AI Failure Detection', desc: 'AI failure detection with Obico ML container.' },
    notifications: { title: 'Notifications', desc: 'Push events via ntfy or Discord webhook. Add and configure notification destinations.' },
    logs: { title: 'Activity Logs', desc: 'Connection events, print jobs, detections, etc' },
    danger: { title: 'Danger Zone', desc: 'Irreversible actions. Make sure you know what you are doing.' },
  };
</script>

<Modal open={true} onClose={close}>
  <div
    class="modal-sheet settings-sheet"
    role="dialog"
    aria-modal="true"
    in:fly={{ y: 10, duration: 220, easing: cubicOut }}
    out:fade={{ duration: 120 }}
  >
    <div class="modal-head">
      <div class="head-left">
        <span class="head-icon">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="2.2" stroke="currentColor" stroke-width="1.3" fill="none"/>
            <path d="M8 1.5v1.6M8 12.9v1.6M14.5 8h-1.6M3.1 8H1.5M12.6 3.4l-1.1 1.1M4.5 11.5l-1.1 1.1M12.6 12.6l-1.1-1.1M4.5 4.5L3.4 3.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          </svg>
        </span>
        <span class="modal-title">Settings</span>
      </div>
      <button class="modal-close" on:click={close} aria-label="Close settings">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M3 3l8 8M11 3L3 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    </div>

    <div class="settings-body">
      <nav class="nav" aria-label="Settings sections">
        {#each navGroups as g}
          <div class="nav-group-label">{g.title}</div>
          {#each g.items as s}
            <button
              class="nav-item"
              class:active={activeSection === s.id}
              on:click={() => (activeSection = s.id)}
            >
              <span class="nav-rail" aria-hidden="true"></span>
              <span class="nav-icon">
                {#if s.icon === 'printer'}
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                    <rect x="3.5" y="2" width="9" height="4" rx="1" stroke="currentColor" stroke-width="1.3"/>
                    <rect x="2" y="6" width="12" height="6" rx="1.5" stroke="currentColor" stroke-width="1.3"/>
                    <rect x="4.5" y="10" width="7" height="4" rx="0.8" stroke="currentColor" stroke-width="1.3"/>
                    <circle cx="12" cy="8.5" r="0.6" fill="currentColor"/>
                  </svg>
                {:else if s.icon === 'eye'}
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                    <path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z" stroke="currentColor" stroke-width="1.3"/>
                    <circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.3"/>
                  </svg>
                {:else if s.icon === 'bell'}
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                    <path d="M8 1.5a4.5 4.5 0 00-4.5 4.5v3L2 11h12l-1.5-2V6A4.5 4.5 0 008 1.5zM6.5 13a1.5 1.5 0 003 0" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
                  </svg>
                {:else if s.icon === 'warn'}
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                    <path d="M8 2l6 11H2L8 2z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
                    <path d="M8 6.5v3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
                    <circle cx="8" cy="11.3" r="0.7" fill="currentColor"/>
                  </svg>
                {:else}
                  <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                    <rect x="2" y="2" width="12" height="12" rx="1.5" stroke="currentColor" stroke-width="1.3"/>
                    <path d="M4.5 5.5h7M4.5 8h7M4.5 10.5h4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
                  </svg>
                {/if}
              </span>
              <span class="nav-text">
                <span class="nav-label">{s.label}</span>
                <span class="nav-desc">{s.desc}</span>
              </span>
            </button>
          {/each}
        {/each}
      </nav>

      <div class="content">
        <header class="section-head">
          <h3>{sectionMeta[activeSection].title}</h3>
          <p>{sectionMeta[activeSection].desc}</p>
        </header>

        {#key activeSection}
          <div class="section-wrap" in:fade={{ duration: 180, easing: cubicOut }}>
            {#if activeSection === 'general'}
              <SectionGeneral bind:printer={settings.printer} />
            {:else if activeSection === 'detection'}
              <SectionDetection bind:detection={settings.detection} />
            {:else if activeSection === 'notifications'}
              <SectionNotifications />
            {:else if activeSection === 'logs'}
              <SectionLogs />
            {:else if activeSection === 'danger'}
              <SectionDanger />
            {/if}
          </div>
        {/key}
      </div>
    </div>

    {#if activeSection !== 'logs' && activeSection !== 'danger'}
      <div class="modal-foot">
        <div class="foot-left">
          {#if saveState === 'error'}
            <span class="save-msg err">
              <svg width="12" height="12" viewBox="0 0 14 14" fill="none"><circle cx="7" cy="7" r="6" stroke="currentColor" stroke-width="1.3"/><path d="M7 3.5v4M7 9.5v.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
              {errorMsg}
            </span>
          {:else if saveState === 'saved'}
            <span class="save-msg ok" in:fly={{ y: 4, duration: 200 }}>
              <svg width="12" height="12" viewBox="0 0 14 14" fill="none"><path d="M3 7.5l2.5 2.5L11 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
              Changes saved
            </span>
          {/if}
        </div>
        <div class="foot-right">
          <button class="btn" on:click={close}>Close</button>
          <button class="btn primary" on:click={saveSettings} disabled={saveState === 'saving'}>
            {#if saveState === 'saving'}
              <span class="spinner sm"></span>
              Saving…
            {:else}
              Save changes
            {/if}
          </button>
        </div>
      </div>
    {/if}
  </div>
</Modal>

<style>
  .settings-sheet {
    width: min(860px, calc(100vw - 40px));
    height: min(600px, calc(100vh - 48px));
    border-radius: 12px;
    border: 1px solid var(--border2);
    box-shadow: 0 24px 80px -20px rgba(0,0,0,0.65), 0 4px 14px rgba(0,0,0,0.3);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--surface);
  }

  .settings-body {
    flex: 1;
    display: grid;
    grid-template-columns: 210px 1fr;
    min-height: 0;
    overflow: hidden;
  }

  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 11px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 10px;
  }
  .modal-title { font-size: 13px; font-weight: 600; color: var(--text); }
  .modal-close {
    width: 28px; height: 28px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: 50%;
    background: var(--surface2); border: 1px solid var(--border); color: var(--muted);
    cursor: pointer; flex-shrink: 0;
  }
  .modal-close:hover { background: var(--border); color: var(--text); }
  .head-left { display: flex; align-items: center; gap: 10px; color: var(--muted); }
  .head-icon {
    display: inline-flex; width: 26px; height: 26px;
    align-items: center; justify-content: center;
    border-radius: 7px;
    background: var(--accent-dim); color: var(--accent);
  }

  /* Sidebar */
  .nav {
    display: flex; flex-direction: column;
    padding: 14px 10px 14px 14px;
    gap: 2px;
    border-right: 1px solid var(--border);
    background: var(--bg-deep);
    overflow-y: auto;
  }
  .nav-group-label {
    font-size: 9.5px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase;
    color: var(--muted2); padding: 14px 10px 6px;
  }
  .nav-group-label:first-child { padding-top: 4px; }
  .nav-item {
    position: relative;
    display: flex; align-items: center; gap: 11px;
    padding: 9px 10px 9px 12px;
    font-size: 12px; color: var(--muted);
    border-radius: 7px; text-align: left;
    transition: color 0.15s, background 0.15s;
  }
  .nav-item:hover { color: var(--text); background: var(--surface2); }
  .nav-item.active { color: var(--text); background: var(--accent-dim); }
  .nav-rail {
    position: absolute; left: 0; top: 8px; bottom: 8px;
    width: 2px; border-radius: 2px;
    background: transparent; transition: background 0.15s;
  }
  .nav-item.active .nav-rail { background: var(--accent); }
  .nav-icon {
    display: inline-flex; width: 26px; height: 26px;
    align-items: center; justify-content: center;
    border-radius: 6px; background: var(--surface2);
    border: 1px solid var(--border); color: var(--muted);
    flex-shrink: 0; transition: color 0.15s, background 0.15s, border-color 0.15s;
  }
  .nav-item:hover .nav-icon { color: var(--text); border-color: var(--border2); }
  .nav-item.active .nav-icon { color: var(--accent); border-color: rgba(45,135,240,0.35); background: var(--accent-bg); }
  .nav-text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .nav-label { font-size: 12.5px; font-weight: 500; }
  .nav-desc { font-size: 10.5px; color: var(--muted2); letter-spacing: 0.01em; }
  .nav-item.active .nav-desc { color: var(--muted); }

  /* Content */
  .content {
    padding: 22px 26px 24px;
    overflow-y: auto;
    display: flex; flex-direction: column;
    min-height: 0;
  }
  .section-head { margin-bottom: 18px; }
  .section-head h3 { font-size: 16px; font-weight: 600; letter-spacing: -0.01em; color: var(--text); }
  .section-head p { font-size: 12.5px; color: var(--muted); margin-top: 4px; line-height: 1.55; max-width: 56ch; }
  .section-wrap { display: flex; flex-direction: column; gap: 14px; }

  /* Footer */
  .modal-foot {
    padding: 12px 18px;
    border-top: 1px solid var(--border);
    display: flex; justify-content: space-between; align-items: center;
    gap: 10px; flex-shrink: 0;
    background: var(--surface);
  }
  .foot-left { display: flex; align-items: center; }
  .foot-right { display: flex; gap: 8px; }
  .save-msg { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; }
  .save-msg.err { color: var(--danger); }
  .save-msg.ok { color: var(--success); }

  .spinner.sm {
    width: 12px; height: 12px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    display: inline-block;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  @media (max-width: 700px) {
    .settings-sheet { height: calc(100vh - 48px); }
    .settings-body { grid-template-columns: 1fr; }
    .nav {
      flex-direction: row; overflow-x: auto;
      padding: 8px 10px; border-right: none; border-bottom: 1px solid var(--border); gap: 4px;
    }
    .nav-group-label { display: none; }
    .nav-item { flex-direction: column; gap: 4px; min-width: 88px; padding: 8px 10px; }
    .nav-desc { display: none; }
    .nav-rail { display: none; }
  }
</style>
