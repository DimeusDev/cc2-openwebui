<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { Printer } from 'lucide-svelte';
  import PhaseOnePrinter from './onboarding/PhaseOnePrinter.svelte';
  import PhaseTwoDetection from './onboarding/PhaseTwoDetection.svelte';
  import PhaseThreeNotifications from './onboarding/PhaseThreeNotifications.svelte';

  const dispatch = createEventDispatcher<{ complete: void }>();

  let phase: 1 | 2 | 3 | 7 = 1;
  let detectSettings = { obicoUrl: 'http://localhost:3333/p/', notifyThreshold: 0.6, pauseThreshold: 0.7 };

  function onPhase1Complete() {
    phase = 2;
  }

  function onPhase2Next(e: CustomEvent<{ obicoUrl: string; notifyThreshold: number; pauseThreshold: number }>) {
    detectSettings = e.detail;
    phase = 3;
  }

  function onPhase2Back() {
    phase = 1;
  }

  function onPhase3Back() {
    phase = 2;
  }

  function onPhase3Complete() {
    phase = 7;
    setTimeout(() => dispatch('complete'), 800);
  }

  const steps = [
    { n: 1, label: 'Printer' },
    { n: 2, label: 'Detection' },
    { n: 3, label: 'Notifications' },
  ] as const;

  function progressIndex(p: number): 1 | 2 | 3 {
    if (p === 1) return 1;
    if (p === 2) return 2;
    return 3;
  }

  $: progressPct = ((progressIndex(phase) - 1) / (steps.length - 1)) * 100;
</script>

<div class="onboarding">
  <div class="bg-glow" aria-hidden="true"></div>

  <header class="top">
    <div class="brand">
      <span class="brand-icon"><Printer size={22} strokeWidth={1.9} /></span>
      <div class="brand-name">CC2 Monitor</div>
      <span class="brand-sub">Setup</span>
    </div>

    <div class="progress-wrap" aria-label="Setup progress">
      <div class="progress-track">
        <div class="progress-fill" style="width: {progressPct}%"></div>
      </div>
      <ol class="progress-labels">
        {#each steps as s}
          {@const cur = progressIndex(phase)}
          <li class:done={cur > s.n} class:active={cur === s.n}>
            <span class="dot">
              {#if cur > s.n}
                <svg width="10" height="10" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                  <path d="M3 6.5l2 2 4-5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              {:else}
                <span class="dot-n">{s.n}</span>
              {/if}
            </span>
            <span class="step-label">{s.label}</span>
          </li>
        {/each}
      </ol>
    </div>
  </header>

  <main class="stage">
    {#key phase}
      <div class="step-wrap" in:fly={{ y: 10, duration: 260, easing: cubicOut }} out:fade={{ duration: 120 }}>
        {#if phase === 1}
          <PhaseOnePrinter on:complete={onPhase1Complete} />
        {:else if phase === 2}
          <PhaseTwoDetection on:next={onPhase2Next} on:back={onPhase2Back} />
        {:else if phase === 3}
          <PhaseThreeNotifications
            obicoUrl={detectSettings.obicoUrl}
            notifyThreshold={detectSettings.notifyThreshold}
            pauseThreshold={detectSettings.pauseThreshold}
            on:back={onPhase3Back}
            on:complete={onPhase3Complete}
          />
        {:else if phase === 7}
          <section class="card center">
            <div class="done">
              <div class="done-mark">
                <svg width="52" height="52" viewBox="0 0 52 52" fill="none">
                  <circle cx="26" cy="26" r="24" stroke="var(--success)" stroke-width="1.5"/>
                  <path d="M15 26.5l8 8 14-16" stroke="var(--success)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
              <h2>You're all set</h2>
              <p>Opening the dashboard…</p>
            </div>
          </section>
        {/if}
      </div>
    {/key}
  </main>
</div>

<style>
  .onboarding {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 32px 24px 64px;
    background: var(--bg);
    position: relative;
    overflow-x: hidden;
  }
  .bg-glow {
    position: absolute;
    inset: -120px -120px auto -120px;
    height: 420px;
    background:
      radial-gradient(circle at 30% 0%, rgba(45,135,240,0.10), transparent 55%),
      radial-gradient(circle at 80% 0%, rgba(45,135,240,0.06), transparent 55%);
    pointer-events: none;
    z-index: 0;
  }

  .top {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    max-width: 880px;
    width: 100%;
    margin: 0 auto 30px;
    gap: 28px;
  }
  .brand { display: flex; align-items: center; gap: 10px; }
  .brand-icon { color: var(--accent); display: inline-flex; }
  .brand-name { font-size: 14px; font-weight: 700; letter-spacing: -0.01em; }
  .brand-sub {
    font-size: 10px;
    color: var(--muted);
    letter-spacing: 0.14em;
    text-transform: uppercase;
    padding-left: 8px;
    border-left: 1px solid var(--border);
  }

  .progress-wrap { flex: 1; max-width: 460px; display: flex; flex-direction: column; gap: 8px; }
  .progress-track { position: relative; height: 3px; background: var(--surface2); border-radius: var(--radius-pill); overflow: hidden; }
  .progress-fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: linear-gradient(90deg, var(--accent), var(--accent-hi));
    border-radius: var(--radius-pill);
    transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  }
  .progress-labels { list-style: none; display: flex; justify-content: space-between; }
  .progress-labels li {
    display: flex; align-items: center; gap: 7px;
    font-size: 11px; color: var(--muted); letter-spacing: 0.02em; transition: color 0.2s;
  }
  .progress-labels .dot {
    width: 18px; height: 18px;
    border-radius: 50%;
    background: var(--surface2);
    border: 1px solid var(--border2);
    display: inline-flex; align-items: center; justify-content: center;
    font-size: 10px; font-weight: 600; color: var(--muted);
    transition: background 0.2s, border-color 0.2s, color 0.2s;
  }
  .progress-labels li.active { color: var(--text); }
  .progress-labels li.active .dot { background: var(--accent); border-color: var(--accent); color: #fff; box-shadow: 0 0 0 3px var(--accent-dim); }
  .progress-labels li.done { color: var(--text); }
  .progress-labels li.done .dot { background: var(--success-dim); border-color: var(--success); color: var(--success); }
  .dot-n { line-height: 1; }

  .stage { position: relative; z-index: 1; max-width: 880px; width: 100%; margin: 0 auto; flex: 1; }
  .step-wrap { width: 100%; }

  .card {
    background: linear-gradient(180deg, var(--surface), var(--surface2));
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 30px 32px 26px;
    box-shadow: 0 1px 0 rgba(255,255,255,0.03) inset, 0 16px 40px -20px rgba(0,0,0,0.5), 0 2px 8px rgba(0,0,0,0.2);
  }
  .card.center { text-align: center; padding: 64px 32px; }

  .done { display: flex; flex-direction: column; align-items: center; gap: 12px; }
  .done-mark { animation: pop 0.5s cubic-bezier(0.34, 1.56, 0.64, 1); }
  @keyframes pop { 0% { transform: scale(0); opacity: 0; } 100% { transform: scale(1); opacity: 1; } }
  .done h2 { font-size: 22px; font-weight: 600; }
  .done p { color: var(--muted); font-size: 13px; }

  @media (max-width: 760px) {
    .progress-wrap { max-width: none; }
  }
  @media (max-width: 560px) {
    .progress-labels .step-label { display: none; }
    .top { flex-direction: column; align-items: stretch; gap: 16px; }
  }
</style>
