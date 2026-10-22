<script lang="ts">
  // Seçenekler > Kaydetme yeri: varsayılan klasör ve kategoriler.
  import Icon from '../../lib/icons/Icon.svelte';
  import { pickFolder } from '../../lib/api';
  import { t } from '../../lib/i18n/index.svelte';
  import { categoryIcon, categoryName } from '../../lib/state.svelte';
  import { joinPath } from '../../lib/format';
  import type { Category, Settings } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
  let selId = $state<string | null>(null);
  const cur = $derived(s.categories.find((c) => c.id === selId) ?? null);

  async function browseDefault() {
    const r = await pickFolder(s.defaultSaveDir);
    if (r) {
      const old = s.defaultSaveDir;
      s.defaultSaveDir = r;
      // Varsayılan klasöre bağlı kategori klasörlerini de taşı.
      for (const c of s.categories) {
        if (c.saveDir === old) c.saveDir = r;
        else if (c.saveDir.startsWith(old + '/') || c.saveDir.startsWith(old + '\\')) c.saveDir = r + c.saveDir.slice(old.length);
      }
    }
  }

  async function browseCat() {
    if (!cur) return;
    const r = await pickFolder(cur.saveDir);
    if (r) cur.saveDir = r;
  }

  function add() {
    const name = window.prompt(t('fileinfo.newCategoryName'));
    if (!name) return;
    const c: Category = { id: 'c' + Date.now().toString(36), name, extensions: [], saveDir: joinPath(s.defaultSaveDir, name), builtin: false };
    s.categories.push(c);
    selId = c.id;
  }

  function remove() {
    if (!cur || cur.builtin) return;
    s.categories = s.categories.filter((c) => c.id !== cur.id);
    selId = null;
  }
</script>

<label for="st-def">{t('opt.st.defaultDir')}</label>
<div class="row">
  <input id="st-def" type="text" bind:value={s.defaultSaveDir} />
  <button class="small" onclick={browseDefault}>{t('common.browseLong')}</button>
</div>
<label class="check mt"><input type="checkbox" bind:checked={s.perCategoryDirs} />{t('opt.st.perCategory')}</label>

<div class="mt dim">{t('opt.st.categories')}</div>
<div class="table-wrap cats">
  <table class="grid">
    <colgroup><col style:width="130px" /><col style:width="170px" /><col /></colgroup>
    <thead><tr><th>{t('opt.st.colName')}</th><th>{t('opt.st.colExt')}</th><th>{t('opt.st.colDir')}</th></tr></thead>
    <tbody>
      {#each s.categories as c (c.id)}
        <tr class:selected={selId === c.id} onclick={() => (selId = c.id)}>
          <td><Icon name={categoryIcon(c.id)} /> {categoryName(c)}</td>
          <td class="dim" title={c.extensions.join(' ')}>{c.extensions.join(' ')}</td>
          <td title={c.saveDir}>{c.saveDir}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<div class="row mt">
  <button class="small" onclick={add}><Icon name="plus" size={12} />{t('common.add')}</button>
  <button class="small" onclick={remove} disabled={!cur || cur.builtin}><Icon name="minus" size={12} />{t('common.delete')}</button>
  {#if cur?.builtin}<span class="dim small">{t('opt.st.builtinNote')}</span>{/if}
</div>

{#if cur}
  <fieldset class="mt">
    <legend>{t('opt.st.editTitle')}: {categoryName(cur)}</legend>
    <div class="form">
      <label for="st-name">{t('opt.st.colName')}</label>
      <input id="st-name" type="text" bind:value={cur.name} disabled={cur.builtin} />
      <label for="st-ext">{t('opt.st.colExt')}</label>
      <input id="st-ext" type="text" class="mono" value={cur.extensions.join(' ')} disabled={cur.id === 'general'} oninput={(e) => (cur.extensions = (e.target as HTMLInputElement).value.split(/[\s,;]+/).map((x) => x.replace(/^\./, '').toLowerCase()).filter(Boolean))} />
      <label for="st-dir">{t('opt.st.colDir')}</label>
      <div class="row">
        <input id="st-dir" type="text" bind:value={cur.saveDir} />
        <button class="small" onclick={browseCat}>{t('common.browse')}</button>
      </div>
    </div>
  </fieldset>
{/if}

<style>
  .mt {
    margin-top: 6px;
  }
  label[for='st-def'] {
    display: block;
    margin-bottom: 3px;
  }
  .cats {
    height: 160px;
    margin-top: 3px;
  }
  .form {
    grid-template-columns: 80px 1fr;
  }
</style>
