<script lang="ts">
  // Video kalite seçici: yt-dlp biçim listesinden çözünürlük / yalnız ses seçimi.
  import { onMount } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import ToolIcon from '../lib/icons/ToolIcon.svelte';
  import { api, closeWindow, EV, hashParams, on, pickFolder, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { formatDuration, formatError, formatSize, isValidUrl } from '../lib/format';
  import { app } from '../lib/state.svelte';
  import type { MediaFormat, MediaProbe, PendingMedia } from '../lib/types';
  import { emptyAddRequest } from '../lib/types';

  const params = hashParams();
  const token = params.get('token');
  let url = $state(params.get('url') ?? '');
  let probe = $state<MediaProbe | null>(null);
  let probing = $state(false);
  let error = $state<string | null>(null);

  let audioOnly = $state(false);
  let formatId = $state('');
  let fileName = $state('');
  let saveDir = $state('');
  let subs = $state<Set<string>>(new Set());
  let busy = $state(false);
  let nameEdited = false;

  const videoFormats = $derived((probe?.formats ?? []).filter((f) => !f.audioOnly).sort((a, b) => (b.height ?? 0) - (a.height ?? 0) || (b.fps ?? 0) - (a.fps ?? 0)));
  const audioFormats = $derived((probe?.formats ?? []).filter((f) => f.audioOnly).sort((a, b) => (b.tbr ?? 0) - (a.tbr ?? 0)));
  const list = $derived(audioOnly ? audioFormats : videoFormats);
  const current = $derived(list.find((f) => f.formatId === formatId) ?? null);

  function sanitize(s: string): string {
    return s.replace(/[\\/:*?"<>|\u0000-\u001f]+/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 180);
  }

  function extFor(f: MediaFormat | null): string {
    const m = app.settings?.media;
    if (audioOnly) return m?.audioFormat || f?.ext || 'm4a';
    return m?.container || f?.ext || 'mp4';
  }

  function pickDefault() {
    const q = app.settings?.media.defaultQuality ?? 'best';
    if (q === 'audio' && audioFormats.length) {
      audioOnly = true;
      formatId = audioFormats[0].formatId;
      return;
    }
    const target = parseInt(q, 10);
    const pick = isNaN(target) ? videoFormats[0] : videoFormats.find((f) => (f.height ?? 0) <= target) ?? videoFormats[videoFormats.length - 1];
    if (pick) formatId = pick.formatId;
    else if (audioFormats.length) {
      audioOnly = true;
      formatId = audioFormats[0].formatId;
    }
  }

  function setProbe(p: MediaProbe) {
    probe = p;
    pickDefault();
    if (!nameEdited) fileName = sanitize(p.title || 'video');
    const subLangs = app.settings?.media.downloadSubtitles ? app.settings.media.subtitleLangs : [];
    subs = new Set(p.subtitles.filter((s) => subLangs.includes(s)));
  }

  $effect(() => {
    // Kalite değişince varsayılan klasör kategoriye göre.
    if (!saveDir && app.settings) {
      const c = app.settings.categories.find((x) => x.id === (audioOnly ? 'music' : 'video'));
      saveDir = c?.saveDir || app.settings.defaultSaveDir;
    }
  });

  async function fetchProbe() {
    if (!isValidUrl(url)) {
      error = t('addurl.invalid');
      return;
    }
    error = null;
    probing = true;
    probe = null;
    try {
      setProbe(await api.mediaProbe(url.trim()));
    } catch (e) {
      error = formatError(String(e));
    } finally {
      probing = false;
    }
  }

  onMount(() => {
    setTitle(t('media.title'));
    let off: (() => void) | undefined;
    if (token) {
      probing = true;
      api.getPendingMedia(token).then((p) => {
        url = p.request.pageUrl;
        if (p.probe) setProbe(p.probe);
        probing = p.probing;
        error = p.error ? formatError(p.error) : null;
      });
      on<PendingMedia>(EV.pendingMedia, (p) => {
        if (p.token !== token) return;
        if (p.probe) setProbe(p.probe);
        probing = p.probing;
        error = p.error ? formatError(p.error) : null;
      }).then((u) => (off = u));
    } else if (url) {
      fetchProbe();
    }
    return () => off?.();
  });

  function setAudio(v: boolean) {
    audioOnly = v;
    formatId = (v ? audioFormats : videoFormats)[0]?.formatId ?? '';
    const c = app.settings?.categories.find((x) => x.id === (v ? 'music' : 'video'));
    if (c) saveDir = c.saveDir || app.settings!.defaultSaveDir;
  }

  function toggleSub(s: string) {
    const n = new Set(subs);
    if (n.has(s)) n.delete(s);
    else n.add(s);
    subs = n;
  }

  async function download() {
    if (!probe || !formatId || busy) return;
    busy = true;
    const full = `${sanitize(fileName) || 'video'}.${extFor(current)}`;
    try {
      if (token) {
        await api.confirmPendingMedia(token, { formatId, audioOnly, fileName: full, saveDir, subtitles: [...subs] });
      } else {
        const req = emptyAddRequest(probe.pageUrl || url);
        req.fileName = full;
        req.saveDir = saveDir;
        req.category = audioOnly ? 'music' : 'video';
        req.media = {
          pageUrl: probe.pageUrl || url,
          mediaUrl: null,
          formatId,
          audioOnly,
          audioFormat: audioOnly ? app.settings?.media.audioFormat ?? 'm4a' : null,
          container: audioOnly ? null : app.settings?.media.container ?? 'mp4',
          title: probe.title,
          thumbnail: probe.thumbnail,
          impersonate: false,
          subtitles: [...subs],
        };
        await api.addDownload(req);
      }
      closeWindow();
    } catch (e) {
      error = formatError(String(e));
      busy = false;
    }
  }

  async function cancel() {
    if (token) await api.cancelPending(token).catch(() => {});
    closeWindow();
  }

  function codec(f: MediaFormat): string {
    const v = f.vcodec?.split('.')[0];
    const a = f.acodec?.split('.')[0];
    return [v, a].filter(Boolean).join(' + ');
  }

  function qualityLabel(f: MediaFormat): string {
    if (f.audioOnly) return f.tbr ? `${Math.round(f.tbr)} kbps` : f.label;
    const fps = f.fps && f.fps > 30 ? Math.round(f.fps) : '';
    return `${f.height ?? '?'}p${fps}${f.hdr ? ' HDR' : ''}`;
  }

  function badge(f: MediaFormat): string {
    const h = f.height ?? 0;
    if (h >= 2160) return '4K';
    if (h >= 1440) return '2K';
    if (h >= 1080) return 'FHD';
    if (h >= 720) return 'HD';
    return '';
  }

</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && cancel()} />

<div class="window">
  {#if !token}
    <div class="row">
      <label for="m-url">{t('media.urlPrompt')}</label>
      <input id="m-url" type="text" bind:value={url} onkeydown={(e) => e.key === 'Enter' && fetchProbe()} placeholder="https://www.youtube.com/watch?v=…" />
      <button onclick={fetchProbe} disabled={probing}>{t('media.fetch')}</button>
    </div>
  {/if}

  <div class="head">
    <div class="thumb">
      {#if probe?.thumbnail}
        <img src={probe.thumbnail} alt="" referrerpolicy="no-referrer" />
      {:else}
        <ToolIcon name="video" size={56} />
      {/if}
      {#if probe?.duration}<span class="dur">{formatDuration(probe.duration)}</span>{/if}
      {#if probe?.isLive}<span class="live">{t('media.live')}</span>{/if}
    </div>
    <div class="meta">
      {#if probing}
        <div class="dim">{t('media.probing')}</div>
        <div class="spinner"></div>
      {:else if probe}
        <div class="title" title={probe.title}>{probe.title}</div>
        <div class="dim small">
          {probe.extractor}{probe.uploader ? ` • ${probe.uploader}` : ''}{probe.duration ? ` • ${formatDuration(probe.duration)}` : ''}
        </div>
        {#if probe.playlistCount}<div class="small">{t('media.playlist', { n: probe.playlistCount })}</div>{/if}
      {/if}
    </div>
  </div>

  {#if error}<div class="notice error"><Icon name="error" /><span class="selectable">{error}</span></div>{/if}

  {#if probe}
    <div class="row">
      <label class="check"><input type="radio" checked={!audioOnly} onchange={() => setAudio(false)} disabled={!videoFormats.length} />{t('media.video')}</label>
      <label class="check"><input type="radio" checked={audioOnly} onchange={() => setAudio(true)} disabled={!audioFormats.length} />{t('media.audioOnly')}</label>
    </div>
    <div class="table-wrap grow">
      <table class="grid">
        <colgroup><col style:width="118px" /><col style:width="80px" /><col /><col style:width="96px" /></colgroup>
        <thead><tr><th>{audioOnly ? t('media.colFormat') : t('media.colQuality')}</th><th>{t('media.colFormat')}</th><th>{t('media.colCodec')}</th><th class="num">{t('media.colSize')}</th></tr></thead>
        <tbody>
          {#each list as f (f.formatId)}
            <tr class:selected={formatId === f.formatId} onclick={() => (formatId = f.formatId)} ondblclick={download}>
              <td>
                <input type="radio" name="fmt" checked={formatId === f.formatId} onchange={() => (formatId = f.formatId)} />
                <b>{qualityLabel(f)}</b>
                {#if badge(f)}<span class="badge">{badge(f)}</span>{/if}
              </td>
              <td>{f.ext.toUpperCase()}</td>
              <td class="dim">{codec(f)}</td>
              <td class="num">{f.filesize ? '~' + formatSize(f.filesize) : ''}</td>
            </tr>
          {/each}
          {#if !list.length}<tr><td colspan="4" class="dim">{t('media.noFormats')}</td></tr>{/if}
        </tbody>
      </table>
    </div>

    {#if probe.subtitles.length}
      <div class="row subs">
        <span class="dim">{t('media.subtitles')}:</span>
        {#each probe.subtitles.slice(0, 14) as s (s)}
          <label class="check"><input type="checkbox" checked={subs.has(s)} onchange={() => toggleSub(s)} />{s}</label>
        {/each}
      </div>
    {/if}

    <div class="form">
      <label for="m-name">{t('media.fileName')}</label>
      <div class="row">
        <input id="m-name" type="text" bind:value={fileName} oninput={() => (nameEdited = true)} />
        <span class="dim">.{extFor(current)}</span>
      </div>
      <label for="m-dir">{t('media.saveTo')}</label>
      <div class="row">
        <input id="m-dir" type="text" bind:value={saveDir} />
        <button class="small" onclick={async () => { const r = await pickFolder(saveDir); if (r) saveDir = r; }}>{t('common.browse')}</button>
      </div>
    </div>
  {:else}
    <div class="grow"></div>
  {/if}

  <div class="buttons">
    <button class="primary" disabled={!probe || !formatId || busy} onclick={download}>{t('media.download')}</button>
    <button onclick={cancel}>{t('common.cancel')}</button>
  </div>
</div>

<style>
  .head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .thumb {
    position: relative;
    width: 160px;
    height: 90px;
    flex: none;
    background: #111;
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .dur,
  .live {
    position: absolute;
    right: 4px;
    bottom: 4px;
    background: rgba(0, 0, 0, 0.8);
    color: #fff;
    font-size: 11px;
    padding: 0 4px;
    border-radius: 2px;
  }
  .live {
    left: 4px;
    right: auto;
    background: #d0201a;
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .badge {
    display: inline-block;
    font-size: 9px;
    font-weight: 700;
    padding: 0 3px;
    margin-left: 4px;
    border-radius: 2px;
    background: #1d6fd0;
    color: #fff;
    vertical-align: 1px;
  }
  .subs {
    flex-wrap: wrap;
    gap: 2px 10px;
  }
  .form {
    grid-template-columns: 82px 1fr;
  }
  .spinner {
    width: 22px;
    height: 22px;
    border: 3px solid var(--border-light);
    border-top-color: #2a83e0;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  td input[type='radio'] {
    margin-right: 4px;
  }
</style>
