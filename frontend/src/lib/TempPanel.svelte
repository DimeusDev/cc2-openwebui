<script lang="ts">
  import { printer } from '../stores';

  $: s = $printer.state;
  $: nozzleTemp = s?.extruder?.temperature ?? 0;
  $: nozzleTarget = s?.extruder?.target ?? 0;
  $: bedTemp = s?.heater_bed?.temperature ?? 0;
  $: bedTarget = s?.heater_bed?.target ?? 0;
  $: chamberTemp = s?.ztemperature_sensor?.temperature ?? 0;
  $: nStat = tempStatus(nozzleTemp, nozzleTarget);
  $: bStat = tempStatus(bedTemp, bedTarget);

  function tempStatus(current: number, target: number): 'off' | 'heating' | 'ready' {
    if (target === 0) return 'off';
    if (current < target - 5) return 'heating';
    return 'ready';
  }
</script>

<section class="panel">
  <div class="panel-header">
    <span class="panel-title">Temperature</span>
  </div>

  <div class="temp-table">
    <div class="temp-head">
      <span>Component</span>
      <span class="ra">Current</span>
      <span class="ra">Target</span>
    </div>

    <div class="temp-row">
      <div class="col-name">
        <div class="temp-icon nozzle">
          <svg width="13" height="13" viewBox="0 0 14 14" fill="none" aria-hidden="true">
            <path d="M2.5,2L2.5,1C2.5,0.447715,2.94772,0,3.5,0L10.5,0C11.0523,0,11.5,0.447715,11.5,1L11.5,2L13.5,2C13.7761,2,14,2.22386,14,2.5L14,6C14,6.27614,13.7761,6.5,13.5,6.5L11.5,6.5L11.5,7.5C11.5,8.05229,11.0523,8.5,10.5,8.5L10.1,8.5L9.5514,9.8714C9.39953,10.2511,9.03183,10.5,8.62293,10.5L8,10.5L7.5,12L6.5,12L6,10.5L5.37707,10.5C4.96817,10.5,4.60047,10.2511,4.4486,9.8714L3.90002,8.5L3.5,8.5C2.94772,8.5,2.5,8.05229,2.5,7.5L2.5,6.5L0.5,6.5C0.223858,6.5,0,6.27614,0,6L0,2.5C0,2.22386,0.223858,2,0.5,2L2.5,2ZM3.5,1L3.5,7.5L10.5,7.5L10.5,1L3.5,1ZM2.5,3L1,3L1,5.5L2.5,5.5L2.5,3ZM11.5,5.5L11.5,3L13,3L13,5.5L11.5,5.5ZM1.04483,12.157Q1.5,11.6638,1.5,11.2175Q1.5,11.101,1.43798,11.0148Q1.38125,10.936,1.17991,10.7682Q0.881245,10.5194,0.750483,10.3377Q0.5,9.98984,0.5,9.55079Q0.5,8.66742,1.30795,7.84175C1.50108,7.64438,1.82322,7.67199,2,7.88413C2.17678,8.09626,2.14247,8.40838,1.95517,8.6113Q1.5,9.10443,1.5,9.55079Q1.5,9.6673,1.56202,9.75343Q1.61875,9.83223,1.82009,10Q2.11875,10.2489,2.24952,10.4305Q2.5,10.7784,2.5,11.2175Q2.5,12.1008,1.69205,12.9265C1.49892,13.1239,1.17678,13.0963,1,12.8841C0.823218,12.672,0.857533,12.3599,1.04483,12.157ZM12.0448,12.157Q12.5,11.6638,12.5,11.2175Q12.5,11.101,12.438,11.0148Q12.3812,10.936,12.1799,10.7682Q11.8812,10.5194,11.7505,10.3377Q11.5,9.98984,11.5,9.55079Q11.5,8.66742,12.3079,7.84175C12.5011,7.64438,12.8232,7.67199,13,7.88413C13.1768,8.09626,13.1425,8.40838,12.9552,8.6113Q12.5,9.10443,12.5,9.55079Q12.5,9.6673,12.562,9.75343Q12.6188,9.83223,12.8201,10Q13.1188,10.2489,13.2495,10.4305Q13.5,10.7784,13.5,11.2175Q13.5,12.1008,12.6921,12.9265C12.4989,13.1239,12.1768,13.0963,12,12.8841C11.8232,12.672,11.8575,12.3599,12.0448,12.157ZM4.97706,8.5L5.37707,9.5L8.62293,9.5L9.02294,8.5L4.97706,8.5Z" fill-rule="evenodd" fill="currentColor"/>
          </svg>
        </div>
        <span class="name-txt">Nozzle</span>
      </div>
      <div class="col-val ra">
        <span class="temp-cur {nStat} mono">{Math.round(nozzleTemp)}°</span>
      </div>
      <div class="col-tgt ra">
        {#if nozzleTarget > 0}
          <span class="temp-tgt mono">{nozzleTarget}°</span>
        {:else}
          <span class="temp-tgt dim">-</span>
        {/if}
      </div>
    </div>

    <div class="temp-row">
      <div class="col-name">
        <div class="temp-icon bed">
          <svg width="13" height="13" viewBox="0 0 16 12" fill="none" aria-hidden="true">
            <path d="M2.54483,4.4487Q3,3.95557,3,3.50921Q3,3.3927,2.93798,3.30657Q2.88125,3.22777,2.67991,3.05998Q2.38125,2.8111,2.25048,2.62948Q2,2.28159,2,1.84254Q2,0.959167,2.80795,0.133494C3.00108,-0.0638753,3.32322,-0.0362649,3.5,0.175874C3.67678,0.388012,3.64247,0.700128,3.45517,0.903044Q3,1.39618,3,1.84254Q3,1.95904,3.06202,2.04518Q3.11875,2.12398,3.32009,2.29176Q3.61875,2.54065,3.74952,2.72226Q4,3.07016,4,3.50921Q4,4.39258,3.19205,5.21825C2.99892,5.41562,2.67678,5.38801,2.5,5.17587C2.32322,4.96374,2.35753,4.65162,2.54483,4.4487ZM7.54483,4.4487Q8,3.95557,8,3.50921Q8,3.3927,7.93798,3.30657Q7.88125,3.22777,7.67991,3.05998Q7.38125,2.8111,7.25048,2.62948Q7,2.28159,7,1.84254Q7,0.959167,7.80795,0.133494C8.00108,-0.0638753,8.32322,-0.0362649,8.5,0.175874C8.67678,0.388012,8.64247,0.700128,8.45517,0.903044Q8,1.39618,8,1.84254Q8,1.95904,8.06202,2.04518Q8.11875,2.12398,8.32009,2.29176Q8.61875,2.54065,8.74952,2.72226Q9,3.07016,9,3.50921Q9,4.39258,8.19205,5.21825C7.99892,5.41562,7.67678,5.38801,7.5,5.17587C7.32322,4.96374,7.35753,4.65162,7.54483,4.4487ZM12.5448,4.4487Q13,3.95557,13,3.50921Q13,3.3927,12.938,3.30657Q12.8812,3.22777,12.6799,3.05998Q12.3812,2.8111,12.2505,2.62948Q12,2.28159,12,1.84254Q12,0.959167,12.8079,0.133494C13.0011,-0.0638753,13.3232,-0.0362649,13.5,0.175874C13.6768,0.388012,13.6425,0.700128,13.4552,0.903044Q13,1.39618,13,1.84254Q13,1.95904,13.062,2.04518Q13.1188,2.12398,13.3201,2.29176Q13.6188,2.54065,13.7495,2.72226Q14,3.07016,14,3.50921Q14,4.39258,13.1921,5.21825C12.9989,5.41562,12.6768,5.38801,12.5,5.17587C12.3232,4.96374,12.3575,4.65162,12.5448,4.4487ZM0,8.59175L0,6.99175Q0,6.49469,0.351472,6.14322Q0.702944,5.79175,1.2,5.79175L14.8,5.79175Q15.2971,5.79175,15.6485,6.14322Q16,6.49469,16,6.99175L16,8.59175Q16,9.0888,15.6485,9.44028Q15.2971,9.79175,14.8,9.79175L14.4849,9.79175Q14.5,9.88862,14.5,9.99175L14.5,10.5917Q14.5,11.0888,14.1485,11.4403Q13.7971,11.7917,13.3,11.7917L11.7,11.7917Q11.2029,11.7917,10.8515,11.4403Q10.5,11.0888,10.5,10.5917L10.5,9.99175Q10.5,9.88862,10.5151,9.79175L5.48487,9.79175Q5.5,9.88861,5.5,9.99175L5.5,10.5917Q5.5,11.0888,5.14853,11.4403Q4.79706,11.7917,4.3,11.7917L2.7,11.7917Q2.20294,11.7917,1.85147,11.4403Q1.5,11.0888,1.5,10.5917L1.5,9.99175Q1.5,9.88861,1.51513,9.79175L1.2,9.79175Q0.702944,9.79175,0.351472,9.44028Q0,9.0888,0,8.59175ZM2.7,9.79175C2.58954,9.79175,2.5,9.88129,2.5,9.99175L2.5,10.5917C2.5,10.7022,2.58954,10.7917,2.7,10.7917L4.3,10.7917C4.41046,10.7917,4.5,10.7022,4.5,10.5917L4.5,9.99175C4.5,9.88129,4.41046,9.79175,4.3,9.79175L2.7,9.79175ZM4.3,8.79175L14.8,8.79175C14.9105,8.79175,15,8.7022,15,8.59175L15,6.99175C15,6.88129,14.9105,6.79175,14.8,6.79175L1.2,6.79175C1.08954,6.79175,1,6.88129,1,6.99175L1,8.59175C1,8.7022,1.08954,8.79175,1.2,8.79175L4.3,8.79175ZM11.7,9.79175C11.5895,9.79175,11.5,9.88129,11.5,9.99175L11.5,10.5917C11.5,10.7022,11.5895,10.7917,11.7,10.7917L13.3,10.7917C13.4105,10.7917,13.5,10.7022,13.5,10.5917L13.5,9.99175C13.5,9.88129,13.4105,9.79175,13.3,9.79175L11.7,9.79175Z" fill-rule="evenodd" fill="currentColor"/>
          </svg>
        </div>
        <span class="name-txt">Heated Bed</span>
      </div>
      <div class="col-val ra">
        <span class="temp-cur {bStat} mono">{Math.round(bedTemp)}°</span>
      </div>
      <div class="col-tgt ra">
        {#if bedTarget > 0}
          <span class="temp-tgt mono">{bedTarget}°</span>
        {:else}
          <span class="temp-tgt dim">-</span>
        {/if}
      </div>
    </div>

    <div class="temp-row">
      <div class="col-name">
        <div class="temp-icon chamber">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <rect x="1.6" y="1.6" width="12.8" height="12.8" rx="0.4" fill="none" stroke="currentColor" stroke-width="1.2"/>
            <g transform="matrix(-1,0,0,-1,26,22)">
              <rect x="13.65" y="11.65" width="2.7" height="4.7" rx="0.35" fill="none" stroke="currentColor" stroke-width="1.3"/>
            </g>
          </svg>
        </div>
        <span class="name-txt">Chamber</span>
      </div>
      <div class="col-val ra">
        <span class="temp-cur off mono">{Math.round(chamberTemp)}°</span>
      </div>
      <div class="col-tgt ra">
        <span class="temp-tgt dim">-</span>
      </div>
    </div>
  </div>
</section>

<style>
  .temp-table {
    display: flex;
    flex-direction: column;
  }

  .temp-head {
    display: grid;
    grid-template-columns: 1.2fr 1fr 1fr;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface2);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }

  .temp-row {
    display: grid;
    grid-template-columns: 1.2fr 1fr 1fr;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    align-items: center;
  }
  .temp-row:last-child { border-bottom: none; }

  .col-name {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name-txt { font-size: 13px; color: var(--text); }

  .ra { text-align: right; }

  .temp-icon {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .temp-icon.nozzle { background: var(--danger-dim); color: var(--danger); }
  .temp-icon.bed { background: var(--warning-dim); color: var(--warning); }
  .temp-icon.chamber { background: var(--surface2); color: var(--text-dim); border: 1px solid var(--border); }

  .temp-cur {
    font-size: 13px;
    font-weight: 600;
  }
  .temp-cur.off { color: var(--text); }
  .temp-cur.heating { color: var(--warning); }
  .temp-cur.ready { color: var(--success); }

  .temp-tgt {
    font-size: 12px;
    color: var(--muted);
  }
  .temp-tgt.dim { color: var(--muted2); }
</style>
