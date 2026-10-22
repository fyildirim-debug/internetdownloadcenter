<script lang="ts">
  // İndirme ilerleme penceresi (IDM'in imza penceresi).
  import { onMount } from 'svelte';
  import Tabs from '../lib/components/Tabs.svelte';
  import ProgressBar from '../lib/components/ProgressBar.svelte';
  import SegmentBar from '../lib/components/SegmentBar.svelte';
  import SpeedGraph from '../lib/components/SpeedGraph.svelte';
  import FileIcon from '../lib/icons/FileIcon.svelte';
  import { api, closeWindow, EV, hashParams, on, resizeWindow, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { formatEta, formatError, formatPercent, formatSize, formatSpeed, percent } from '../lib/format';
  import type { AfterAction, DownloadDetail, DownloadView, ProgressTick } from '../lib/types';
  import { MAX_CONNECTIONS } from '../lib/types';
  import { app } from '../lib/state.svelte';

  const id = hashParams().get('id') ?? '';
  let d = $state<DownloadDetail | null>(null);
  let missing = $state(false);
  let tab = $state('status');
  let details = $state(false);
  let history = $state<number[]>([]);

  // Hız sınırlayıcı sekmesi
  let limitOn = $state(false);
  let limitKib = $state(512);
  let conns = $state(8);
  // Bitince sekmesi
  let after = $state<AfterAction>('none');
  let showComplete = $state(true);

  try {
    details = localStorage.getItem('idc-progress-details') === '1';
  } catch {
    /* yok */
  }

  const W = 470;
  const H_SMALL = 352;
  const H_BIG = 610;

  onMount(() => {
    const offs: (() => void)[] = [];
    (async () => {
      try {
        d = await api.getDownload(id);
        history = d.speedHistory.slice(-120);
        limitOn = d.speedLimit != null;
        if (d.speedLimit) limitKib = Math.round(d.speedLimit / 1024);
        conns = d.maxConnections;
        after = d.after;
        showComplete = app.settings?.downloads.showCompleteDialog ?? true;
      } catch {
        missing = true;
      }
    })();
    on<ProgressTick[]>(EV.progress, (ticks) => {
      const tk = ticks.find((x) => x.id === id);
      if (!tk || !d) return;
      d.status = tk.status;
      d.size = tk.size;
      d.downloaded = tk.downloaded;
      d.speed = tk.speed;
      d.eta = tk.eta;
      d.connections = tk.connections;
      d.segments = tk.segments;
      history.push(tk.speed);
      if (history.length > 120) history.shift();
    }).then((u) => offs.push(u));
    on<DownloadView>(EV.changed, (v) => {
      if (v.id !== id || !d) return;
      const was = d.status;
      Object.assign(d, v);
      if (v.status === 'completed' && was !== 'completed') setTimeout(closeWindow, 600);
    }).then((u) => offs.push(u));
    on<string[]>(EV.removed, (ids) => ids.includes(id) && closeWindow()).then((u) => offs.push(u));
    resizeWindow(W, details ? H_BIG : H_SMALL);
    return () => offs.forEach((f) => f());
  });

  const pct = $derived(d ? percent(d.downloaded, d.size) : null);
  $effect(() => {
    if (!d) return;
    const p = pct != null ? `${Math.floor(pct)}% ` : '';
    setTitle(`${p}${d.fileName}`);
  });

  const running = $derived(d != null && (d.status === 'downloading' || d.status === 'connecting' || d.status === 'processing' || d.status === 'queued'));

  const statusLine = $derived.by(() => {
    if (!d) return '';
    if (d.status === 'downloading') return t('status.receiving');
    if (d.status === 'error') return formatError(d.error);
    return t(`status.${d.status}`);
  });

  const variant = $derived(
    !d ? 'active' : d.status === 'error' ? 'error' : d.status === 'paused' || d.status === 'queued' ? 'paused' : d.status === 'completed' ? 'done' : 'active',
  );

  function toggleDetails() {
    details = !details;
    try {
      localStorage.setItem('idc-progress-details', details ? '1' : '0');
    } catch {
      /* yok */
    }
    resizeWindow(W, details ? H_BIG : H_SMALL);
  }

  async function pauseResume() {
    if (!d) return;
    if (running) await api.pauseDownloads([id]);
    else await api.startDownloads([id]);
  }

  async function cancel() {
    if (d && running) await api.pauseDownloads([id]);
    closeWindow();
  }

  async function applyLimit() {
    await api.updateDownload(id, { speedLimit: limitOn ? Math.max(1, limitKib) * 1024 : 0, maxConnections: conns });
  }

  async function applyAfter() {
    await api.updateDownload(id, { after, showCompleteDialog: showComplete });
  }

  function segInfo(info: string): string {
    const k = `progress.segInfo.${info}`;
    const s = t(k);
    return s === k ? info : s;
  }
</script>

<div class="window prog">
  {#if missing}
    <p>{t('progress.closed')}</p>
    <div class="buttons"><button onclick={closeWindow}>{t('common.close')}</button></div>
  {:else if d}
    <Tabs
      tabs={[
        { id: 'status', label: t('progress.tabStatus') },
        { id: 'limiter', label: t('progress.tabLimiter') },
        { id: 'after', label: t('progress.tabAfter') },
      ]}
      bind:active={tab}
    />
    <div class="tab-body body">
      {#if tab === 'status'}
        <div class="url">
          <FileIcon name={d.fileName} size={32} media={d.kind === 'media'} />
          <div class="urltext selectable" title={d.url}>{d.url}</div>
        </div>
        <div class="stat-line" class:err={d.status === 'error'}>{statusLine}</div>
        <div class="info">
          <span>{t('progress.fileSize')}</span><b>{d.size != null ? formatSize(d.size) : t('common.unknown')}</b>
          <span>{t('progress.downloaded')}</span>
          <b>{formatSize(d.downloaded) || '0 B'}{pct != null ? ` (${formatPercent(d.downloaded, d.size)})` : ''}</b>
          <span>{t('progress.rate')}</span><b>{running ? formatSpeed(d.speed) || '—' : '—'}</b>
          <span>{t('progress.timeLeft')}</span><b>{running && d.eta != null ? formatEta(d.eta) : '—'}</b>
          <span>{t('progress.resumable')}</span>
          <b class:yes={d.resumable} class:no={!d.resumable}>{d.resumable ? t('common.yes') : t('common.no')}</b>
        </div>
      {:else if tab === 'limiter'}
        <label class="check"><input type="checkbox" bind:checked={limitOn} onchange={applyLimit} />{t('progress.limitEnable')}</label>
        <div class="row lim">
          <span>{t('progress.limitValue')}</span>
          <input type="number" min="1" step="16" bind:value={limitKib} disabled={!limitOn} onchange={applyLimit} />
          <span>KB/{t('time.s')}</span>
        </div>
        <div class="row lim">
          <span>{t('progress.connections')}</span>
          <select bind:value={conns} onchange={applyLimit} disabled={d.kind === 'media' && running}>
            {#each Array.from({ length: MAX_CONNECTIONS }, (_, i) => i + 1) as n (n)}<option value={n}>{n}</option>{/each}
          </select>
        </div>
        <p class="dim small">{t('progress.limitHint')}</p>
      {:else}
        <label class="check"><input type="checkbox" bind:checked={showComplete} onchange={applyAfter} />{t('progress.afterShowComplete')}</label>
        <fieldset>
          <legend>{t('progress.afterTitle')}</legend>
          {#each [['none', 'progress.afterNone'], ['exit', 'progress.afterExit'], ['shutdown', 'progress.afterShutdown'], ['sleep', 'progress.afterSleep'], ['hibernate', 'progress.afterHibernate']] as [v, k] (v)}
            <label class="check"><input type="radio" name="after" value={v} bind:group={after} onchange={applyAfter} />{t(k)}</label>
          {/each}
        </fieldset>
        <p class="dim small">{t('progress.afterWarn')}</p>
      {/if}
    </div>

    <ProgressBar value={d.status === 'completed' ? 100 : pct} variant={variant} height={20} />

    <div class="buttons">
      <div class="left">
        <button onclick={toggleDetails}>{details ? t('progress.hideDetails') : t('progress.showDetails')}</button>
      </div>
      <button class="primary" onclick={pauseResume} disabled={d.status === 'completed'}>
        {running ? t('progress.pause') : t('progress.resume')}
      </button>
      <button onclick={cancel}>{t('progress.cancel')}</button>
    </div>

    {#if details}
      <div class="details">
        <div class="dim small">{t('progress.segmentsTitle')}</div>
        <SegmentBar segments={d.segments} size={d.size} height={16} />
        <div class="table-wrap conns">
          <table class="grid">
            <colgroup><col style:width="36px" /><col style:width="110px" /><col style:width="88px" /><col /></colgroup>
            <thead><tr><th>{t('progress.colN')}</th><th class="num">{t('progress.colDownloaded')}</th><th class="num">{t('progress.colSpeed')}</th><th>{t('progress.colInfo')}</th></tr></thead>
            <tbody>
              {#each d.segments as s, i (i)}
                <tr>
                  <td>{i + 1}</td>
                  <td class="num">{formatSize(s.pos - s.start) || '0 B'}</td>
                  <td class="num">{s.active ? formatSpeed(s.speed) : ''}</td>
                  <td>{segInfo(s.info)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <SpeedGraph samples={history} height={60} />
      </div>
    {/if}
  {/if}
</div>

<style>
  .prog {
    gap: 7px;
  }
  .body {
    height: 182px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .url {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .urltext {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--link);
  }
  .stat-line {
    font-weight: 600;
  }
  .stat-line.err {
    color: var(--err);
  }
  .info {
    display: grid;
    grid-template-columns: 130px 1fr;
    row-gap: 3px;
  }
  .info b {
    font-weight: normal;
  }
  .yes {
    color: var(--ok);
    font-weight: 600 !important;
  }
  .no {
    color: var(--err);
    font-weight: 600 !important;
  }
  .lim {
    margin: 4px 0;
  }
  .lim span:first-child {
    width: 170px;
    flex: none;
  }
  .lim input,
  .lim select {
    flex: 0 0 100px;
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex: 1;
    min-height: 0;
  }
  .conns {
    flex: 1;
    min-height: 60px;
  }
</style>
