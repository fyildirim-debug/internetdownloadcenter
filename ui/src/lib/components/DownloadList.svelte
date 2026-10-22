<script lang="ts" module>
  export type ColId = 'fileName' | 'queue' | 'size' | 'status' | 'timeLeft' | 'rate' | 'lastTry' | 'description' | 'added' | 'saveTo';

  export interface Column {
    id: ColId;
    width: number;
    visible: boolean;
  }

  export const DEFAULT_COLUMNS: Column[] = [
    { id: 'fileName', width: 300, visible: true },
    { id: 'queue', width: 28, visible: true },
    { id: 'size', width: 82, visible: true },
    { id: 'status', width: 140, visible: true },
    { id: 'timeLeft', width: 92, visible: true },
    { id: 'rate', width: 100, visible: true },
    { id: 'lastTry', width: 150, visible: true },
    { id: 'description', width: 160, visible: true },
    { id: 'added', width: 132, visible: false },
    { id: 'saveTo', width: 200, visible: false },
  ];
</script>

<script lang="ts">
  // Ana penceredeki indirme listesi: sıralanabilir, sütunları boyutlanabilir, sanal kaydırmalı.
  import FileIcon from '../icons/FileIcon.svelte';
  import Icon from '../icons/Icon.svelte';
  import { t } from '../i18n/index.svelte';
  import { formatDate, formatEta, formatSize, formatSpeed, percent, statusText } from '../format';
  import type { DownloadView } from '../types';
  import { app } from '../state.svelte';

  let {
    rows,
    columns = $bindable(),
    selected = $bindable(),
    sortBy = $bindable(),
    sortDir = $bindable(),
    onopen,
    oncontext,
    ondelete,
    oncolumnschange,
  }: {
    rows: DownloadView[];
    columns: Column[];
    selected: string[];
    sortBy: ColId;
    sortDir: 1 | -1;
    onopen: (id: string) => void;
    oncontext: (e: MouseEvent) => void;
    ondelete: () => void;
    oncolumnschange?: () => void;
  } = $props();

  const ROW = 20;
  const labels: Record<ColId, string> = {
    fileName: 'col.fileName',
    queue: 'col.queue',
    size: 'col.size',
    status: 'col.status',
    timeLeft: 'col.timeLeft',
    rate: 'col.rate',
    lastTry: 'col.lastTry',
    description: 'col.description',
    added: 'col.added',
    saveTo: 'col.saveTo',
  };
  const right = new Set<ColId>(['size', 'timeLeft', 'rate']);

  let scroller: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewH = $state(400);
  let anchor = $state<string | null>(null);
  let headerMenu = $state<{ x: number; y: number } | null>(null);

  const visibleCols = $derived(columns.filter((c) => c.visible));
  const totalW = $derived(visibleCols.reduce((a, c) => a + c.width, 0));
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - 10));
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + viewH) / ROW) + 10));
  const slice = $derived(rows.slice(first, last));
  const selSet = $derived(new Set(selected));

  function queueMark(id: string | null): string {
    if (!id) return '';
    const i = app.settings?.queues.findIndex((q) => q.id === id) ?? -1;
    return i >= 0 ? String(i + 1) : '•';
  }

  function clickRow(e: MouseEvent, id: string) {
    if (e.shiftKey && anchor) {
      const a = rows.findIndex((r) => r.id === anchor);
      const b = rows.findIndex((r) => r.id === id);
      const [lo, hi] = a < b ? [a, b] : [b, a];
      const range = rows.slice(lo, hi + 1).map((r) => r.id);
      selected = e.ctrlKey ? [...new Set([...selected, ...range])] : range;
    } else if (e.ctrlKey || e.metaKey) {
      selected = selSet.has(id) ? selected.filter((s) => s !== id) : [...selected, id];
      anchor = id;
    } else {
      selected = [id];
      anchor = id;
    }
  }

  function contextRow(e: MouseEvent, id: string) {
    e.preventDefault();
    if (!selSet.has(id)) {
      selected = [id];
      anchor = id;
    }
    oncontext(e);
  }

  function contextEmpty(e: MouseEvent) {
    e.preventDefault();
    if ((e.target as HTMLElement).closest('tr[data-id]')) return;
    selected = [];
    oncontext(e);
  }

  function key(e: KeyboardEvent) {
    if (!rows.length) return;
    if (e.key === 'a' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      selected = rows.map((r) => r.id);
      return;
    }
    if (e.key === 'Delete') {
      e.preventDefault();
      if (selected.length) ondelete();
      return;
    }
    if (e.key === 'Enter' && selected.length) {
      e.preventDefault();
      onopen(selected[selected.length - 1]);
      return;
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      const cur = rows.findIndex((r) => r.id === (anchor ?? selected[0]));
      let n = cur;
      if (e.key === 'ArrowDown') n = Math.min(rows.length - 1, cur + 1);
      if (e.key === 'ArrowUp') n = Math.max(0, cur - 1);
      if (e.key === 'Home') n = 0;
      if (e.key === 'End') n = rows.length - 1;
      const id = rows[n].id;
      selected = [id];
      anchor = id;
      if (scroller) {
        const top = n * ROW;
        if (top < scroller.scrollTop) scroller.scrollTop = top;
        else if (top + ROW * 2 > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = top + ROW * 2 - scroller.clientHeight;
      }
    }
  }

  function sort(id: ColId) {
    if (sortBy === id) sortDir = sortDir === 1 ? -1 : 1;
    else {
      sortBy = id;
      sortDir = 1;
    }
  }

  function startResize(e: MouseEvent, col: Column) {
    e.preventDefault();
    e.stopPropagation();
    const x0 = e.clientX;
    const w0 = col.width;
    const move = (ev: MouseEvent) => {
      col.width = Math.max(24, w0 + ev.clientX - x0);
    };
    const up = () => {
      window.removeEventListener('mousemove', move);
      window.removeEventListener('mouseup', up);
      oncolumnschange?.();
    };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', up);
  }

  function variant(r: DownloadView): 'active' | 'done' | 'paused' | 'error' {
    if (r.status === 'completed') return 'done';
    if (r.status === 'error') return 'error';
    if (r.status === 'paused' || r.status === 'queued') return 'paused';
    return 'active';
  }

  $effect(() => {
    if (!scroller) return;
    const ro = new ResizeObserver(() => (viewH = scroller!.clientHeight));
    ro.observe(scroller);
    return () => ro.disconnect();
  });
</script>

<div
  class="list table-wrap"
  bind:this={scroller}
  onscroll={() => (scrollTop = scroller!.scrollTop)}
  tabindex="0"
  role="grid"
  aria-multiselectable="true"
  onkeydown={key}
  oncontextmenu={contextEmpty}
  onmousedown={(e) => {
    if (!(e.target as HTMLElement).closest('tr[data-id], th') && e.button === 0) selected = [];
  }}
>
  <table class="grid" style:width="{totalW}px">
    <colgroup>
      {#each visibleCols as c (c.id)}<col style:width="{c.width}px" />{/each}
    </colgroup>
    <thead>
      <tr oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation(); headerMenu = { x: e.clientX, y: e.clientY }; }}>
        {#each visibleCols as c (c.id)}
          <th class:num={right.has(c.id)} onclick={() => sort(c.id)} title={t(c.id === 'queue' ? 'col.queueLong' : labels[c.id])}>
            {t(labels[c.id])}
            {#if sortBy === c.id}<span class="sort">{sortDir === 1 ? '▲' : '▼'}</span>{/if}
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <span class="grip" role="separator" onmousedown={(e) => startResize(e, c)}></span>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#if first > 0}<tr class="spacer" style:height="{first * ROW}px"><td colspan={visibleCols.length}></td></tr>{/if}
      {#each slice as r (r.id)}
        <tr
          data-id={r.id}
          class:selected={selSet.has(r.id)}
          class:err={r.status === 'error'}
          onmousedown={(e) => e.button === 0 && clickRow(e, r.id)}
          ondblclick={() => onopen(r.id)}
          oncontextmenu={(e) => contextRow(e, r.id)}
        >
          {#each visibleCols as c (c.id)}
            {#if c.id === 'fileName'}
              <td class="name" title={r.fileName}>
                <FileIcon name={r.fileName} media={r.kind === 'media'} />
                <span>{r.fileName}</span>
                {#if r.status === 'error'}<Icon name="error" size={12} />{/if}
              </td>
            {:else if c.id === 'queue'}
              <td class="q">{queueMark(r.queueId)}</td>
            {:else if c.id === 'size'}
              <td class="num">{formatSize(r.size)}</td>
            {:else if c.id === 'status'}
              <td class="status" title={statusText(r.status, r.downloaded, r.size, r.error)}>
                <span class="st {variant(r)}">{statusText(r.status, r.downloaded, r.size, r.error)}</span>
                {#if r.status !== 'completed' && percent(r.downloaded, r.size) != null}
                  <span class="mini"><span class={variant(r)} style:width="{percent(r.downloaded, r.size)}%"></span></span>
                {/if}
              </td>
            {:else if c.id === 'timeLeft'}
              <td class="num">{r.status === 'downloading' ? formatEta(r.eta) : ''}</td>
            {:else if c.id === 'rate'}
              <td class="num">{r.status === 'downloading' ? formatSpeed(r.speed) : ''}</td>
            {:else if c.id === 'lastTry'}
              <td>{formatDate(r.lastTryAt ?? r.addedAt)}</td>
            {:else if c.id === 'description'}
              <td title={r.description}>{r.description}</td>
            {:else if c.id === 'added'}
              <td>{formatDate(r.addedAt)}</td>
            {:else if c.id === 'saveTo'}
              <td title={r.saveDir}>{r.saveDir}</td>
            {/if}
          {/each}
        </tr>
      {/each}
      {#if last < rows.length}<tr class="spacer" style:height="{(rows.length - last) * ROW}px"><td colspan={visibleCols.length}></td></tr>{/if}
    </tbody>
  </table>
</div>

{#if headerMenu}
  <div class="hm-back" role="presentation" onmousedown={() => (headerMenu = null)}></div>
  <div class="hm" style:left="{headerMenu.x}px" style:top="{headerMenu.y}px">
    {#each columns as c (c.id)}
      <label class="check">
        <input type="checkbox" bind:checked={c.visible} disabled={c.id === 'fileName'} onchange={() => oncolumnschange?.()} />
        {t(c.id === 'queue' ? 'col.queueLong' : labels[c.id])}
      </label>
    {/each}
  </div>
{/if}

<style>
  .list {
    height: 100%;
    outline: none;
    position: relative;
  }
  .list:focus-visible {
    border-color: var(--input-focus);
  }
  th {
    position: relative;
  }
  th:hover {
    background: var(--btn-hover);
  }
  .sort {
    font-size: 8px;
    margin-left: 4px;
    color: var(--text-dim);
  }
  .grip {
    position: absolute;
    right: -3px;
    top: 0;
    bottom: 0;
    width: 7px;
    cursor: col-resize;
    z-index: 2;
  }
  tbody tr:nth-child(even):not(.selected):not(:hover) td {
    background: var(--row-alt);
  }
  .list:focus tbody tr.selected td {
    background: var(--sel-focus);
    color: var(--sel-focus-text);
  }
  .list:focus tbody tr.selected .st {
    color: inherit;
  }
  td.name {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  td.name span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  td.q {
    text-align: center;
    color: var(--text-dim);
  }
  td.status {
    position: relative;
  }
  .st.error {
    color: var(--err);
  }
  .st.done {
    color: var(--ok);
  }
  .mini {
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: 2px;
    height: 3px;
    background: color-mix(in srgb, var(--border) 35%, transparent);
    border-radius: 2px;
    overflow: hidden;
  }
  .mini span {
    display: block;
    height: 100%;
    background: #2a83e0;
  }
  .mini span.paused {
    background: #d99a0b;
  }
  .mini span.error {
    background: var(--err);
  }
  .spacer td {
    padding: 0;
  }
  .hm-back {
    position: fixed;
    inset: 0;
    z-index: 900;
  }
  .hm {
    position: fixed;
    z-index: 901;
    background: var(--panel);
    border: 1px solid var(--border-dark);
    box-shadow: var(--menu-shadow);
    padding: 4px 10px;
    min-width: 160px;
  }
</style>
