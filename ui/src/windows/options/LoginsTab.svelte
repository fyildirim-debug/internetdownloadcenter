<script lang="ts">
  // Seçenekler > Site girişleri.
  import Icon from '../../lib/icons/Icon.svelte';
  import { t } from '../../lib/i18n/index.svelte';
  import type { Settings } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
  let selIdx = $state<number | null>(null);
  let show = $state(false);
</script>

<p class="dim">{t('opt.lg.hint')}</p>
<div class="table-wrap lg">
  <table class="grid">
    <colgroup><col /><col style:width="140px" /><col style:width="140px" /></colgroup>
    <thead><tr><th>{t('opt.lg.pattern')}</th><th>{t('opt.lg.user')}</th><th>{t('opt.lg.pass')}</th></tr></thead>
    <tbody>
      {#each s.siteLogins as l, i (i)}
        <tr class:selected={selIdx === i} onclick={() => (selIdx = i)}>
          <td><input type="text" class="cell mono" bind:value={l.pattern} /></td>
          <td><input type="text" class="cell" bind:value={l.username} /></td>
          <td><input type={show ? 'text' : 'password'} class="cell" bind:value={l.password} /></td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<div class="row mt">
  <button class="small" onclick={() => { s.siteLogins.push({ pattern: 'https://*.example.com/*', username: '', password: '' }); selIdx = s.siteLogins.length - 1; }}><Icon name="plus" size={12} />{t('common.new')}</button>
  <button class="small" disabled={selIdx == null} onclick={() => { if (selIdx != null) s.siteLogins.splice(selIdx, 1); selIdx = null; }}><Icon name="minus" size={12} />{t('common.delete')}</button>
  <span class="sp"></span>
  <label class="check"><input type="checkbox" bind:checked={show} /><Icon name="lock" size={12} /></label>
</div>

<style>
  .lg {
    height: 300px;
  }
  .mt {
    margin-top: 6px;
  }
  .sp {
    flex: 1;
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
