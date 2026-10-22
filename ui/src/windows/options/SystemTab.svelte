<script lang="ts">
  // Seçenekler > Sistem: masaüstü ortamı, başlangıç, adres işleyicisi, KDE varsayılan indirme yöneticisi.
  import { onMount } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { api } from '../../lib/api';
  import { t } from '../../lib/i18n/index.svelte';
  import type { AppInfo, OsIntegration, Settings } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
  let os = $state<OsIntegration | null>(null);
  let info = $state<AppInfo | null>(null);
  let applied = $state(false);
  let error = $state('');

  onMount(async () => {
    [os, info] = await Promise.all([api.osIntegrationStatus(), api.appInfo()]);
  });

  async function apply() {
    if (!os) return;
    error = '';
    try {
      os = await api.osIntegrationApply($state.snapshot(os) as OsIntegration);
      s.general.launchOnStartup = os.autostart;
      applied = true;
      setTimeout(() => (applied = false), 2000);
    } catch (e) {
      error = String(e);
    }
  }
</script>

{#if os}
  <div class="facts">
    <span class="dim">{t('opt.sy.desktop')}</span><b>{os.desktop.toUpperCase()}</b>
    <span class="dim">{t('opt.sy.session')}</span><b>{os.session}</b>
    {#if info}
      <span class="dim">{t('opt.sy.version')}</span><span>{info.version} ({info.os})</span>
      <span class="dim">{t('opt.sy.dataDir')}</span><span class="selectable small">{info.dataDir}</span>
    {/if}
  </div>
  <fieldset class="mt">
    <label class="check"><input type="checkbox" bind:checked={os.autostart} />{t('opt.sy.autostart')}</label>
    <label class="check"><input type="checkbox" bind:checked={os.urlHandler} />{t('opt.sy.urlHandler')}</label>
    <label class="check"><input type="checkbox" bind:checked={os.contextMenu} />{t('opt.sy.contextMenu')}</label>
    {#if os.kdeDownloadManager != null}
      <label class="check"><input type="checkbox" checked={os.kdeDownloadManager} onchange={(e) => (os!.kdeDownloadManager = (e.target as HTMLInputElement).checked)} />{t('opt.sy.kde')}</label>
    {/if}
  </fieldset>
  {#if !os.traySupported || os.desktop === 'gnome'}
    <div class="notice warn"><Icon name="warning" />{t('opt.sy.trayWarn')}</div>
  {/if}
  {#if error}<div class="notice error mt"><Icon name="error" />{error}</div>{/if}
  <div class="row mt">
    <button onclick={apply}>{t('opt.sy.apply')}</button>
    {#if applied}<span class="ok"><Icon name="check" size={12} /> {t('opt.sy.applied')}</span>{/if}
  </div>
{:else}
  <p class="dim">{t('common.loading')}</p>
{/if}

<style>
  .facts {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 4px 8px;
  }
  .mt {
    margin-top: 8px;
  }
  .ok {
    color: var(--ok);
  }
</style>
