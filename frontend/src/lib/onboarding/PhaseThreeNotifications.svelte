<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { fly } from 'svelte/transition';
  import {
    completeOnboarding, createDestination, testDestination, deleteDestination, defaultToggles,
  } from '../../api';
  import { toErrorMessage } from '../errors';
  import appleStoreIcon from '../icons/apple-store.svg';
  import androidStoreIcon from '../icons/android-store.svg';

  export let obicoUrl: string;
  export let notifyThreshold: number;
  export let pauseThreshold: number;

  const dispatch = createEventDispatcher<{ back: void; complete: void }>();

  type NotifMethod = 'ntfy' | 'discord' | '';
  let notifMethod: NotifMethod = '';
  let ntfyServer = 'https://ntfy.sh';
  let ntfyTopic = '';
  let discordWebhookUrl = '';
  let notifTestState: 'idle' | 'sending' | 'sent' | 'error' = 'idle';
  let notifTestError = '';
  let finalizing = false;
  let error = '';
  type StoreKind = 'apple' | 'android';
  let storeModal: StoreKind | null = null;
  const ntfyStore = {
    apple: {
      label: 'Apple App Store',
      link: 'https://apps.apple.com/app/ntfy/id1625396347',
    },
    android: {
      label: 'Google Play',
      link: 'https://play.google.com/store/apps/details?id=io.heckel.ntfy',
    },
  } as const;

  async function sendTestNotif() {
    notifTestState = 'sending';
    notifTestError = '';
    let tempId: string | null = null;
    try {
      const dest = notifMethod === 'discord'
        ? { id: '', kind: 'discord' as const, enabled: true, label: 'Onboarding test', discord_webhook_url: discordWebhookUrl, toggles: defaultToggles() }
        : { id: '', kind: 'ntfy' as const, enabled: true, label: 'Onboarding test', ntfy_server: ntfyServer, ntfy_topic: ntfyTopic, toggles: defaultToggles() };
      tempId = await createDestination(dest);
      await testDestination(tempId);
      notifTestState = 'sent';
      setTimeout(() => { if (notifTestState === 'sent') notifTestState = 'idle'; }, 3000);
    } catch (e) {
      notifTestState = 'error';
      notifTestError = toErrorMessage(e) || 'Failed';
    } finally {
      if (tempId) await deleteDestination(tempId).catch(() => {});
    }
  }

  async function finalize(skipNotifs = false) {
    finalizing = true;
    error = '';
    try {
      type DestInput = {
        id: string; kind: 'ntfy' | 'discord'; enabled: boolean; label: string;
        ntfy_server?: string; ntfy_topic?: string; discord_webhook_url?: string;
        toggles: ReturnType<typeof defaultToggles>;
      };
      const dests: DestInput[] = [];
      if (!skipNotifs) {
        if (notifMethod === 'ntfy' && ntfyTopic) {
          dests.push({ id: '', kind: 'ntfy', enabled: true, label: 'NTFY', ntfy_server: ntfyServer, ntfy_topic: ntfyTopic, toggles: defaultToggles() });
        } else if (notifMethod === 'discord' && discordWebhookUrl) {
          dests.push({ id: '', kind: 'discord', enabled: true, label: 'Discord', discord_webhook_url: discordWebhookUrl, toggles: defaultToggles() });
        }
      }
      await completeOnboarding({
        detection: { obico_url: obicoUrl, notify_threshold: notifyThreshold, pause_threshold: pauseThreshold },
        notifications: dests.length > 0 ? { destinations: dests } : undefined,
      });
      dispatch('complete');
    } catch (e) {
      error = toErrorMessage(e) || 'Failed to save onboarding.';
    }
    finalizing = false;
  }
</script>

