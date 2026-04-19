<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listDestinations, createDestination, updateDestination, deleteDestination, testDestination,
    defaultToggles, type NotificationDestination, type DestinationKind,
  } from '../../api';
  import { toErrorMessage } from '../errors';

  let destinations: NotificationDestination[] = [];
  let destsLoading = false;
  let expandedDest: string | null = null;
  let testStates: Record<string, 'idle' | 'sending' | 'sent' | 'error'> = {};
  let testErrors: Record<string, string> = {};
  let deleteConfirm: string | null = null;

  let addingDest = false;
  let newKind: DestinationKind = 'ntfy';
  let newLabel = '';
  let newNtfyServer = 'https://ntfy.sh';
  let newNtfyTopic = '';
  let newDiscordUrl = '';
  let addDestState: 'idle' | 'saving' | 'error' = 'idle';
  let addDestError = '';

  onMount(() => { loadDestinations(); });

  async function loadDestinations() {
    destsLoading = true;
    try {
      destinations = await listDestinations();
    } catch {
      destinations = [];
    } finally {
      destsLoading = false;
    }
  }

  async function saveDest(dest: NotificationDestination) {
    try {
      await updateDestination(dest.id, dest);
    } catch (e) {
      testErrors[dest.id] = toErrorMessage(e);
      testErrors = testErrors;
    }
  }

  async function sendTest(id: string) {
    testStates[id] = 'sending';
    testErrors[id] = '';
    try {
      await testDestination(id);
      testStates[id] = 'sent';
      setTimeout(() => { if (testStates[id] === 'sent') testStates[id] = 'idle'; testStates = testStates; }, 3000);
    } catch (e) {
      testStates[id] = 'error';
      testErrors[id] = toErrorMessage(e);
    }
    testStates = testStates;
  }

  async function doDeleteDest(id: string) {
    try {
      await deleteDestination(id);
      destinations = destinations.filter((d) => d.id !== id);
      if (expandedDest === id) expandedDest = null;
      deleteConfirm = null;
    } catch (e) {
      testErrors[id] = toErrorMessage(e);
      testErrors = testErrors;
    }
  }

  async function doAddDest() {
    if (!newLabel.trim()) { addDestError = 'Label is required.'; return; }
    addDestState = 'saving';
    addDestError = '';
    try {
      const dest: Omit<NotificationDestination, 'id'> = {
        kind: newKind,
        enabled: true,
        label: newLabel.trim(),
        toggles: defaultToggles(),
        ...(newKind === 'ntfy'
          ? { ntfy_server: newNtfyServer, ntfy_topic: newNtfyTopic }
          : { discord_webhook_url: newDiscordUrl }),
      };
      await createDestination(dest);
      await loadDestinations();
      addingDest = false;
      newLabel = '';
      newNtfyTopic = '';
      newDiscordUrl = '';
      addDestState = 'idle';
    } catch (e) {
      addDestState = 'error';
      addDestError = toErrorMessage(e);
    }
  }
</script>

