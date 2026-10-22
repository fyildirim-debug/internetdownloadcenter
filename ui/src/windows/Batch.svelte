<script lang="ts">
  // Toplu indirme: adresteki * yerine sayı ya da harf dizisi konur.
  import { onMount } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { api, closeWindow, pickFolder, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { app, categoryName } from '../lib/state.svelte';
  import { emptyAddRequest } from '../lib/types';

  let pattern = $state('https://');
  let mode = $state<'numbers' | 'letters'>('numbers');
  let from = $state('1');
  let to = $state('10');
  let step = $state(1);
  let padding = $state(1);
  let preview = $state<string[]>([]);
  let category = $state('general');
  let dir = $state('');
  let error = $state('');

  onMount(() => {
    setTitle(t('batch.title'));
    dir = app.settings?.defaultSaveDir ?? '';
  });

  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const args = [pattern, mode, from, to, step, padding] as const;
    clearTimeout(timer);
    timer = setTimeout(async () => {
      void args;
      error = '';
      if (!pattern.includes('*')) {
        preview = [];
        if (pattern.length > 'https://'.length) error = t('batch.noStar');
        return;
      }
      try {
        preview = await api.batchPreview(pattern, from, to, Math.max(1, step), Math.max(1, padding));
      } catch (e) {
        error = String(e);
        preview = [];
      }
    }, 200);
  });

  function setMode(m: 'numbers' | 'letters') {
    mode = m;
    if (m === 'letters') {
      from = 'a';
      to = 'z';
    } else {
      from = '1';
      to = '10';
    }
  }

  function onCategory() {
    const c = app.settings?.categories.find((x) => x.id === category);
    if (c) dir = c.saveDir || app.settings!.defaultSaveDir;
  }

  async function addAll() {
    if (!preview.length) return;
    await api.addMany(
      preview.map((u) => ({ ...emptyAddRequest(u), start: false, category, saveDir: dir || null })),
    );
    closeWindow();
  }
</script>

<div class="window">
  <label for="b-url">{t('batch.address')}</label>
  <input id="b-url" type="text" bind:value={pattern} class="mono" />
  <span class="dim small">{t('batch.hint')}</span>
  <fieldset>
    <legend>{t('batch.mode')}</legend>
    <div class="row">
      <label class="check"><input type="radio" checked={mode === 'numbers'} onchange={() => setMode('numbers')} />{t('batch.numbers')}</label>
      <label class="check"><input type="radio" checked={mode === 'letters'} onchange={() => setMode('letters')} />{t('batch.letters')}</label>
    </div>
    <div class="row nums">
      <label for="b-from">{t('batch.from')}</label>
      <input id="b-from" type="text" bind:value={from} />
      <label for="b-to">{t('batch.to')}</label>
      <input id="b-to" type="text" bind:value={to} />
      <label for="b-step">{t('batch.step')}</label>
      <input id="b-step" type="number" min="1" bind:value={step} />
      {#if mode === 'numbers'}
        <label for="b-pad">{t('batch.padding')}</label>
        <input id="b-pad" type="number" min="1" max="10" bind:value={padding} />
      {/if}
    </div>
  </fieldset>
  <div class="form">
    <label for="b-cat">{t('fileinfo.category')}</label>
    <select id="b-cat" bind:value={category} onchange={onCategory}>
      {#each app.settings?.categories ?? [] as c (c.id)}<option value={c.id}>{categoryName(c)}</option>{/each}
    </select>
    <label for="b-dir">{t('fileinfo.saveAs')}</label>
    <div class="row">
      <input id="b-dir" type="text" bind:value={dir} />
      <button class="small" onclick={async () => { const r = await pickFolder(dir); if (r) dir = r; }}>{t('common.browse')}</button>
    </div>
  </div>
  <div class="row"><b>{t('batch.preview')}</b><span class="dim">— {t('batch.count', { n: preview.length })}</span></div>
  <div class="table-wrap grow prev">
    {#if error}
      <div class="notice warn" style="margin: 6px"><Icon name="warning" />{error}</div>
    {/if}
    {#each preview.slice(0, 500) as u, i (i)}
      <div class="line mono">{u}</div>
    {/each}
    {#if preview.length > 500}<div class="line dim">… +{preview.length - 500}</div>{/if}
  </div>
  <div class="buttons">
    <button class="primary" disabled={!preview.length} onclick={addAll}>{t('batch.addAll')}</button>
    <button onclick={closeWindow}>{t('common.cancel')}</button>
  </div>
</div>

<style>
  .form {
    grid-template-columns: 110px 1fr;
  }
  .nums {
    margin-top: 6px;
  }
  .nums input {
    flex: 0 0 64px !important;
  }
  .prev {
    padding: 2px 0;
  }
  .line {
    padding: 1px 6px;
    font-size: 11px;
    white-space: nowrap;
  }
</style>
