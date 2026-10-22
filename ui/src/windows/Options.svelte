<script lang="ts">
  // Seçenekler penceresi: ayarların yerel kopyası üzerinde çalışır; Tamam/Uygula kaydeder.
  import { onMount } from 'svelte';
  import Tabs from '../lib/components/Tabs.svelte';
  import { closeWindow, hashParams, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { app, saveSettings } from '../lib/state.svelte';
  import type { Settings } from '../lib/types';
  import GeneralTab from './options/GeneralTab.svelte';
  import FileTypesTab from './options/FileTypesTab.svelte';
  import SaveToTab from './options/SaveToTab.svelte';
  import DownloadsTab from './options/DownloadsTab.svelte';
  import ConnectionTab from './options/ConnectionTab.svelte';
  import ProxyTab from './options/ProxyTab.svelte';
  import LoginsTab from './options/LoginsTab.svelte';
  import VideoTab from './options/VideoTab.svelte';
  import SystemTab from './options/SystemTab.svelte';

  let tab = $state(hashParams().get('tab') ?? 'general');
  let s = $state<Settings>(structuredClone($state.snapshot(app.settings!) as Settings));
  let saving = $state(false);
  let error = $state('');

  let original = $state(JSON.stringify($state.snapshot(app.settings)));
  const dirty = $derived(JSON.stringify(s) !== original);

  onMount(() => setTitle(t('opt.title')));

  const tabs = $derived([
    { id: 'general', label: t('opt.tab.general') },
    { id: 'fileTypes', label: t('opt.tab.fileTypes') },
    { id: 'saveTo', label: t('opt.tab.saveTo') },
    { id: 'downloads', label: t('opt.tab.downloads') },
    { id: 'connection', label: t('opt.tab.connection') },
    { id: 'proxy', label: t('opt.tab.proxy') },
    { id: 'logins', label: t('opt.tab.logins') },
    { id: 'video', label: t('opt.tab.video') },
    { id: 'system', label: t('opt.tab.system') },
  ]);

  async function apply(): Promise<boolean> {
    saving = true;
    error = '';
    try {
      const saved = await saveSettings(s);
      s = structuredClone(saved);
      original = JSON.stringify(saved);
      return true;
    } catch (e) {
      error = String(e);
      return false;
    } finally {
      saving = false;
    }
  }

  async function ok() {
    if (await apply()) closeWindow();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && closeWindow()} />

<div class="window opts">
  <div class="tabwrap"><Tabs {tabs} bind:active={tab} /></div>
  <div class="tab-body grow">
    {#if tab === 'general'}<GeneralTab bind:s />
    {:else if tab === 'fileTypes'}<FileTypesTab bind:s />
    {:else if tab === 'saveTo'}<SaveToTab bind:s />
    {:else if tab === 'downloads'}<DownloadsTab bind:s />
    {:else if tab === 'connection'}<ConnectionTab bind:s />
    {:else if tab === 'proxy'}<ProxyTab bind:s />
    {:else if tab === 'logins'}<LoginsTab bind:s />
    {:else if tab === 'video'}<VideoTab bind:s />
    {:else if tab === 'system'}<SystemTab bind:s />
    {/if}
  </div>
  {#if error}<div class="notice error">{error}</div>{/if}
  <div class="buttons">
    <button class="primary" onclick={ok} disabled={saving}>{t('common.ok')}</button>
    <button onclick={closeWindow}>{t('common.cancel')}</button>
    <button onclick={apply} disabled={saving || !dirty}>{t('common.apply')}</button>
  </div>
</div>

<style>
  .opts {
    gap: 0;
  }
  .opts .buttons {
    margin-top: 8px;
  }
  .tabwrap :global(.tabs) {
    flex-wrap: wrap;
  }
  .tab-body {
    min-height: 0;
  }
</style>
