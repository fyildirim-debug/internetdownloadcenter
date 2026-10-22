<script lang="ts">
  // Seçenekler > Bağlantı: bağlantı (parça) sayısı, eşzamanlı indirme, sunucu sınırları, hız sınırı.
  import Icon from '../../lib/icons/Icon.svelte';
  import { t } from '../../lib/i18n/index.svelte';
  import type { Settings } from '../../lib/types';
  import { MAX_CONNECTIONS } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
  let selIdx = $state<number | null>(null);
</script>

<fieldset>
  <legend>{t('opt.cn.connections')}</legend>
  <div class="row">
    <input type="range" min="1" max={MAX_CONNECTIONS} step="1" bind:value={s.downloads.defaultConnections} class="range" />
    <b class="cnt">{s.downloads.defaultConnections}</b>
  </div>
  <div class="ticks">
    {#each [1, 2, 4, 8, 12, 16] as n (n)}<span style:left="{((n - 1) / (MAX_CONNECTIONS - 1)) * 100}%">{n}</span>{/each}
  </div>
  <p class="dim small">{t('opt.cn.connectionsHint')}</p>
</fieldset>

<div class="grid2">
  <label for="cn-max">{t('opt.cn.maxSimultaneous')}</label>
  <input id="cn-max" type="number" min="0" max="64" bind:value={s.downloads.maxSimultaneous} />
  <label for="cn-to">{t('opt.cn.timeout')}</label>
  <input id="cn-to" type="number" min="5" max="600" bind:value={s.connection.timeoutSec} />
</div>
<label class="check"><input type="checkbox" bind:checked={s.connection.adaptive} />{t('opt.cn.adaptive')}</label>
<label class="check"><input type="checkbox" bind:checked={s.connection.http2} />{t('opt.cn.http2')}</label>

<fieldset class="mt">
  <legend>{t('menu.speedLimiter')}</legend>
  <label class="check"><input type="checkbox" bind:checked={s.connection.speedLimitEnabled} />{t('opt.cn.limitEnable')}</label>
  <div class="row">
    <label for="cn-lim">{t('opt.cn.limitValue')}</label>
    <input id="cn-lim" type="number" min="1" step="64" bind:value={s.connection.speedLimitKib} disabled={!s.connection.speedLimitEnabled} style:flex="0 0 100px" />
  </div>
</fieldset>

<fieldset>
  <legend>{t('opt.cn.serverLimits')}</legend>
  <div class="table-wrap sl">
    <table class="grid">
      <colgroup><col /><col style:width="110px" /></colgroup>
      <thead><tr><th>{t('opt.cn.host')}</th><th>{t('col.connections')}</th></tr></thead>
      <tbody>
        {#each s.connection.serverLimits as l, i (i)}
          <tr class:selected={selIdx === i} onclick={() => (selIdx = i)}>
            <td><input type="text" bind:value={l.host} class="cell" /></td>
            <td>
              <select bind:value={l.connections} class="cell">
                {#each Array.from({ length: MAX_CONNECTIONS }, (_, k) => k + 1) as n (n)}<option value={n}>{n}</option>{/each}
              </select>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="row mt">
    <button class="small" onclick={() => { s.connection.serverLimits.push({ host: '*.example.com', connections: 2 }); selIdx = s.connection.serverLimits.length - 1; }}><Icon name="plus" size={12} />{t('common.add')}</button>
    <button class="small" disabled={selIdx == null} onclick={() => { if (selIdx != null) s.connection.serverLimits.splice(selIdx, 1); selIdx = null; }}><Icon name="minus" size={12} />{t('common.delete')}</button>
  </div>
</fieldset>

<style>
  .range {
    flex: 1;
    accent-color: #2a83e0;
  }
  .cnt {
    width: 28px;
    text-align: right;
    font-size: 14px;
  }
  .ticks {
    position: relative;
    height: 14px;
    margin: 0 34px 0 8px;
    font-size: 10px;
    color: var(--text-dim);
  }
  .ticks span {
    position: absolute;
    transform: translateX(-50%);
  }
  .grid2 {
    display: grid;
    grid-template-columns: 260px 90px;
    gap: 5px 8px;
    align-items: center;
    margin-bottom: 4px;
  }
  .mt {
    margin-top: 6px;
  }
  .sl {
    height: 92px;
  }
  .cell {
    width: 100%;
    height: 18px;
    border-color: transparent;
    background: transparent;
    padding: 0 2px;
  }
  .cell:focus {
    border-color: var(--input-focus);
    background: var(--input);
  }
</style>
