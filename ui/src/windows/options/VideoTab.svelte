<script lang="ts">
  // Seçenekler > Video: kalite, kap, araçlar (yt-dlp, FFmpeg, Deno), engel aşma.
  import { onMount } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { api, EV, on } from '../../lib/api';
  import { t } from '../../lib/i18n/index.svelte';
  import type { Settings, ToolsStatus, ToolInfo } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
  let tools = $state<ToolsStatus | null>(null);

  onMount(() => {
    let off: (() => void) | undefined;
    api.toolsStatus().then((x) => (tools = x));
    on<ToolsStatus>(EV.tools, (x) => (tools = x)).then((u) => (off = u));
    return () => off?.();
  });

  async function update() {
    if (tools) tools.busy = true;
    try {
      tools = await api.toolsUpdate();
    } catch (e) {
      if (tools) {
        tools.busy = false;
        tools.message = String(e);
      }
    }
  }

  const rows = $derived<[string, ToolInfo][]>(tools ? [['yt-dlp', tools.ytdlp], ['FFmpeg', tools.ffmpeg], ['Deno', tools.deno]] : []);
</script>

<div class="grid2">
  <label for="v-q">{t('opt.vd.quality')}</label>
  <select id="v-q" bind:value={s.media.defaultQuality}>
    <option value="best">{t('opt.vd.best')}</option>
    {#each ['2160', '1440', '1080', '720', '480', '360'] as q (q)}<option value={q}>{q}p</option>{/each}
    <option value="audio">{t('opt.vd.audio')}</option>
  </select>
  <label for="v-c">{t('opt.vd.container')}</label>
  <select id="v-c" bind:value={s.media.container}>
    <option value="mp4">MP4</option><option value="mkv">MKV</option><option value="webm">WEBM</option>
  </select>
  <label for="v-a">{t('opt.vd.audioFormat')}</label>
  <select id="v-a" bind:value={s.media.audioFormat}>
    <option value="m4a">M4A (AAC)</option><option value="mp3">MP3</option><option value="opus">OPUS</option>
  </select>
</div>
<label class="check"><input type="checkbox" bind:checked={s.media.downloadSubtitles} />{t('opt.vd.subtitles')}</label>
<div class="row ind">
  <label for="v-sl">{t('opt.vd.subLangs')}</label>
  <input id="v-sl" type="text" value={s.media.subtitleLangs.join(', ')} disabled={!s.media.downloadSubtitles} oninput={(e) => (s.media.subtitleLangs = (e.target as HTMLInputElement).value.split(/[\s,;]+/).filter(Boolean))} style:flex="0 0 160px" />
</div>
<label class="check"><input type="checkbox" bind:checked={s.media.embedThumbnail} />{t('opt.vd.thumbnail')}</label>
<label class="check"><input type="checkbox" bind:checked={s.media.sponsorblock} />{t('opt.vd.sponsorblock')}</label>
<label class="check"><input type="checkbox" bind:checked={s.media.impersonateFallback} />{t('opt.vd.impersonate')}</label>
<label class="check"><input type="checkbox" bind:checked={s.media.browserFallback} />{t('opt.vd.browserFallback')}</label>

<fieldset class="mt">
  <legend>{t('opt.vd.tools')}</legend>
  <div class="table-wrap">
    <table class="grid">
      <colgroup><col style:width="80px" /><col style:width="110px" /><col style:width="70px" /><col /></colgroup>
      <thead><tr><th>{t('opt.vd.tool')}</th><th>{t('opt.vd.version')}</th><th>{t('opt.vd.source')}</th><th></th></tr></thead>
      <tbody>
        {#each rows as [name, info] (name)}
          <tr>
            <td><Icon name={info.installed ? 'check' : 'warning'} size={12} /> {name}</td>
            <td>{info.version ?? '—'}</td>
            <td>{t(`opt.vd.src.${info.source}`)}</td>
            <td class="dim small" title={info.path ?? ''}>{info.path ?? ''}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="row mt">
    <button onclick={update} disabled={!tools || tools.busy}><Icon name="refresh" size={12} />{t('opt.vd.installTools')}</button>
    {#if tools?.busy}<span class="spinner"></span>{/if}
    <span class="dim small">{tools?.message ?? ''}</span>
  </div>
  <label class="check mt"><input type="checkbox" bind:checked={s.media.autoUpdateTools} />{t('opt.vd.autoUpdate')}</label>
</fieldset>

<style>
  .grid2 {
    display: grid;
    grid-template-columns: 240px 150px;
    gap: 5px 8px;
    align-items: center;
    margin-bottom: 6px;
  }
  .ind {
    padding-left: 20px;
  }
  .mt {
    margin-top: 6px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-light);
    border-top-color: #2a83e0;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
