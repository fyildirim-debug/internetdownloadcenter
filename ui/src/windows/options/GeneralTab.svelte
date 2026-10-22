<script lang="ts">
  // Seçenekler > Genel: başlangıç, pano, tarayıcı entegrasyonu, video paneli, dil ve tema.
  import { onMount } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { api, writeClipboard } from '../../lib/api';
  import { t } from '../../lib/i18n/index.svelte';
  import type { BrowserInfo, Settings } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
  let browsers = $state<BrowserInfo[]>([]);
  let chosen = $state<Set<string>>(new Set());
  let busy = $state(false);
  let copied = $state('');

  async function refresh() {
    browsers = await api.browsersStatus();
    chosen = new Set(browsers.filter((b) => b.installed).map((b) => b.id));
  }
  onMount(refresh);

  async function install() {
    busy = true;
    try {
      browsers = await api.browsersInstall([...chosen]);
    } finally {
      busy = false;
    }
  }

  function toggle(id: string) {
    const n = new Set(chosen);
    if (n.has(id)) n.delete(id);
    else n.add(id);
    chosen = n;
  }

  async function copy(b: BrowserInfo) {
    await writeClipboard(b.extensionPath);
    copied = b.id;
    setTimeout(() => (copied = ''), 1500);
  }

  const keys = ['none', 'alt', 'shift', 'ctrl'];
</script>

<label class="check"><input type="checkbox" bind:checked={s.general.launchOnStartup} />{t('opt.gen.startup')}</label>
<label class="check"><input type="checkbox" bind:checked={s.general.closeToTray} />{t('opt.gen.tray')}</label>
<label class="check"><input type="checkbox" bind:checked={s.general.clipboardMonitor} />{t('opt.gen.clipboard')}</label>
<label class="check"><input type="checkbox" bind:checked={s.general.playSounds} />{t('opt.gen.sounds')}</label>

<fieldset class="mt">
  <legend>{t('opt.gen.browsers')}</legend>
  <label class="check"><input type="checkbox" bind:checked={s.general.browserIntegration} />{t('opt.gen.integration')}</label>
  <div class="table-wrap blist">
    <table class="grid">
      <colgroup><col style:width="28px" /><col style:width="150px" /><col style:width="130px" /><col /></colgroup>
      <thead><tr><th></th><th>{t('opt.gen.browserCol')}</th><th>{t('opt.gen.stateCol')}</th><th></th></tr></thead>
      <tbody>
        {#each browsers as b (b.id)}
          <tr class:off={!b.installed}>
            <td><input type="checkbox" checked={chosen.has(b.id)} disabled={!b.installed} onchange={() => toggle(b.id)} /></td>
            <td><Icon name="browser" /> {b.name}</td>
            <td>
              {#if !b.installed}
                <span class="dim">{t('opt.gen.notInstalled')}</span>
              {:else if b.hostRegistered}
                <span class="ok"><Icon name="check" size={12} /> {t('opt.gen.registered')}</span>
              {:else}
                <span class="warn">{t('opt.gen.notRegistered')}</span>
              {/if}
              {#if b.sandboxed}<span class="dim small" title={t('opt.gen.sandboxed')}> ⚠</span>{/if}
            </td>
            <td class="acts">
              {#if b.installed}
                <a href={'#'} onclick={(e) => { e.preventDefault(); copy(b); }}>{copied === b.id ? t('common.copied') : t('opt.gen.copyPath')}</a>
                · <a href={'#'} onclick={(e) => { e.preventDefault(); api.browserOpenExtensions(b.id); }}>{t('opt.gen.openExtPage')}</a>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="row mt">
    <button onclick={install} disabled={busy || !chosen.size}>{t('opt.gen.install')}</button>
    <button class="small" onclick={refresh}><Icon name="refresh" size={12} />{t('opt.gen.refresh')}</button>
  </div>
</fieldset>

<label class="check"><input type="checkbox" bind:checked={s.general.videoPanel} />{t('opt.gen.videoPanel')}</label>
<div class="grid2 mt">
  <label for="g-bypass">{t('opt.gen.bypassKey')}</label>
  <select id="g-bypass" bind:value={s.general.bypassKey}>
    {#each keys as k (k)}<option value={k}>{t(`opt.gen.key.${k}`)}</option>{/each}
  </select>
  <label for="g-force">{t('opt.gen.forceKey')}</label>
  <select id="g-force" bind:value={s.general.forceKey}>
    {#each keys as k (k)}<option value={k}>{t(`opt.gen.key.${k}`)}</option>{/each}
  </select>
  <label for="g-lang">{t('opt.gen.language')}</label>
  <select id="g-lang" bind:value={s.language}>
    <option value="auto">{t('opt.gen.lang.auto')}</option>
    <option value="tr">Türkçe</option>
    <option value="en">English</option>
  </select>
  <label for="g-theme">{t('opt.gen.theme')}</label>
  <select id="g-theme" bind:value={s.theme}>
    <option value="light">{t('menu.themeLight')}</option>
    <option value="dark">{t('menu.themeDark')}</option>
    <option value="system">{t('menu.themeSystem')}</option>
  </select>
</div>

<style>
  .mt {
    margin-top: 6px;
  }
  .blist {
    max-height: 148px;
    margin-top: 4px;
  }
  tr.off td {
    color: var(--text-disabled);
  }
  .ok {
    color: var(--ok);
  }
  .warn {
    color: var(--warn);
  }
  .acts {
    font-size: 11px;
  }
  .grid2 {
    display: grid;
    grid-template-columns: 220px 160px;
    gap: 5px 8px;
    align-items: center;
  }
</style>
