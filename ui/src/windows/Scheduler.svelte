<script lang="ts">
  // Zamanlayıcı ve kuyruk yönetimi.
  import { onMount } from 'svelte';
  import Tabs from '../lib/components/Tabs.svelte';
  import FileIcon from '../lib/icons/FileIcon.svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { api, closeWindow, hashParams, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { formatSize, statusText } from '../lib/format';
  import { app, queueName } from '../lib/state.svelte';
  import type { AfterAction, DownloadView, Queue } from '../lib/types';

  let queues = $state<Queue[]>([]);
  let currentId = $state(hashParams().get('queue') ?? 'main');
  let tab = $state('schedule');
  let downloads = $state<DownloadView[]>([]);
  let selFiles = $state<string[]>([]);
  let dirty = $state(false);
  let onceMode = $state(false);

  const current = $derived(queues.find((q) => q.id === currentId) ?? queues[0]);

  onMount(async () => {
    setTitle(t('sched.title'));
    queues = structuredClone($state.snapshot(app.settings?.queues ?? []) as Queue[]);
    if (!queues.some((q) => q.id === currentId)) currentId = queues[0]?.id ?? 'main';
    onceMode = !!current?.schedule.onceDate;
    downloads = await api.listDownloads();
  });

  $effect(() => {
    onceMode = !!current?.schedule.onceDate;
  });

  const files = $derived.by(() => {
    if (!current) return [];
    const byId = new Map(downloads.map((d) => [d.id, d]));
    const inOrder = current.order.map((id) => byId.get(id)).filter((d): d is DownloadView => !!d);
    const rest = downloads.filter((d) => d.queueId === current.id && !current.order.includes(d.id));
    return [...inOrder, ...rest];
  });

  function mark() {
    dirty = true;
  }

  function toggleDay(d: number) {
    if (!current) return;
    const s = current.schedule;
    s.days = s.days.includes(d) ? s.days.filter((x) => x !== d) : [...s.days, d].sort();
    mark();
  }

  function setOnce(v: boolean) {
    if (!current) return;
    onceMode = v;
    current.schedule.onceDate = v ? new Date().toISOString().slice(0, 10) : null;
    mark();
  }

  async function newQueue() {
    const name = window.prompt(t('sched.newQueueName'));
    if (!name) return;
    const q: Queue = {
      id: 'q' + Date.now().toString(36),
      name,
      maxConcurrent: 1,
      order: [],
      builtin: false,
      schedule: structuredClone($state.snapshot(queues[0].schedule)),
    };
    q.schedule.enabled = false;
    queues.push(q);
    currentId = q.id;
    await api.saveQueue(q);
  }

  async function deleteQueue() {
    if (!current || current.builtin) return;
    if (!confirm(`${t('sched.deleteQueue')}: ${current.name}?`)) return;
    await api.deleteQueue(current.id);
    queues = queues.filter((q) => q.id !== current.id);
    currentId = queues[0]?.id ?? 'main';
  }

  async function apply() {
    for (const q of queues) await api.saveQueue($state.snapshot(q) as Queue);
    dirty = false;
  }

  async function move(delta: number) {
    if (!current || !selFiles.length) return;
    const order = files.map((f) => f.id);
    const idx = selFiles.map((id) => order.indexOf(id)).sort((a, b) => (delta < 0 ? a - b : b - a));
    for (const i of idx) {
      const j = i + delta;
      if (i < 0 || j < 0 || j >= order.length || selFiles.includes(order[j])) continue;
      [order[i], order[j]] = [order[j], order[i]];
    }
    current.order = order;
    await api.moveInQueue(current.id, selFiles, delta);
  }

  async function removeFromQueue() {
    if (!current) return;
    for (const id of selFiles) await api.updateDownload(id, { clearQueue: true });
    current.order = current.order.filter((x) => !selFiles.includes(x));
    downloads = await api.listDownloads();
    selFiles = [];
  }

  async function startNow() {
    if (dirty) await apply();
    await api.startQueue(current.id);
  }

  const afterOpts: [AfterAction, string][] = [
    ['none', 'progress.afterNone'],
    ['exit', 'progress.afterExit'],
    ['shutdown', 'progress.afterShutdown'],
    ['sleep', 'progress.afterSleep'],
    ['hibernate', 'progress.afterHibernate'],
  ];
