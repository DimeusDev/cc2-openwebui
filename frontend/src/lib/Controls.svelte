<script lang="ts">
  import { printer } from '../stores';
  import AuxControls from './AuxControls.svelte';

  function jogAxis(_axis: string, _dist: number) {}
  function homeAxes(_axes: string) {}

  $: s = $printer.state;

  let collapsed = false;

  let step = 10;
  const STEPS = [0.1, 1, 10, 30];
</script>

<div class="panel">
  <div class="panel-header" role="button" tabindex="0"
    on:click={() => collapsed = !collapsed}
    on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && (collapsed = !collapsed)}>
    <span class="panel-title">Control</span>
    <svg class="chevron {collapsed ? 'up' : ''}" width="12" height="12" viewBox="0 0 14 14" fill="none">
      <path d="M3 5l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </div>

  {#if !collapsed}
    <div class="body">
      <div class="control-grid">

        <div class="jog-col">
          <div class="pos-steps-row">
            <div class="pos-row">
              {#each [
                {ax: 'X', val: s?.gcode_move?.x},
                {ax: 'Y', val: s?.gcode_move?.y},
                {ax: 'Z', val: s?.gcode_move?.z},
              ] as p}
                <span class="pos-item">
                  <span class="pos-axis">{p.ax}:</span>
                  <span class="pos-val mono">{p.val != null ? Number(p.val).toFixed(1) : '0.0'}</span>
                </span>
              {/each}
            </div>

            <div class="steps">
              {#each STEPS as sv}
                <button class="step-chip" class:active={step === sv} on:click={() => step = sv}>{sv}mm</button>
              {/each}
            </div>
          </div>

          <div class="jog-row">
            <div class="pill-group">
              <button class="pill-btn top" title="Z+" on:click={() => jogAxis('z', step)} disabled>
                <span class="pill-lbl">Z<span class="arrow">↑</span></span>
              </button>
              <button class="pill-btn mid" title="Home Z" on:click={() => homeAxes('z')} disabled>
                <svg width="14" height="14" viewBox="0 0 20.894287109375 19.5634765625" fill="none" aria-hidden="true">
                  <path d="M12.5465,15.3319C12.5465,14.8782,12.2069,13.934,11.1495,13.934C11.1495,13.934,9.75109,13.934,9.75109,13.934C8.83292,13.934,8.35404,14.5988,8.35404,15.3319C8.35404,20.3021,8.35404,19.5185,8.35404,19.5185C8.35404,19.5185,5.13098,19.5185,5.13098,19.5185C3.7858,19.5185,2.74296,18.3669,2.74296,17.1888C2.74296,17.1888,2.74296,10.6833,2.74296,10.6833C2.74296,10.6833,1.19216,10.6833,1.19216,10.6833C0.523175,10.6833,0.199428,10.2178,0.0684266,9.92291C0.0165834,9.80462,-0.204882,9.17167,0.579099,8.35853C0.579099,8.35853,8.72609,0.68981,8.72609,0.68981C9.15537,0.245242,9.73017,0,10.3446,0C10.9589,0,11.5338,0.245242,11.9631,0.690256C11.9631,0.690256,20.3105,8.35358,20.3105,8.35358C20.3119,8.35541,20.3138,8.35719,20.3156,8.35858C21.0991,9.17217,20.8777,9.80417,20.8258,9.92295C20.6948,10.2178,20.372,10.6833,19.7021,10.6833C19.7021,10.6833,18.1468,10.6833,18.1468,10.6833C18.1468,10.6833,18.1468,17.1888,18.1468,17.1888C18.1468,18.3669,17.0594,19.5186,15.7138,19.5186C15.7138,19.5186,12.5466,19.5186,12.5466,19.5186C12.5465,19.5185,12.5465,19.5071,12.5465,15.3319Z"
                        fill="currentColor"/>
                  <path class="home-z-text" d="M8,10.61L12.36333,10.61L12.36333,9.84547L9.22088,9.84547L12.26779,5.50624L12.26779,5L8.329108,5L8.329108,5.76453L11.03629,5.76453L8,10.10376L8,10.61Z"/>
                </svg>
              </button>
              <button class="pill-btn bot" title="Z-" on:click={() => jogAxis('z', -step)} disabled>
                <span class="pill-lbl">Z<span class="arrow">↓</span></span>
              </button>
            </div>

            <div class="disc" aria-label="XY jog pad">
              <svg viewBox="0 0 120 120" class="disc-svg" aria-hidden="true">
                <path class="quarter q-top"    d="M60 60 L20.4 20.4 A56 56 0 0 1 99.6 20.4 Z" />
                <path class="quarter q-right"  d="M60 60 L99.6 20.4 A56 56 0 0 1 99.6 99.6 Z" />
                <path class="quarter q-bottom" d="M60 60 L99.6 99.6 A56 56 0 0 1 20.4 99.6 Z" />
                <path class="quarter q-left"   d="M60 60 L20.4 99.6 A56 56 0 0 1 20.4 20.4 Z" />

                <line x1="20.4" y1="20.4" x2="99.6" y2="99.6" stroke="var(--border2)" stroke-width="1" opacity="0.8"/>
                <line x1="20.4" y1="99.6" x2="99.6" y2="20.4" stroke="var(--border2)" stroke-width="1" opacity="0.8"/>

                <circle cx="60" cy="60" r="56" fill="none" stroke="var(--border2)" stroke-width="1"/>

                <path class="quarter-hit q-top-hit"    d="M60 60 L20.4 20.4 A56 56 0 0 1 99.6 20.4 Z"
                      on:click={() => jogAxis('y', step)}
                      on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && jogAxis('y', step)}
                      role="button" tabindex="0" aria-label="Jog Y+"/>
                <path class="quarter-hit q-right-hit"  d="M60 60 L99.6 20.4 A56 56 0 0 1 99.6 99.6 Z"
                      on:click={() => jogAxis('x', step)}
                      on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && jogAxis('x', step)}
                      role="button" tabindex="0" aria-label="Jog X+"/>
                <path class="quarter-hit q-bottom-hit" d="M60 60 L99.6 99.6 A56 56 0 0 1 20.4 99.6 Z"
                      on:click={() => jogAxis('y', -step)}
                      on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && jogAxis('y', -step)}
                      role="button" tabindex="0" aria-label="Jog Y-"/>
                <path class="quarter-hit q-left-hit"   d="M60 60 L20.4 99.6 A56 56 0 0 1 20.4 20.4 Z"
                      on:click={() => jogAxis('x', -step)}
                      on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && jogAxis('x', -step)}
                      role="button" tabindex="0" aria-label="Jog X-"/>

                <text x="60" y="28" text-anchor="middle" dominant-baseline="central" class="axis-lbl">Y+</text>
                <text x="60" y="92" text-anchor="middle" dominant-baseline="central" class="axis-lbl">Y-</text>
                <text x="28" y="60" text-anchor="middle" dominant-baseline="central" class="axis-lbl">X-</text>
                <text x="92" y="60" text-anchor="middle" dominant-baseline="central" class="axis-lbl">X+</text>
              </svg>

              <button class="center-btn" title="Home All" on:click={() => homeAxes('xyz')} disabled>
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <path d="M14.099234375,17.550100683593747C14.099234375,17.09640068359375,13.759634375,16.152200683593747,12.702234375,16.152200683593747C12.702234375,16.152200683593747,11.303824375,16.152200683593747,11.303824375,16.152200683593747C10.385654375,16.152200683593747,9.906774375,16.81700068359375,9.906774375,17.550100683593747C9.906774375,22.52030068359375,9.906774375,21.73670068359375,9.906774375,21.73670068359375C9.906774375,21.73670068359375,6.683714375,21.73670068359375,6.683714375,21.73670068359375C5.338534375,21.73670068359375,4.295694375,20.58510068359375,4.295694375,19.40700068359375C4.295694375,19.40700068359375,4.295694375,12.90150068359375,4.295694375,12.90150068359375C4.295694375,12.90150068359375,2.7448943750000003,12.90150068359375,2.7448943750000003,12.90150068359375C2.075909375,12.90150068359375,1.752162375,12.43600068359375,1.621160975,12.14111068359375C1.569317775,12.02282068359375,1.347852375,11.38987068359375,2.1318333750000003,10.57673068359375C2.1318333750000003,10.57673068359375,10.278824375,2.90801068359375,10.278824375,2.90801068359375C10.708104375,2.46344268359375,11.282904375,2.21820068359375,11.897334375,2.21820068359375C12.511634375,2.21820068359375,13.086534375,2.46344268359375,13.515834375,2.9084566835937498C13.515834375,2.9084566835937498,21.863234375,10.57178068359375,21.863234375,10.57178068359375C21.864634375,10.57361068359375,21.866534375,10.57539068359375,21.868334375,10.57678068359375C22.651834375,11.39037068359375,22.430434375,12.02237068359375,22.378534375,12.14115068359375C22.247534375,12.43600068359375,21.924734375,12.90150068359375,21.254834375,12.90150068359375C21.254834375,12.90150068359375,19.699534375,12.90150068359375,19.699534375,12.90150068359375C19.699534375,12.90150068359375,19.699534375,19.40700068359375,19.699534375,19.40700068359375C19.699534375,20.58510068359375,18.612134375,21.73680068359375,17.266534375,21.73680068359375C17.266534375,21.73680068359375,14.099334375,21.73680068359375,14.099334375,21.73680068359375C14.099234375,21.73670068359375,14.099234375,21.72530068359375,14.099234375,17.550100683593747Z"
                        fill="currentColor"/>
                  <path class="home-all-text" d="M9.15173375,14.166649091796874C9.66828375,14.166649091796874,10.13858375,13.896979091796876,10.53949375,13.565659091796874L10.56261375,13.565659091796874L10.62429375,14.066489091796875L11.202523750000001,14.066489091796875L11.202523750000001,11.493019091796874C11.202523750000001,10.452849091796875,10.778493749999999,9.774809091796875,9.75309375,9.774809091796875C9.07463375,9.774809091796875,8.48869575,10.075309091796875,8.11091775,10.321869091796875L8.38075975,10.807279091796875C8.71227875,10.583839091796875,9.15173375,10.360389091796876,9.63745375,10.360389091796876C10.32361375,10.360389091796876,10.50094375,10.876619091796876,10.50094375,11.415969091796875C8.71998875,11.616299091796876,7.93359375,12.070899091796875,7.93359375,12.980089091796875C7.93359375,13.735169091796875,8.45014675,14.166649091796874,9.15173375,14.166649091796874ZM9.35218375,13.596479091796876C8.93586375,13.596479091796876,8.61205175,13.411559091796875,8.61205175,12.933859091796876C8.61205175,12.394509091796875,9.09005375,12.047779091796876,10.50094375,11.885979091796875L10.50094375,13.049429091796874C10.09232375,13.411559091796875,9.75309375,13.596479091796876,9.35218375,13.596479091796876ZM13.268743749999999,14.166649091796874C13.46148375,14.166649091796874,13.57713375,14.135829091796875,13.67735375,14.105009091796875L13.57713375,13.565659091796874C13.50003375,13.581069091796875,13.469193749999999,13.581069091796875,13.43064375,13.581069091796875C13.32271375,13.581069091796875,13.237903750000001,13.496319091796874,13.237903750000001,13.280579091796875L13.237903750000001,7.933319091796875L12.52860375,7.933319091796875L12.52860375,13.234349091796876C12.52860375,13.827629091796876,12.744473750000001,14.166649091796874,13.268743749999999,14.166649091796874ZM15.45831375,14.166649091796874C15.651053749999999,14.166649091796874,15.76670375,14.135829091796875,15.86692375,14.105009091796875L15.76670375,13.565659091796874C15.68960375,13.581069091796875,15.65876375,13.581069091796875,15.62021375,13.581069091796875C15.51227375,13.581069091796875,15.42747375,13.496319091796874,15.42747375,13.280579091796875L15.42747375,7.933319091796875L14.71817375,7.933319091796875L14.71817375,13.234349091796876C14.71817375,13.827629091796876,14.93404375,14.166649091796874,15.45831375,14.166649091796874Z"/>
                </svg>
              </button>
            </div>

            <div class="pill-group right">
              <button class="pill-btn top" title="Home X" on:click={() => homeAxes('x')} disabled>
                <svg width="14" height="14" viewBox="0 0 20.894287109375 19.5634765625" fill="none" aria-hidden="true">
                  <path d="M12.5465,15.3319C12.5465,14.8782,12.2069,13.934,11.1495,13.934C11.1495,13.934,9.75109,13.934,9.75109,13.934C8.83292,13.934,8.35404,14.5988,8.35404,15.3319C8.35404,20.3021,8.35404,19.5185,8.35404,19.5185C8.35404,19.5185,5.13098,19.5185,5.13098,19.5185C3.7858,19.5185,2.74296,18.3669,2.74296,17.1888C2.74296,17.1888,2.74296,10.6833,2.74296,10.6833C2.74296,10.6833,1.19216,10.6833,1.19216,10.6833C0.523175,10.6833,0.199428,10.2178,0.0684266,9.92291C0.0165834,9.80462,-0.204882,9.17167,0.579099,8.35853C0.579099,8.35853,8.72609,0.68981,8.72609,0.68981C9.15537,0.245242,9.73017,0,10.3446,0C10.9589,0,11.5338,0.245242,11.9631,0.690256C11.9631,0.690256,20.3105,8.35358,20.3105,8.35358C20.3119,8.35541,20.3138,8.35719,20.3156,8.35858C21.0991,9.17217,20.8777,9.80417,20.8258,9.92295C20.6948,10.2178,20.372,10.6833,19.7021,10.6833C19.7021,10.6833,18.1468,10.6833,18.1468,10.6833C18.1468,10.6833,18.1468,17.1888,18.1468,17.1888C18.1468,18.3669,17.0594,19.5186,15.7138,19.5186C15.7138,19.5186,12.5466,19.5186,12.5466,19.5186C12.5465,19.5185,12.5465,19.5071,12.5465,15.3319Z"
                        fill="currentColor"/>
                  <text x="10.45" y="9.4" text-anchor="middle" dominant-baseline="central" class="home-axis-text">X</text>
                </svg>
              </button>
              <button class="pill-btn bot" title="Home Y" on:click={() => homeAxes('y')} disabled>
                <svg width="14" height="14" viewBox="0 0 20.894287109375 19.5634765625" fill="none" aria-hidden="true">
                  <path d="M12.5465,15.3319C12.5465,14.8782,12.2069,13.934,11.1495,13.934C11.1495,13.934,9.75109,13.934,9.75109,13.934C8.83292,13.934,8.35404,14.5988,8.35404,15.3319C8.35404,20.3021,8.35404,19.5185,8.35404,19.5185C8.35404,19.5185,5.13098,19.5185,5.13098,19.5185C3.7858,19.5185,2.74296,18.3669,2.74296,17.1888C2.74296,17.1888,2.74296,10.6833,2.74296,10.6833C2.74296,10.6833,1.19216,10.6833,1.19216,10.6833C0.523175,10.6833,0.199428,10.2178,0.0684266,9.92291C0.0165834,9.80462,-0.204882,9.17167,0.579099,8.35853C0.579099,8.35853,8.72609,0.68981,8.72609,0.68981C9.15537,0.245242,9.73017,0,10.3446,0C10.9589,0,11.5338,0.245242,11.9631,0.690256C11.9631,0.690256,20.3105,8.35358,20.3105,8.35358C20.3119,8.35541,20.3138,8.35719,20.3156,8.35858C21.0991,9.17217,20.8777,9.80417,20.8258,9.92295C20.6948,10.2178,20.372,10.6833,19.7021,10.6833C19.7021,10.6833,18.1468,10.6833,18.1468,10.6833C18.1468,10.6833,18.1468,17.1888,18.1468,17.1888C18.1468,18.3669,17.0594,19.5186,15.7138,19.5186C15.7138,19.5186,12.5466,19.5186,12.5466,19.5186C12.5465,19.5185,12.5465,19.5071,12.5465,15.3319Z"
                        fill="currentColor"/>
                  <text x="10.45" y="9.4" text-anchor="middle" dominant-baseline="central" class="home-axis-text">Y</text>
                </svg>
              </button>
            </div>
          </div>
        </div>

        <AuxControls />

      </div>
    </div>
  {/if}
