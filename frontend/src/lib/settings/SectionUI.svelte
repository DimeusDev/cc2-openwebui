<script>
  import { get } from 'svelte/store';
  import { ui_settings } from '../../stores';

  function toggleSwitch(id) {
    ui_settings.update(settings =>
      settings.map(switchItem =>
        switchItem.id === id
          ? {
              ...switchItem,
              checked: !switchItem.checked,
              value: switchItem.checked ? 'off' : 'on'
            }
          : switchItem
      )
    );
    localStorage.setItem('ui_settings', JSON.stringify(get(ui_settings))); 
  }

  function applyChanges() {  
    localStorage.setItem('ui_settings', JSON.stringify($ui_settings));  
  }  

</script>

<div class="group">
  {#each $ui_settings as switchItem (switchItem.id)}
    <div class="row">
      <span class="row-label">{switchItem.label}</span>
      <div class="row-input">
        <label class="switch">
          <input type="checkbox" checked={switchItem.checked} on:change={() => toggleSwitch(switchItem.id)}/>
          <span class="slider"></span>
        </label>
      </div>
    </div>
  {/each}
</div>

<style>
  .group {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
    /* Neue Eigenschaften für begrenzte Breite */
    width: auto;       /* Passt sich dem Inhalt an */
    max-width: 200px;  /* Maximale Breite (kannst du anpassen) */
    margin-left: 0 auto;    /* Zentriert den Rahmen */
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 50px;
    align-items: center;
    gap: 10px;
    padding: 5px 10px;
    border-top: 1px solid var(--border);
  }

  .row-label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text);
    min-width: 0;
  }

  .row-input {
    width: 100%;
    justify-self: end;
  }
</style>