<div class="group">
  {#if destsLoading}
    <div class="row"><span class="muted-text">Loading destinations…</span></div>
  {:else if destinations.length === 0}
    <div class="row"><span class="muted-text">No destinations configured. Add one below.</span></div>
  {:else}
    {#each destinations as dest (dest.id)}
      <div class="dest-item">
        <div class="dest-head">
          <span class="kind-badge kind-{dest.kind}">{dest.kind}</span>
          <span class="dest-label-txt">{dest.label}</span>
          <div class="dest-acts">
            <span class="switch sm">
              <input type="checkbox" bind:checked={dest.enabled} on:change={() => saveDest(dest)} />
              <span class="slider"></span>
            </span>
            <button class="btn xs" on:click={() => { expandedDest = expandedDest === dest.id ? null : dest.id; }}>
              {expandedDest === dest.id ? 'Close' : 'Edit'}
            </button>
            {#if deleteConfirm === dest.id}
              <button class="btn xs danger" on:click={() => doDeleteDest(dest.id)}>Confirm</button>
              <button class="btn xs" on:click={() => (deleteConfirm = null)}>Cancel</button>
            {:else}
              <button class="btn xs" on:click={() => (deleteConfirm = dest.id)}>Delete</button>
            {/if}
          </div>
        </div>

        {#if expandedDest === dest.id}
          <div class="dest-body">
            {#if dest.kind === 'ntfy'}
              <div class="field-row">
                <label class="field-lbl" for="ns-{dest.id}">Server</label>
                <input id="ns-{dest.id}" class="input mono" type="text" bind:value={dest.ntfy_server} />
              </div>
              <div class="field-row">
                <label class="field-lbl" for="nt-{dest.id}">Topic</label>
                <input id="nt-{dest.id}" class="input mono" type="text" bind:value={dest.ntfy_topic} />
              </div>
            {:else if dest.kind === 'discord'}
              <div class="field-row">
                <label class="field-lbl" for="dw-{dest.id}">Webhook URL</label>
                <input id="dw-{dest.id}" class="input mono" type="text" bind:value={dest.discord_webhook_url} />
              </div>
            {/if}

            <div class="toggles-grid">
              <label class="tgl"><input type="checkbox" bind:checked={dest.toggles.print_started} /> Print started</label>
              <label class="tgl"><input type="checkbox" bind:checked={dest.toggles.print_finished} /> Print finished</label>
              <label class="tgl"><input type="checkbox" bind:checked={dest.toggles.print_paused} /> Print paused</label>
              <label class="tgl"><input type="checkbox" bind:checked={dest.toggles.failure_notify} /> Failure risk</label>
              <label class="tgl"><input type="checkbox" bind:checked={dest.toggles.failure_pause} /> Failure confirmed</label>
              <label class="tgl"><input type="checkbox" bind:checked={dest.toggles.auto_paused} /> Auto-paused</label>
            </div>

            <div class="dest-footer">
              <button class="btn sm primary" on:click={() => saveDest(dest)}>Save</button>
              <button class="btn sm" on:click={() => sendTest(dest.id)} disabled={testStates[dest.id] === 'sending'}>
                {testStates[dest.id] === 'sending' ? 'Sending…' : testStates[dest.id] === 'sent' ? 'Sent!' : 'Test'}
              </button>
              {#if testErrors[dest.id]}
                <span class="test-err">{testErrors[dest.id]}</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

{#if !addingDest}
  <div class="group">
    <div class="row">
      <div class="row-label">
        <div class="row-title">Add destination</div>
        <div class="row-sub">ntfy topic or Discord webhook.</div>
      </div>
      <button class="btn sm" on:click={() => { addingDest = true; addDestError = ''; addDestState = 'idle'; }}>+ Add</button>
    </div>
  </div>
{:else}
  <div class="group">
    <div class="add-form">
      <div class="field-row">
        <span class="field-lbl">Type</span>
        <div class="kind-pills">
          <button class="kind-pill" class:active={newKind === 'ntfy'} on:click={() => (newKind = 'ntfy')}>ntfy</button>
          <button class="kind-pill" class:active={newKind === 'discord'} on:click={() => (newKind = 'discord')}>Discord</button>
        </div>
      </div>
      <div class="field-row">
        <label class="field-lbl" for="new-lbl">Label</label>
        <input id="new-lbl" class="input" type="text" bind:value={newLabel} placeholder="My phone" />
      </div>
      {#if newKind === 'ntfy'}
        <div class="field-row">
          <label class="field-lbl" for="new-ns">Server</label>
          <input id="new-ns" class="input mono" type="text" bind:value={newNtfyServer} />
        </div>
        <div class="field-row">
          <label class="field-lbl" for="new-nt">Topic</label>
          <input id="new-nt" class="input mono" type="text" bind:value={newNtfyTopic} placeholder="cc2-my-topic" />
        </div>
      {:else if newKind === 'discord'}
        <div class="field-row">
          <label class="field-lbl" for="new-dw">Webhook URL</label>
          <input id="new-dw" class="input mono" type="text" bind:value={newDiscordUrl} placeholder="https://discord.com/api/webhooks/…" />
        </div>
      {/if}
      {#if addDestError}
        <div class="row-warn">{addDestError}</div>
      {/if}
      <div class="dest-footer">
        <button class="btn sm primary" on:click={doAddDest} disabled={addDestState === 'saving'}>
          {addDestState === 'saving' ? 'Saving…' : 'Add destination'}
        </button>
        <button class="btn sm" on:click={() => { addingDest = false; addDestError = ''; newLabel = ''; }}>Cancel</button>
      </div>
    </div>
  </div>
{/if}

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
  .row-sub { font-size: 11.5px; color: var(--muted); margin-top: 2px; }
  .row-warn { margin-top: 4px; font-size: 11px; color: var(--danger); }

  .muted-text { font-size: 12.5px; color: var(--muted); padding: 4px 0; }

  .dest-item { border-top: 1px solid var(--border); padding: 10px 16px; }
  .dest-item:first-child { border-top: none; }
  .dest-head { display: flex; align-items: center; gap: 8px; }
  .kind-badge {
    font-size: 10px; font-weight: 600; letter-spacing: 0.04em;
    padding: 2px 6px; border-radius: var(--radius-pill); text-transform: uppercase;
  }
  .kind-ntfy { background: #f0fdf4; color: #16a34a; border: 1px solid #bbf7d0; }
  .kind-discord { background: #ede9fe; color: #7c3aed; border: 1px solid #ddd6fe; }
  .kind-webhook { background: var(--surface2); color: var(--muted); border: 1px solid var(--border); }
  .dest-label-txt { flex: 1; font-size: 13px; font-weight: 500; color: var(--text); }
  .dest-acts { display: flex; align-items: center; gap: 6px; margin-left: auto; }

  .dest-body { margin-top: 10px; display: flex; flex-direction: column; gap: 8px; }
  .field-row { display: flex; align-items: center; gap: 10px; }
  .field-lbl { font-size: 11.5px; color: var(--muted); min-width: 72px; flex-shrink: 0; }
  .field-row .input { flex: 1; font-size: 12px; }

  .toggles-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 16px; margin-top: 4px; }
  .tgl { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text); cursor: pointer; }

  .dest-footer { display: flex; align-items: center; gap: 8px; margin-top: 6px; }
  .test-err { font-size: 11px; color: var(--danger); max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .add-form { padding: 12px 16px; display: flex; flex-direction: column; gap: 8px; }
  .kind-pills { display: flex; gap: 6px; }
  .kind-pill {
    font-size: 11.5px; padding: 3px 10px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border);
    background: var(--surface2); color: var(--text);
    cursor: pointer;
  }
  .kind-pill.active { background: var(--accent-dim); border-color: var(--accent); color: var(--accent); }

  .btn.xs { font-size: 10.5px; padding: 3px 8px; border-radius: var(--radius-sm); }
  .btn.danger { color: var(--danger); border-color: rgba(192,57,74,0.4); background: var(--danger-dim); }
  .switch.sm { transform: scale(0.8); transform-origin: right center; }

  @media (max-width: 700px) {
    .row { grid-template-columns: 1fr; gap: 8px; }
  }
</style>