</div>

<style>
  .chevron { color: var(--muted); transition: transform 0.2s; }
  .chevron.up { transform: rotate(-180deg); }

  .body { padding: 14px 16px 16px; }

  .control-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 24px;
    align-items: start;
  }

  /* Left jog column */
  .jog-col { display: flex; flex-direction: column; gap: 14px; }

  .pos-steps-row {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .pos-row {
    display: flex;
    gap: 14px;
    padding: 6px 8px;
    background: var(--bg-deep);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .pos-item { display: flex; gap: 4px; font-size: 11px; }
  .pos-axis { color: var(--muted); font-weight: 600; }
  .pos-val { color: var(--text); }

  .steps { display: flex; gap: 4px; }
  .step-chip {
    padding: 5px 10px;
    font-size: 11px;
    font-weight: 500;
    color: var(--muted);
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    transition: color 0.12s, background 0.12s, border-color 0.12s;
  }
  .step-chip:hover { color: var(--text); border-color: var(--border2); }
  .step-chip.active {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  .jog-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
  }

  /* Pill groups */
  .pill-group {
    display: flex;
    flex-direction: column;
    gap: 0;
    overflow: hidden;
    border-radius: 22px;
    border: 1px solid var(--border);
    background: var(--surface2);
  }
  .pill-btn {
    width: 36px;
    height: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    transition: background 0.12s, color 0.12s;
    border: none;
    background: transparent;
  }
  .pill-btn + .pill-btn { border-top: 1px solid var(--border); }
  .pill-btn:hover:not(:disabled) {
    background: var(--accent-dim);
    color: var(--accent);
  }
  .pill-btn:disabled { opacity: 0.55; cursor: not-allowed; }
  .pill-btn:disabled:hover { background: var(--accent-dim); color: var(--accent); opacity: 0.55; } /* preserve hover per spec */
  .pill-lbl {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    display: inline-flex;
    align-items: baseline;
    gap: 1px;
  }
  .pill-lbl .arrow { font-size: 13px; }

  /* disc */
  .disc {
    width: 132px;
    height: 132px;
    position: relative;
    flex-shrink: 0;
  }
  .disc-svg { width: 100%; height: 100%; display: block; }

  .quarter {
    fill: var(--surface2);
    stroke: none;
    transition: fill 0.12s;
  }
  .quarter-hit {
    fill: transparent;
    cursor: pointer;
    pointer-events: all;
  }
  .quarter-hit:focus-visible { outline: none; }
  .disc:has(.q-top-hit:hover) .q-top { fill: var(--accent-dim); }
  .disc:has(.q-right-hit:hover) .q-right { fill: var(--accent-dim); }
  .disc:has(.q-bottom-hit:hover) .q-bottom { fill: var(--accent-dim); }
  .disc:has(.q-left-hit:hover) .q-left { fill: var(--accent-dim); }

  .axis-lbl {
    font-family: var(--font-body);
    font-size: 10px;
    font-weight: 700;
    fill: var(--muted);
    letter-spacing: 0.04em;
    pointer-events: none;
    user-select: none;
  }

  .center-btn {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: var(--bg-deep);
    border: 1px solid var(--border2);
    color: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.12s, border-color 0.12s, background 0.12s;
    z-index: 2;
  }
  .center-btn:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-dim);
  }
  .center-btn:disabled { opacity: 0.55; cursor: not-allowed; }
  .home-all-text { fill: var(--bg-deep); }
  .center-btn:hover:not(:disabled) .home-all-text { fill: var(--accent-dim); }

  .home-z-text { fill: var(--surface2); }
  .pill-btn:hover:not(:disabled) .home-z-text { fill: var(--accent-dim); }
  .home-axis-text {
    fill: var(--surface2);
    font-size: 7.2px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .pill-btn:hover:not(:disabled) .home-axis-text { fill: var(--accent-dim); }

  @media (max-width: 640px) {
    .control-grid { grid-template-columns: 1fr; }
    .jog-row { justify-content: center; }
  }
</style>
