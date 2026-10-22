<script lang="ts">
  // "İndirme Dosya Bilgisi" penceresi: yakalanan ya da elle eklenen indirmeyi onaylatır.
  import { onMount } from 'svelte';
  import FileIcon from '../lib/icons/FileIcon.svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { api, closeWindow, EV, hashParams, on, pickFolder, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { formatSize, joinPath } from '../lib/format';
  import { app, categoryForName, categoryName, queueName, saveSettings } from '../lib/state.svelte';
  import type { AddRequest, PendingAdd, Settings } from '../lib/types';
  import { MAX_CONNECTIONS } from '../lib/types';

  const token = hashParams().get('token') ?? '';
  let p = $state<PendingAdd | null>(null);
  let error = $state<string | null>(null);

  let url = $state('');
  let category = $state('general');
  let dir = $state('');
  let name = $state('');
  let description = $state('');
  let remember = $state(false);
  let conns = $state(8);
  let queueId = $state('');
  let checksum = $state('');
  let userEditedName = false;
  let userEditedDir = false;
  let busy = $state(false);

  function fill(x: PendingAdd) {
    p = x;
    url = x.request.request.url;
    if (!userEditedName) name = x.probe?.fileName || x.suggestedName;
    if (!userEditedDir) {
      category = x.probe?.category || x.category || categoryForName(name);
      dir = x.suggestedDir || dirFor(category);
    }
    description ||= x.request.description ?? '';
    conns = x.request.maxConnections ?? app.settings?.downloads.defaultConnections ?? 8;
  }

  function dirFor(cat: string): string {
    const s = app.settings;
    if (!s) return '';
    const c = s.categories.find((x) => x.id === cat);
    return c?.saveDir || s.defaultSaveDir;
  }

  onMount(() => {
    setTitle(t('fileinfo.title'));
    let off: (() => void) | undefined;
    api
      .getPending(token)
      .then(fill)
      .catch((e) => (error = String(e)));
    on<PendingAdd>(EV.pending, (x) => x.token === token && fill(x)).then((u) => (off = u));
    return () => off?.();
  });

  function onCategory() {
    if (!userEditedDir || remember) dir = dirFor(category);
  }

  async function browse() {
    const r = await pickFolder(dir);
    if (r) {
      dir = r;
      userEditedDir = true;
    }
  }

  async function addCategory() {
    const n = window.prompt(t('fileinfo.newCategoryName'));
    if (!n || !app.settings) return;
    const s = $state.snapshot(app.settings) as Settings;
    const id = 'c' + Date.now().toString(36);
    const ext = name.includes('.') ? [name.split('.').pop()!.toLowerCase()] : [];
    s.categories.push({ id, name: n, extensions: ext, saveDir: joinPath(s.defaultSaveDir, n), builtin: false });
    await saveSettings(s);
    category = id;
    dir = dirFor(id);
  }

  function buildReq(start: boolean): AddRequest {
    const base = $state.snapshot(p!.request) as AddRequest;
    base.request.url = url.trim();
    base.fileName = name.trim() || null;
    base.saveDir = dir.trim() || null;
    base.category = category;
    base.description = description.trim() || null;
    base.maxConnections = conns;
    base.start = start;
    base.queueId = queueId || null;
    const cs = checksum.trim();
    if (cs) {
      const algo = cs.length === 32 ? 'md5' : cs.length === 40 ? 'sha1' : 'sha256';
      base.checksum = { algo, expected: cs.toLowerCase() };
    }
    return base;
  }

  async function confirm(start: boolean) {
    if (!p || busy) return;
    busy = true;
    try {
      if (remember && app.settings) {
        const s = $state.snapshot(app.settings) as Settings;
        const c = s.categories.find((x) => x.id === category);
        if (c && c.saveDir !== dir) {
          c.saveDir = dir;
          await saveSettings(s);
        }
      }
      await api.confirmPending(token, buildReq(start));
      closeWindow();
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }

  async function cancel() {
    await api.cancelPending(token).catch(() => {});
    closeWindow();
  }

  async function asVideo() {
    await api.openWindow('media', { url });
    cancel();
  }

  const catLabel = $derived(categoryName(app.settings?.categories.find((c) => c.id === category)));
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') cancel(); if (e.key === 'Enter' && !(e.target as HTMLElement).matches('textarea')) confirm(true); }} />

<div class="window fi">
  {#if !p && !error}
    <p class="dim">{t('common.loading')}</p>
  {:else}
    <div class="top">
      <div class="form grow">
        <label for="fi-url">{t('fileinfo.url')}</label>
        <input id="fi-url" type="text" bind:value={url} />

        <label for="fi-cat">{t('fileinfo.category')}</label>
        <div class="row">
          <select id="fi-cat" bind:value={category} onchange={onCategory}>
            {#each app.settings?.categories ?? [] as c (c.id)}<option value={c.id}>{categoryName(c)}</option>{/each}
          </select>
          <button class="small" title={t('fileinfo.addCategory')} onclick={addCategory}><Icon name="plus" size={12} /></button>
        </div>

        <label for="fi-dir">{t('fileinfo.saveAs')}</label>
        <div class="row">
          <input id="fi-dir" type="text" bind:value={dir} oninput={() => (userEditedDir = true)} />
          <button class="small" onclick={browse} title={t('common.browseLong')}>{t('common.browse')}</button>
        </div>

        <span></span>
        <div class="row">
          <input class="name" type="text" bind:value={name} oninput={() => (userEditedName = true)} aria-label={t('props.fileName')} />
        </div>

        <span></span>
        <label class="check"><input type="checkbox" bind:checked={remember} />{t('fileinfo.remember', { c: catLabel })}</label>

        <label for="fi-desc">{t('fileinfo.description')}</label>
        <input id="fi-desc" type="text" bind:value={description} />
      </div>
      <div class="side">
        <FileIcon name={name} size={48} />
        <div class="size">
          {#if p?.probing}
            <span class="dim">{t('fileinfo.probing')}</span>
          {:else if p?.probe?.size != null}
            <b>{formatSize(p.probe.size)}</b>
          {:else}
            <span class="dim">{t('common.unknown')}</span>
          {/if}
        </div>
        {#if p?.probe && !p.probing}
          <div class="small" class:ok={p.probe.resumable} class:bad={!p.probe.resumable}>
            {t('fileinfo.resumable')}: {p.probe.resumable ? t('common.yesShort') : t('common.noShort')}
          </div>
        {/if}
      </div>
    </div>

    <div class="row opts">
      <label for="fi-conn">{t('fileinfo.connections')}</label>
      <select id="fi-conn" bind:value={conns} style:flex="0 0 60px">
        {#each Array.from({ length: MAX_CONNECTIONS }, (_, i) => i + 1) as n (n)}<option value={n}>{n}</option>{/each}
      </select>
      <label for="fi-q">{t('fileinfo.queue')}</label>
      <select id="fi-q" bind:value={queueId} style:flex="0 0 140px">
        <option value="">{t('queue.none')}</option>
        {#each app.settings?.queues ?? [] as q (q.id)}<option value={q.id}>{queueName(q)}</option>{/each}
      </select>
      <input type="text" bind:value={checksum} placeholder={t('fileinfo.checksum')} class="mono small" />
    </div>

    {#if error}
      <div class="notice error"><Icon name="error" />{error}</div>
    {:else if p?.duplicateOf}
      <div class="notice warn"><Icon name="warning" />{t('fileinfo.duplicate')}</div>
    {:else if p?.probe?.isMediaSite}
      <div class="notice"><Icon name="info" /><span>{t('fileinfo.mediaSite')} <a href={'#'} onclick={(e) => { e.preventDefault(); asVideo(); }}>{t('fileinfo.asVideo')}</a></span></div>
    {:else if p?.probe?.isHtml}
      <div class="notice warn"><Icon name="warning" />{t('fileinfo.html')}</div>
    {:else if p?.error}
      <div class="notice warn"><Icon name="warning" />{p.error}</div>
    {/if}

    <div class="buttons">
      <button onclick={() => confirm(false)} disabled={busy}>{t('fileinfo.later')}</button>
      <button class="primary" onclick={() => confirm(true)} disabled={busy}>{t('fileinfo.start')}</button>
      <button onclick={cancel}>{t('common.cancel')}</button>
    </div>
  {/if}
</div>

<style>
  .fi {
    gap: 8px;
  }
  .top {
    display: flex;
    gap: 12px;
  }
  .form {
    grid-template-columns: 82px 1fr;
  }
  .side {
    width: 92px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding-top: 30px;
    text-align: center;
  }
  .size b {
    font-size: 13px;
  }
  .ok {
    color: var(--ok);
  }
  .bad {
    color: var(--err);
  }
  .name {
    font-weight: 600;
  }
  .opts {
    gap: 6px;
  }
  .opts input {
    flex: 1;
  }
  .notice {
    font-size: 11.5px;
  }
</style>
