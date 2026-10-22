<script lang="ts">
  // "İndirme tamamlandı" penceresi.
  import { onMount } from 'svelte';
  import FileIcon from '../lib/icons/FileIcon.svelte';
  import { api, closeWindow, hashParams, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { formatEta, formatSize, formatSpeed, joinPath } from '../lib/format';
  import { app, playDoneSound, saveSettings } from '../lib/state.svelte';
  import type { DownloadDetail, Settings } from '../lib/types';

  const id = hashParams().get('id') ?? '';
  let d = $state<DownloadDetail | null>(null);
  let dontShow = $state(false);

  onMount(async () => {
    setTitle(t('complete.title'));
    try {
      d = await api.getDownload(id);
    } catch {
      /* silinmiş */
    }
    if (app.settings?.general.playSounds) playDoneSound();
  });

  const elapsed = $derived(d?.completedAt && d.lastTryAt ? Math.max(1, (d.completedAt - d.lastTryAt) / 1000) : null);

  async function close() {
    if (dontShow && app.settings?.downloads.showCompleteDialog) {
      const s = $state.snapshot(app.settings) as Settings;
      s.downloads.showCompleteDialog = false;
      await saveSettings(s);
    }
    closeWindow();
  }

  async function act(fn: (id: string) => Promise<void>) {
    await fn(id);
    close();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<div class="window cw">
  <div class="head">
    <FileIcon name={d?.fileName ?? ''} size={48} media={d?.kind === 'media'} />
    <div class="grow">
      <div class="title">{t('complete.heading')}</div>
      <div class="fname ellipsis" title={d?.fileName}>{d?.fileName ?? ''}</div>
    </div>
  </div>
  {#if d}
    <div class="info">
      <span>{t('complete.address')}</span><span class="ellipsis selectable link" title={d.url}>{d.url}</span>
      <span>{t('complete.savedAs')}</span><span class="ellipsis selectable" title={joinPath(d.saveDir, d.fileName)}>{joinPath(d.saveDir, d.fileName)}</span>
      <span>{t('complete.size')}</span><span>{formatSize(d.size ?? d.downloaded)}</span>
      {#if elapsed}
        <span>{t('complete.time')}</span><span>{formatEta(elapsed)} — {t('complete.avgSpeed')}: {formatSpeed((d.size ?? d.downloaded) / elapsed)}</span>
      {/if}
      {#if d.checksumOk != null}
        <span></span><span class:ok={d.checksumOk} class:bad={!d.checksumOk}>{d.checksumOk ? t('complete.checksumOk') : t('complete.checksumBad')}</span>
      {/if}
    </div>
  {/if}
  <div class="grow"></div>
  <label class="check"><input type="checkbox" bind:checked={dontShow} />{t('complete.dontShow')}</label>
  <div class="buttons">
    <button class="primary" onclick={() => act(api.openFile)}>{t('menu.open')}</button>
    <button onclick={() => act(api.openWith)}>{t('menu.openWith')}</button>
    <button onclick={() => act(api.openFolder)}>{t('menu.openFolder')}</button>
    <button onclick={close}>{t('common.close')}</button>
  </div>
</div>

<style>
  .head {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--ok);
  }
  .fname {
    font-weight: 600;
    margin-top: 2px;
  }
  .info {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 4px 8px;
    background: var(--panel);
    border: 1px solid var(--border-light);
    padding: 8px 10px;
  }
  .info > span:nth-child(odd) {
    color: var(--text-dim);
  }
  .link {
    color: var(--link);
  }
  .ok {
    color: var(--ok);
    font-weight: 600;
  }
  .bad {
    color: var(--err);
    font-weight: 600;
  }
</style>
