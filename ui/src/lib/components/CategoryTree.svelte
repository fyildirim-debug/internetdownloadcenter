<script lang="ts">
  // Sol paneldeki kategori ağacı.
  import Icon from '../icons/Icon.svelte';
  import { t } from '../i18n/index.svelte';
  import { app, categoryIcon, categoryName, queueName } from '../state.svelte';
  import type { Filter } from '../menu';

  let {
    filter = $bindable(),
    counts,
    onclose,
    oncontext,
  }: {
    filter: Filter;
    counts: Record<string, number>;
    onclose?: () => void;
    oncontext?: (e: MouseEvent, f: Filter) => void;
  } = $props();

  let openAll = $state(true);
  let openQueues = $state(true);

  const is = (f: Filter) => filter.kind === f.kind && (filter.id ?? '') === (f.id ?? '');
  const key = (f: Filter) => `${f.kind}:${f.id ?? ''}`;
</script>

<aside class="tree">
  <header>
    <span>{t('tree.title')}</span>
    {#if onclose}<button class="x" title={t('common.close')} onclick={onclose}>×</button>{/if}
  </header>
  <div class="body">
    {#snippet node(f: Filter, icon: string, label: string, level: number, expander?: { open: boolean; toggle: () => void })}
      <div
        class="node"
        class:sel={is(f)}
        style:padding-left="{4 + level * 16}px"
        role="treeitem"
        aria-selected={is(f)}
        tabindex="0"
        onclick={() => (filter = f)}
        onkeydown={(e) => e.key === 'Enter' && (filter = f)}
        oncontextmenu={(e) => { e.preventDefault(); filter = f; oncontext?.(e, f); }}
      >
        {#if expander}
          <span class="exp" role="button" tabindex="-1" onclick={(e) => { e.stopPropagation(); expander.toggle(); }} onkeydown={() => {}}>{expander.open ? '−' : '+'}</span>
        {:else}
          <span class="exp none"></span>
        {/if}
        <Icon name={icon} />
        <span class="label">{label}</span>
        {#if counts[key(f)]}<span class="count">({counts[key(f)]})</span>{/if}
      </div>
    {/snippet}

    {@render node({ kind: 'all' }, openAll ? 'folder-open' : 'folder', t('tree.all'), 0, { open: openAll, toggle: () => (openAll = !openAll) })}
    {#if openAll}
      {#each app.settings?.categories ?? [] as c (c.id)}
        {@render node({ kind: 'category', id: c.id }, categoryIcon(c.id), categoryName(c), 1)}
      {/each}
    {/if}
    {@render node({ kind: 'unfinished' }, 'unfinished', t('tree.unfinished'), 0)}
    {@render node({ kind: 'finished' }, 'finished', t('tree.finished'), 0)}
    {@render node({ kind: 'queue', id: '*' }, 'queues', t('tree.queues'), 0, { open: openQueues, toggle: () => (openQueues = !openQueues) })}
    {#if openQueues}
      {#each app.settings?.queues ?? [] as q (q.id)}
        {@render node({ kind: 'queue', id: q.id }, 'queue', queueName(q), 1)}
      {/each}
    {/if}
  </div>
</aside>

<style>
  .tree {
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--border);
    min-width: 0;
    height: 100%;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 22px;
    padding: 0 3px 0 7px;
    background: var(--header);
    border-bottom: 1px solid var(--header-border);
    flex: none;
  }
  .x {
    min-width: 0;
    width: 18px;
    height: 16px;
    padding: 0;
    border: none;
    background: none;
    font-size: 15px;
    line-height: 1;
  }
  .body {
    overflow: auto;
    flex: 1;
    padding: 3px 0;
  }
  .node {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    white-space: nowrap;
    padding-right: 6px;
    outline: none;
  }
  .node .label {
    padding: 1px 3px;
    border: 1px solid transparent;
  }
  .node:hover .label {
    background: var(--hover);
  }
  .node.sel .label {
    background: var(--sel-focus);
    color: var(--sel-focus-text);
  }
  .node:focus-visible .label {
    border: 1px dotted var(--text);
  }
  .exp {
    width: 11px;
    height: 11px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    background: var(--panel);
    font-size: 10px;
    line-height: 1;
    color: var(--text-dim);
    flex: none;
  }
  .exp.none {
    border: none;
    background: none;
  }
  .count {
    color: var(--text-dim);
    font-size: 11px;
  }
</style>
