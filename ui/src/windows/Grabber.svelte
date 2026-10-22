<script lang="ts">
  // Site yakalayıcı: bir sayfadan (ve bağlantılı sayfalardan) dosyaları toplar.
  import { onMount } from 'svelte';
  import FileIcon from '../lib/icons/FileIcon.svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { api, closeWindow, hashParams, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { isValidUrl } from '../lib/format';
  import { app, categoryName } from '../lib/state.svelte';
  import { emptyAddRequest, type GrabbedLink } from '../lib/types';

  const PRESETS: Record<string, string> = {
    'grabber.presetImages': 'jpg jpeg png gif webp bmp svg',
    'grabber.presetVideo': 'mp4 mkv webm avi mov flv m4v',
    'grabber.presetAudio': 'mp3 m4a flac wav ogg opus aac',
    'grabber.presetArchives': 'zip rar 7z tar gz xz bz2 iso',
    'grabber.presetDocs': 'pdf doc docx xls xlsx ppt pptx epub txt',
  };

  let url = $state(hashParams().get('url') ?? 'https://');
  let depth = $state(1);
  let exts = $state('');
  let scanning = $state(false);
  let links = $state<GrabbedLink[]>([]);
  let checked = $state<Set<string>>(new Set());
  let filterText = $state('');
  let error = $state('');
  let category = $state('');

  onMount(() => setTitle(t('grabber.title')));

  async function scan() {
    if (!isValidUrl(url)) {
      error = t('addurl.invalid');
      return;
    }
    error = '';
    scanning = true;
    links = [];
    try {
      const list = exts.split(/[\s,;]+/).map((x) => x.replace(/^\./, '').toLowerCase()).filter(Boolean);
      links = await api.grabLinks(url.trim(), depth, list);
      checked = new Set(links.map((l) => l.url));
    } catch (e) {
      error = String(e);
    } finally {
      scanning = false;
    }
  }

  const shown = $derived(
    links.filter((l) => !filterText || l.url.toLowerCase().includes(filterText.toLowerCase()) || l.text.toLowerCase().includes(filterText.toLowerCase())),
  );

  function toggle(u: string) {
    const n = new Set(checked);
    if (n.has(u)) n.delete(u);
    else n.add(u);
    checked = n;
  }

  async function add() {
    const sel = links.filter((l) => checked.has(l.url));
    if (!sel.length) return;
    await api.addMany(
      sel.map((l) => {
        const r = emptyAddRequest(l.url);
        r.request.referer = url;
        r.start = false;
        r.category = category || l.category;
        r.description = l.text || null;
        return r;
      }),
    );
    closeWindow();
  }
</script>

<div class="window">
  <div class="form">
    <label for="g-url">{t('grabber.url')}</label>
    <div class="row">
      <input id="g-url" type="text" bind:value={url} onkeydown={(e) => e.key === 'Enter' && scan()} />
      <button class="primary" onclick={scan} disabled={scanning}>{scanning ? t('grabber.scanning') : t('grabber.start')}</button>
    </div>
    <label for="g-ext">{t('grabber.extensions')}</label>
    <div class="row">
      <input id="g-ext" type="text" bind:value={exts} class="mono" placeholder="jpg png pdf" />
      <label for="g-depth" class="dim">{t('grabber.depth')}</label>
      <select id="g-depth" bind:value={depth} style:flex="0 0 54px">
        {#each [0, 1, 2, 3, 4, 5] as n (n)}<option value={n}>{n}</option>{/each}
      </select>
    </div>
    <span class="dim">{t('grabber.presets')}</span>
    <div class="row presets">
      {#each Object.entries(PRESETS) as [k, v] (k)}
        <button class="small" onclick={() => (exts = v)}>{t(k)}</button>
      {/each}
    </div>
  </div>
  {#if error}<div class="notice error"><Icon name="error" />{error}</div>{/if}
  <div class="row">
    <b>{t('grabber.found', { n: links.length })}</b>
    <span class="grow"></span>
    <input type="text" bind:value={filterText} placeholder={t('grabber.filter')} style:flex="0 0 200px" />
    <button class="small" onclick={() => (checked = new Set(shown.map((l) => l.url)))}>{t('common.selectAll')}</button>
    <button class="small" onclick={() => (checked = new Set())}>{t('common.selectNone')}</button>
  </div>
  <div class="table-wrap grow">
    <table class="grid">
      <colgroup><col style:width="28px" /><col style:width="220px" /><col style:width="160px" /><col /></colgroup>
      <thead><tr><th></th><th>{t('grabber.colFile')}</th><th>{t('grabber.colText')}</th><th>{t('grabber.colUrl')}</th></tr></thead>
      <tbody>
        {#each shown as l (l.url)}
          <tr class:selected={checked.has(l.url)} onclick={() => toggle(l.url)}>
            <td><input type="checkbox" checked={checked.has(l.url)} onclick={(e) => e.stopPropagation()} onchange={() => toggle(l.url)} /></td>
            <td class="fname"><FileIcon name={l.fileName} /> {l.fileName}</td>
            <td title={l.text}>{l.text}</td>
            <td class="dim" title={l.url}>{l.url}</td>
          </tr>
        {/each}
        {#if scanning}
          <tr><td colspan="4" class="dim" style="text-align:center; height: 40px">{t('grabber.scanning')}</td></tr>
        {/if}
      </tbody>
    </table>
  </div>
  <div class="buttons">
    <div class="left">
      <label for="g-cat">{t('fileinfo.category')}</label>
      <select id="g-cat" bind:value={category}>
        <option value="">{t('common.default')}</option>
        {#each app.settings?.categories ?? [] as c (c.id)}<option value={c.id}>{categoryName(c)}</option>{/each}
      </select>
    </div>
    <button class="primary" disabled={!checked.size} onclick={add}>{t('grabber.addSelected', { n: checked.size })}</button>
    <button onclick={closeWindow}>{t('common.close')}</button>
  </div>
</div>

<style>
  .form {
    grid-template-columns: 110px 1fr;
  }
  .presets {
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
  }
  .fname {
    display: flex;
    align-items: center;
    gap: 5px;
  }
</style>