<section class="card">
  <div class="card-head">
    <span class="eyebrow">Step 3 · Notifications</span>
    <h2>Set up notifications</h2>
    <p>Get alerted when prints finish, fail, or get paused. Optional - configure more channels anytime in Settings.</p>
  </div>

  <div class="notif-options">
    <button
      class="notif-option"
      class:selected={notifMethod === 'discord'}
      on:click={() => { notifMethod = notifMethod === 'discord' ? '' : 'discord'; notifTestState = 'idle'; }}
    >
      <div class="notif-logo discord-logo">
        <svg width="22" height="17" viewBox="0 0 71 55" fill="currentColor" aria-hidden="true">
          <path d="M60.1 4.9A58.5 58.5 0 0 0 45.5.4a40.8 40.8 0 0 0-1.8 3.7 54.1 54.1 0 0 0-16.2 0A40 40 0 0 0 25.7.4 58.4 58.4 0 0 0 11 4.9C1.6 19 -1 32.8.3 46.3a59 59 0 0 0 18 9.1 43 43 0 0 0 3.7-6l-.1-.1a38.8 38.8 0 0 1-5.8-2.8l1-.7c11.1 5.1 23.1 5.1 34 0l1 .7a39 39 0 0 1-5.8 2.8l-.1.1a42.7 42.7 0 0 0 3.7 6 58.8 58.8 0 0 0 18-9.1C70.1 30.6 66.1 17 60 5ZM23.7 38c-3.5 0-6.4-3.2-6.4-7.2s2.8-7.2 6.4-7.2c3.6 0 6.5 3.2 6.4 7.2 0 4-2.8 7.2-6.4 7.2Zm23.6 0c-3.5 0-6.4-3.2-6.4-7.2s2.8-7.2 6.4-7.2c3.6 0 6.5 3.2 6.4 7.2 0 4-2.8 7.2-6.4 7.2Z"/>
        </svg>
      </div>
      <div class="notif-option-body">
        <div class="notif-option-title">Discord Webhook</div>
        <div class="notif-option-sub">Alerts to a Discord channel</div>
      </div>
      {#if notifMethod === 'discord'}<div class="notif-check">✓</div>{/if}
    </button>

    <button
      class="notif-option"
      class:selected={notifMethod === 'ntfy'}
      on:click={() => { notifMethod = notifMethod === 'ntfy' ? '' : 'ntfy'; notifTestState = 'idle'; }}
    >
      <div class="notif-logo ntfy-logo">ntfy</div>
      <div class="notif-option-body">
        <div class="notif-option-title-row">
          <div class="notif-option-title">NTFY Push</div>
          <div class="store-buttons">
            <button class="store-btn icon-btn" on:click|stopPropagation={() => (storeModal = 'apple')} title="Apple App Store">
              <img src={appleStoreIcon} alt="Apple" />
            </button>
            <button class="store-btn icon-btn" on:click|stopPropagation={() => (storeModal = 'android')} title="Google Play">
              <img src={androidStoreIcon} alt="Android" />
            </button>
          </div>
        </div>
        <div class="notif-option-sub">Free, open-source, no account</div>
      </div>
      {#if notifMethod === 'ntfy'}<div class="notif-check">✓</div>{/if}
    </button>
  </div>

  {#if notifMethod === 'discord'}
    <div class="notif-form" in:fly={{ y: -6, duration: 180 }}>
      <div class="field">
        <label class="field-label" for="dwurl">Webhook URL</label>
        <input id="dwurl" class="input mono" type="text" bind:value={discordWebhookUrl} placeholder="https://discord.com/api/webhooks/…" />
        <span class="field-hint">Server Settings → Integrations → Webhooks → New Webhook.</span>
      </div>
      <div class="test-row">
        <button class="btn sm" on:click={sendTestNotif} disabled={!discordWebhookUrl || notifTestState === 'sending'}>
          {#if notifTestState === 'sending'}<span class="spinner"></span> Sending…{:else}Send test{/if}
        </button>
        {#if notifTestState === 'sent'}<span class="test-ok">Sent ✓</span>{/if}
        {#if notifTestState === 'error'}<span class="test-err">{notifTestError}</span>{/if}
      </div>
    </div>
  {:else if notifMethod === 'ntfy'}
    <div class="notif-form" in:fly={{ y: -6, duration: 180 }}>
      <div class="field">
        <label class="field-label" for="nserver">NTFY Server</label>
        <input id="nserver" class="input mono" type="text" bind:value={ntfyServer} />
        <span class="field-hint">Use https://ntfy.sh or your self-hosted instance.</span>
      </div>
      <div class="field">
        <label class="field-label" for="ntopic">Topic</label>
        <input id="ntopic" class="input mono" type="text" bind:value={ntfyTopic} placeholder="cc2-monitor-1234" />
        <span class="field-hint">Subscribe to this topic in the ntfy app on your phone.</span>
      </div>
      <div class="test-row">
        <button class="btn sm" on:click={sendTestNotif} disabled={!ntfyTopic || notifTestState === 'sending'}>
          {#if notifTestState === 'sending'}<span class="spinner"></span> Sending…{:else}Send test{/if}
        </button>
        {#if notifTestState === 'sent'}<span class="test-ok">Sent - check your app ✓</span>{/if}
        {#if notifTestState === 'error'}<span class="test-err">{notifTestError}</span>{/if}
      </div>
    </div>
  {/if}

  {#if error}<div class="alert err">{error}</div>{/if}

  <div class="actions">
    <button class="btn ghost" on:click={() => dispatch('back')}>← Back</button>
    <div class="actions-right">
      <button class="btn ghost" on:click={() => finalize(true)} disabled={finalizing}>
        {finalizing ? 'Saving…' : 'Skip'}
      </button>
      <button class="btn primary" on:click={() => finalize(false)} disabled={finalizing}>
        {finalizing ? 'Saving…' : 'Finish Setup'}
      </button>
    </div>
  </div>
</section>

{#if storeModal}
  <div class="modal-backdrop">
    <div class="modal">
      <div class="modal-head">
        <h3>{ntfyStore[storeModal].label}</h3>
        <button class="modal-close" on:click={() => (storeModal = null)}>✕</button>
      </div>
      <a class="modal-link mono" href={ntfyStore[storeModal].link} target="_blank" rel="noreferrer">
        {ntfyStore[storeModal].link}
      </a>
      <img
        class="qr"
        src={`https://api.qrserver.com/v1/create-qr-code/?size=320x320&data=${encodeURIComponent(ntfyStore[storeModal].link)}`}
        alt={`${ntfyStore[storeModal].label} QR code`}
      />
    </div>
  </div>
{/if}

<style>
  .card { background: linear-gradient(180deg, var(--surface), var(--surface2)); border: 1px solid var(--border); border-radius: 12px; padding: 30px 32px 26px; box-shadow: 0 1px 0 rgba(255,255,255,0.03) inset, 0 16px 40px -20px rgba(0,0,0,0.5), 0 2px 8px rgba(0,0,0,0.2); }
  .card-head { margin-bottom: 22px; }
  .eyebrow { display: inline-block; font-size: 10px; letter-spacing: 0.14em; text-transform: uppercase; color: var(--accent); font-weight: 600; margin-bottom: 8px; }
  .card-head h2 { font-size: 22px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.25; }
  .card-head p { color: var(--muted); font-size: 13px; line-height: 1.6; margin-top: 8px; max-width: 62ch; }

  .notif-options { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 16px; }
  .notif-option { display: flex; align-items: center; gap: 12px; padding: 14px 16px; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--surface); text-align: left; cursor: pointer; transition: border-color 0.15s, background 0.15s, transform 0.15s; width: 100%; }
  .notif-option:hover { border-color: var(--border2); background: var(--surface2); transform: translateY(-1px); }
  .notif-option.selected { border-color: var(--accent); background: var(--accent-dim); }
  .notif-logo { width: 42px; height: 42px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
  .discord-logo { background: #5865F2; color: #fff; }
  .ntfy-logo { background: var(--accent); color: #fff; font-size: 11px; font-weight: 700; font-family: var(--font-mono); letter-spacing: -0.02em; }
  .notif-option-body { flex: 1; min-width: 0; }
  .notif-option-title-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .notif-option-title { font-size: 13.5px; font-weight: 600; }
  .notif-option-sub { font-size: 11.5px; color: var(--muted); margin-top: 2px; }
  .notif-check { color: var(--accent); font-size: 14px; font-weight: 700; }
  .store-buttons { display: flex; gap: 6px; margin-top: 8px; }
  .store-btn {
    font-size: 11px;
    line-height: 1;
    padding: 6px 10px;
    border: 1px solid var(--border2);
    border-radius: var(--radius-pill);
    color: var(--text);
    background: var(--surface2);
  }
  .store-btn:hover { border-color: var(--accent); }
  .icon-btn { width: 34px; height: 34px; padding: 0; display: inline-flex; align-items: center; justify-content: center; }
  .icon-btn img { width: 18px; height: 18px; object-fit: contain; display: block; }

  .notif-form { border: 1px solid var(--border); border-radius: var(--radius-md); padding: 14px 16px; display: flex; flex-direction: column; gap: 0; margin-bottom: 6px; }
  .field { display: flex; flex-direction: column; gap: 5px; margin-bottom: 12px; }
  .field-label { font-size: 11px; font-weight: 500; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; }
  .field-hint { font-size: 11px; color: var(--muted); }

  .test-row { display: flex; align-items: center; gap: 12px; padding: 4px 0; }
  .test-ok { font-size: 12px; color: var(--success); }
  .test-err { font-size: 12px; color: var(--danger); }

  .alert { padding: 11px 13px; border-radius: var(--radius-md); font-size: 12.5px; line-height: 1.5; margin-top: 6px; }
  .alert.err { color: var(--danger); border: 1px solid rgba(192,57,74,0.3); background: var(--danger-dim); }

  .actions { display: flex; justify-content: space-between; align-items: center; margin-top: 22px; gap: 10px; }
  .actions-right { display: flex; gap: 8px; }

  .spinner { width: 14px; height: 14px; border: 2px solid var(--border2); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite; display: inline-block; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.56);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 70;
    padding: 20px;
  }
  .modal {
    width: min(420px, 100%);
    border: 1px solid var(--border);
    border-radius: 12px;
    background: linear-gradient(180deg, var(--surface), var(--surface2));
    padding: 16px;
  }
  .modal-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; }
  .modal-head h3 { font-size: 16px; font-weight: 600; }
  .modal-close {
    width: 28px;
    height: 28px;
    border-radius: var(--radius);
    color: var(--muted);
    background: var(--bg-deep);
    border: 1px solid var(--border);
  }
  .modal-link { display: block; margin-bottom: 12px; color: var(--accent); word-break: break-all; }
  .qr {
    width: min(320px, 100%);
    aspect-ratio: 1 / 1;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: #fff;
    display: block;
    margin: 0 auto;
  }

  @media (max-width: 760px) {
    .notif-options { grid-template-columns: 1fr; }
  }
</style>
