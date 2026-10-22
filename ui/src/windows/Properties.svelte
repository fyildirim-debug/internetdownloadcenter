<script lang="ts">
  // Dosya özellikleri: ad, klasör, adres (yenileme), açıklama, referer, sağlama.
  import { onMount } from 'svelte';
  import FileIcon from '../lib/icons/FileIcon.svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { api, closeWindow, hashParams, pickFolder, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { formatDate, formatError, formatPercent, formatSize, statusText } from '../lib/format';
  import { app, categoryName } from '../lib/state.svelte';
  import type { DownloadDetail, DownloadPatch } from '../lib/types';

  const id = hashParams().get('id') ?? '';
  let d = $state<DownloadDetail | null>(null);
  let fileName = $state('');
  let saveDir = $state('');
  let url = $state('');
  let referer = $state('');
  let description = $state('');
  let category = $state('general');
  let algo = $state('sha256');
  let expected = $state('');
  let saving = $state(false);
  let err = $state('');

  onMount(async () => {
    try {
      d = await api.getDownload(id);
    } catch (e) {
      err = String(e);
      return;
    }
    setTitle(`${t('props.title')} — ${d.fileName}`);
    fileName = d.fileName;
    saveDir = d.saveDir;
    url = d.url;
    referer = d.referer ?? '';
    description = d.description;
    category = d.category;
    if (d.checksum) {
      algo = d.checksum.algo;
      expected = d.checksum.expected;
    }
  });

  async function save() {
    if (!d) return;
    saving = true;
    const patch: DownloadPatch = {};
    if (fileName !== d.fileName) patch.fileName = fileName;
    if (saveDir !== d.saveDir) patch.saveDir = saveDir;
    if (url !== d.url) patch.url = url;
    if (referer !== (d.referer ?? '')) patch.referer = referer;
    if (description !== d.description) patch.description = description;
    if (category !== d.category) patch.category = category;
    const cs = expected.trim();
    if (cs && (cs !== d.checksum?.expected || algo !== d.checksum?.algo)) patch.checksum = { algo, expected: cs.toLowerCase() };
    try {
      if (Object.keys(patch).length) await api.updateDownload(id, patch);
      closeWindow();
    } catch (e) {
      err = formatError(String(e));
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && closeWindow()} />

<div class="window">
  {#if d}
    <div class="head">
      <FileIcon name={d.fileName} size={48} media={d.kind === 'media'} />
      <div class="hinfo">
        <div class="fname ellipsis">{d.fileName}</div>
        <div class="dim">{d.status === 'error' ? t('status.error') : statusText(d.status, d.downloaded, d.size, d.error)} — {formatSize(d.downloaded)} / {d.size != null ? formatSize(d.size) : t('common.unknown')} {formatPercent(d.downloaded, d.size)}</div>
      </div>
    </div>
    <div class="form">
      <label for="p-name">{t('props.fileName')}</label>
      <input id="p-name" type="text" bind:value={fileName} disabled={d.status === 'downloading'} />
      <label for="p-dir">{t('props.saveTo')}</label>
      <div class="row">
        <input id="p-dir" type="text" bind:value={saveDir} disabled={d.status === 'downloading'} />
        <button class="small" disabled={d.status === 'downloading'} onclick={async () => { const r = await pickFolder(saveDir); if (r) saveDir = r; }}>{t('common.browse')}</button>
      </div>
      <label for="p-cat">{t('fileinfo.category')}</label>
      <select id="p-cat" bind:value={category}>
        {#each app.settings?.categories ?? [] as c (c.id)}<option value={c.id}>{categoryName(c)}</option>{/each}
      </select>
      <label for="p-url">{t('props.address')}</label>
      <input id="p-url" type="text" bind:value={url} class="mono small" />
      <span></span><span class="dim small">{t('props.addressHint')}</span>
      <label for="p-ref">{t('props.referer')}</label>
      <input id="p-ref" type="text" bind:value={referer} class="mono small" />
      <label for="p-desc">{t('props.description')}</label>
      <input id="p-desc" type="text" bind:value={description} />
    </div>
    <fieldset>
      <legend>{t('props.checksum')}</legend>
      <div class="row">
        <select bind:value={algo} style:flex="0 0 90px">
          <option value="md5">MD5</option>
          <option value="sha1">SHA-1</option>
          <option value="sha256">SHA-256</option>
        </select>
        <input type="text" bind:value={expected} class="mono small" placeholder={t('props.expected')} />
        {#if d.checksumOk != null}
          <Icon name={d.checksumOk ? 'check' : 'error'} />
        {/if}
      </div>
    </fieldset>
    <div class="facts small">
      <span class="dim">{t('props.added')}</span><span>{formatDate(d.addedAt)}</span>
      <span class="dim">{t('props.lastTry')}</span><span>{formatDate(d.lastTryAt)}</span>
      {#if d.finalUrl && d.finalUrl !== d.url}<span class="dim">URL →</span><span class="ellipsis selectable" title={d.finalUrl}>{d.finalUrl}</span>{/if}
      {#if d.media}<span class="dim">{t('props.media')}</span><span>{d.media.formatId}{d.media.audioOnly ? ' (audio)' : ''}</span>{/if}
      {#if d.error}<span class="dim">{t('props.error')}</span><span class="err">{formatError(d.error)}</span>{/if}
    </div>
  {:else if err}
    <div class="notice error"><Icon name="error" />{err}</div>
  {/if}
  {#if err && d}<div class="notice error"><Icon name="error" />{err}</div>{/if}
  <div class="grow"></div>
  <div class="buttons">
    {#if d?.status === 'completed'}
      <div class="left">
        <button onclick={() => api.openFile(id)}>{t('menu.open')}</button>
        <button onclick={() => api.openFolder(id)}>{t('menu.openFolder')}</button>
      </div>
    {/if}
    <button class="primary" onclick={save} disabled={!d || saving}>{t('common.ok')}</button>
    <button onclick={closeWindow}>{t('common.cancel')}</button>
  </div>
</div>

<style>
  .head {
    display: flex;
    gap: 10px;
    align-items: center;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--border-light);
  }
  .hinfo {
    min-width: 0;
    flex: 1;
  }
  .fname {
    font-weight: 600;
    font-size: 13px;
  }
  .form {
    grid-template-columns: 90px 1fr;
  }
  .facts {
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: 2px 8px;
  }
  .err {
    color: var(--err);
  }
</style>