</script>

<div class="window sched">
  <div class="cols grow">
    <div class="qpane">
      <div class="dim">{t('sched.queues')}</div>
      <div class="table-wrap qlist">
        {#each queues as q (q.id)}
          <div class="qitem" class:sel={q.id === currentId} role="option" aria-selected={q.id === currentId} tabindex="0" onclick={() => (currentId = q.id)} onkeydown={(e) => e.key === 'Enter' && (currentId = q.id)}>
            <Icon name="queue" />
            <span class="ellipsis">{queueName(q)}</span>
            {#if q.schedule.enabled}<span class="clock" title={q.schedule.startTime}>⏰</span>{/if}
          </div>
        {/each}
      </div>
      <div class="row">
        <button class="small" onclick={newQueue}><Icon name="plus" size={12} />{t('common.new')}</button>
        <button class="small" onclick={deleteQueue} disabled={!current || current.builtin}><Icon name="minus" size={12} />{t('common.delete')}</button>
      </div>
    </div>

    {#if current}
      <div class="right">
        <div class="row">
          <label for="s-name">{t('sched.name')}</label>
          {#if current.builtin}
            <input id="s-name" type="text" value={queueName(current)} disabled />
          {:else}
            <input id="s-name" type="text" bind:value={current.name} oninput={mark} />
          {/if}
        </div>
        <Tabs tabs={[{ id: 'schedule', label: t('sched.tabSchedule') }, { id: 'files', label: t('sched.tabFiles') }]} bind:active={tab} />
        <div class="tab-body grow">
          {#if tab === 'schedule'}
            <label class="check"><input type="checkbox" bind:checked={current.schedule.enabled} onchange={mark} />{t('sched.enable')}</label>
            <fieldset disabled={!current.schedule.enabled}>
              <div class="row">
                <label for="s-start" class="w">{t('sched.startAt')}</label>
                <input id="s-start" type="time" bind:value={current.schedule.startTime} onchange={mark} style:flex="0 0 100px" />
              </div>
              <div class="row mt">
                <label class="check"><input type="radio" checked={onceMode} onchange={() => setOnce(true)} />{t('sched.once')}</label>
                {#if onceMode}
                  <input type="date" value={current.schedule.onceDate ?? ''} onchange={(e) => { current.schedule.onceDate = (e.target as HTMLInputElement).value; mark(); }} style:flex="0 0 140px" />
                {/if}
              </div>
              <div class="row">
                <label class="check"><input type="radio" checked={!onceMode} onchange={() => setOnce(false)} />{t('sched.daily')}</label>
              </div>
              {#if !onceMode}
                <div class="days">
                  {#each [0, 1, 2, 3, 4, 5, 6] as dd (dd)}
                    <label class="check"><input type="checkbox" checked={current.schedule.days.length === 0 || current.schedule.days.includes(dd)} onchange={() => { if (current.schedule.days.length === 0) current.schedule.days = [0, 1, 2, 3, 4, 5, 6]; toggleDay(dd); }} />{t(`day.${dd}`)}</label>
                  {/each}
                </div>
              {/if}
              <div class="row mt">
                <label class="check w"><input type="checkbox" bind:checked={current.schedule.stopEnabled} onchange={mark} />{t('sched.stopEnable')}</label>
                <input type="time" bind:value={current.schedule.stopTime} disabled={!current.schedule.stopEnabled} onchange={mark} style:flex="0 0 100px" />
              </div>
              <div class="row mt">
                <label for="s-retry" class="w">{t('sched.retry')}</label>
                <input id="s-retry" type="number" min="0" max="99" bind:value={current.schedule.retryFailed} onchange={mark} style:flex="0 0 70px" />
              </div>
            </fieldset>
            <div class="row">
              <label for="s-after" class="w">{t('sched.after')}</label>
              <select id="s-after" bind:value={current.schedule.afterAll} onchange={mark} style:flex="0 0 200px">
                {#each afterOpts as [v, k] (v)}<option value={v}>{t(k)}</option>{/each}
              </select>
            </div>
          {:else}
            <div class="row">
              <label for="s-mc">{t('sched.maxConcurrent')}</label>
              <input id="s-mc" type="number" min="1" max="32" bind:value={current.maxConcurrent} onchange={mark} style:flex="0 0 60px" />
            </div>
            <div class="filesbox">
              <div class="table-wrap flist">
                {#if !files.length}
                  <p class="dim small" style="padding: 8px">{t('sched.empty')}</p>
                {:else}
                  <table class="grid">
                    <colgroup><col /><col style:width="80px" /><col style:width="110px" /></colgroup>
                    <thead><tr><th>{t('col.fileName')}</th><th class="num">{t('col.size')}</th><th>{t('col.status')}</th></tr></thead>
                    <tbody>
                      {#each files as f (f.id)}
                        <tr class:selected={selFiles.includes(f.id)} onclick={(e) => (selFiles = e.ctrlKey ? (selFiles.includes(f.id) ? selFiles.filter((x) => x !== f.id) : [...selFiles, f.id]) : [f.id])}>
                          <td class="fn"><FileIcon name={f.fileName} /> {f.fileName}</td>
                          <td class="num">{formatSize(f.size)}</td>
                          <td>{statusText(f.status, f.downloaded, f.size, f.error)}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                {/if}
              </div>
              <div class="side">
                <button class="small" disabled={!selFiles.length} onclick={() => move(-1)}><Icon name="up" size={12} />{t('common.moveUp')}</button>
                <button class="small" disabled={!selFiles.length} onclick={() => move(1)}><Icon name="down" size={12} />{t('common.moveDown')}</button>
                <button class="small" disabled={!selFiles.length} onclick={removeFromQueue}><Icon name="x" size={12} />{t('common.remove')}</button>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
  <div class="buttons">
    <div class="left">
      <button onclick={startNow}><Icon name="play" size={12} />{t('sched.startNow')}</button>
      <button onclick={() => api.stopQueue(current.id)}><Icon name="stop" size={12} />{t('sched.stop')}</button>
    </div>
    <button class="primary" onclick={async () => { await apply(); closeWindow(); }}>{t('common.ok')}</button>
    <button onclick={closeWindow}>{t('common.cancel')}</button>
    <button onclick={apply} disabled={!dirty}>{t('common.apply')}</button>
  </div>
</div>

<style>
  .cols {
    display: flex;
    gap: 10px;
    min-height: 0;
  }
  .qpane {
    width: 170px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .qlist {
    flex: 1;
    padding: 2px;
  }
  .qitem {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 2px 5px;
    outline: none;
  }
  .qitem:hover {
    background: var(--hover);
  }
  .qitem.sel {
    background: var(--sel-focus);
    color: var(--sel-focus-text);
  }
  .clock {
    margin-left: auto;
    font-size: 10px;
  }
  .right {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .tab-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  fieldset {
    margin: 0;
  }
  fieldset:disabled {
    opacity: 0.6;
  }
  .w {
    width: 190px;
    flex: none;
  }
  .mt {
    margin-top: 6px;
  }
  .days {
    display: flex;
    gap: 10px;
    padding-left: 22px;
    flex-wrap: wrap;
  }
  .filesbox {
    flex: 1;
    display: flex;
    gap: 6px;
    min-height: 0;
  }
  .flist {
    flex: 1;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .side button {
    justify-content: flex-start;
  }
  .fn {
    display: flex;
    align-items: center;
    gap: 5px;
  }
</style>
