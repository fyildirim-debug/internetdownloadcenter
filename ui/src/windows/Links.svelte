<script lang="ts">
  // Eklentiden gelen "tüm bağlantıları indir" listesi; kullanıcı seçer.
  import { onMount } from 'svelte';
  import FileIcon from '../lib/icons/FileIcon.svelte';
  import { api, closeWindow, hashParams, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { extOf } from '../lib/format';
  import { app, categoryForName, categoryName } from '../lib/state.svelte';
  import { emptyAddRequest, type GrabbedLink } from '../lib/types';

  const token = hashParams().get('token') ?? '';
  let links = $state<GrabbedLink[]>([]);
  let referrer = $state<string | null>(null);
  let checked = $state<Set<string>>(new Set());
  let filterText = $state('');
  let extFilter = $state('');
  let start = $state(false);

  onMount(async () => {
    setTitle(t('links.title'));
    const p = await api.getPendingLinks(token);
    links = p.links;
    referrer = p.referrer;
    const caps = new Set((app.settings?.fileTypes.captureExtensions ?? []).map((x) => x.toLowerCase()));
    checked = new Set(links.filter((l) => caps.has(extOf(l.fileName || l.url))).map((l) => l.url));
  });

  const exts = $derived([...new Set(links.map((l) => extOf(l.fileName || l.url)).filter(Boolean))].sort());
  const shown = $derived(
    links.filter(
      (l) =>
        (!extFilter || extOf(l.fileName || l.url) === extFilter) &&
        (!filterText || (l.url + ' ' + l.text).toLowerCase().includes(filterText.toLowerCase())),
    ),
  );

  function toggle(u: string) {
    const n = new Set(checked);
    if (n.has(u)) n.delete(u);
    else n.add(u);
    checked = n;
  }

  async function add() {
    const sel = links.filter((l) => checked.has(l.url));
    await api.addMany(
      sel.map((l) => {
        const r = emptyAddRequest(l.url);
        r.request.referer = referrer;
        r.start = start;
        r.category = categoryForName(l.fileName || l.url);
        r.description = l.text || null;
        return r;
      }),
    );
    await api.cancelPending(token).catch(() => {});
    closeWindow();
  }
</script>

<div class="window">
  <div class="row">
    <b>{t('links.heading')}</b>
    <span class="dim">({links.length})</span>
    <span class="sp"></span>
    <select bind:value={extFilter} style:flex="0 0 110px">
      <option value="">*.*</option>
      {#each exts as e (e)}<option value={e}>*.{e}</option>{/each}
    </select>
    <input type="text" bind:value={filterText} placeholder={t('links.filter')} style:flex="0 0 180px" />
    <button class="small" onclick={() => (checked = new Set([...checked, ...shown.map((l) => l.url)]))}>{t('common.selectAll')}</button>
    <button class="small" onclick={() => (checked = new Set())}>{t('common.selectNone')}</button>
  </div>
  <div class="table-wrap grow">
    <table class="grid">
      <colgroup><col style:width="28px" /><col style:width="230px" /><col style:width="110px" /><col /></colgroup>
      <thead><tr><th></th><th>{t('grabber.colFile')}</th><th>{t('fileinfo.category')}</th><th>{t('grabber.colUrl')}</th></tr></thead>
      <tbody>
        {#each shown as l (l.url)}
          <tr class:selected={checked.has(l.url)} onclick={() => toggle(l.url)}>
            <td><input type="checkbox" checked={checked.has(l.url)} onclick={(e) => e.stopPropagation()} onchange={() => toggle(l.url)} /></td>
            <td class="fname" title={l.text}><FileIcon name={l.fileName} /> {l.fileName}</td>
            <td>{categoryName(app.settings?.categories.find((c) => c.id === categoryForName(l.fileName)))}</td>
            <td class="dim" title={l.url}>{l.url}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="buttons">
    <div class="left"><label class="check"><input type="checkbox" bind:checked={start} />{t('fileinfo.start')}</label></div>
    <button class="primary" disabled={!checked.size} onclick={add}>{t('grabber.addSelected', { n: checked.size })}</button>
    <button onclick={async () => { await api.cancelPending(token).catch(() => {}); closeWindow(); }}>{t('common.cancel')}</button>
  </div>
</div>

<style>
  .sp {
    flex: 1;
  }
  .fname {
    display: flex;
    align-items: center;
    gap: 5px;
  }
</style>
