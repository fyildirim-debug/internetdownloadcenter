<script lang="ts">
  // Hakkında penceresi.
  import { onMount } from 'svelte';
  import ToolIcon from '../lib/icons/ToolIcon.svelte';
  import { api, closeWindow, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import type { AppInfo } from '../lib/types';

  let info = $state<AppInfo | null>(null);
  onMount(async () => {
    setTitle(t('about.title'));
    info = await api.appInfo();
  });
</script>

<svelte:window onkeydown={(e) => (e.key === 'Escape' || e.key === 'Enter') && closeWindow()} />

<div class="window about">
  <div class="banner">
    <ToolIcon name="app" size={64} />
    <div>
      <div class="name">{t('app.name')}</div>
      <div class="ver">{t('about.version', { v: info?.version ?? '' })} — {info?.os ?? ''}</div>
    </div>
  </div>
  <p>{t('about.desc')}</p>
  <p class="dim">{t('about.author')}<br />{t('about.license')}<br />{t('about.credits')}</p>
  {#if info}<p class="small dim selectable">{info.dataDir}</p>{/if}
  <div class="grow"></div>
  <div class="buttons">
    <div class="left"><a href="https://github.com/fyildirim-debug/internetdownloadcenter" target="_blank" rel="noreferrer">github.com/fyildirim-debug/internetdownloadcenter</a></div>
    <button class="primary" onclick={closeWindow}>{t('common.ok')}</button>
  </div>
</div>

<style>
  .about {
    padding: 0 0 10px;
  }
  .about > :not(.banner) {
    margin-left: 16px;
    margin-right: 16px;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    background: linear-gradient(135deg, #0b4f9c, #2a83e0 60%, #5fb0ff);
    color: #fff;
  }
  .name {
    font-size: 18px;
    font-weight: 600;
  }
  .ver {
    opacity: 0.9;
    margin-top: 2px;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
</style>
