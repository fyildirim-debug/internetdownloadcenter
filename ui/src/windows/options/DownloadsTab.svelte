<script lang="ts">
  // Seçenekler > İndirmeler: pencereler, yeniden deneme, kullanıcı aracısı, virüs tarama.
  import { t } from '../../lib/i18n/index.svelte';
  import type { Settings } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
</script>

<div class="grid2">
  <label for="dl-prog">{t('opt.dl.progress')}</label>
  <select id="dl-prog" bind:value={s.downloads.progressDialog}>
    <option value="normal">{t('opt.dl.progress.normal')}</option>
    <option value="minimized">{t('opt.dl.progress.minimized')}</option>
    <option value="none">{t('opt.dl.progress.none')}</option>
  </select>
</div>
<label class="check"><input type="checkbox" bind:checked={s.downloads.showCompleteDialog} />{t('opt.dl.complete')}</label>
<label class="check"><input type="checkbox" bind:checked={s.downloads.showFileInfoDialog} />{t('opt.dl.fileInfo')}</label>
<label class="check ind"><input type="checkbox" bind:checked={s.downloads.startImmediatelyWithDialog} disabled={!s.downloads.showFileInfoDialog} />{t('opt.dl.startImmediately')}</label>
<label class="check"><input type="checkbox" bind:checked={s.downloads.preallocate} />{t('opt.dl.preallocate')}</label>
<label class="check"><input type="checkbox" bind:checked={s.downloads.useServerTime} />{t('opt.dl.serverTime')}</label>

<hr />
<div class="grid2">
  <label for="dl-dup">{t('opt.dl.duplicate')}</label>
  <select id="dl-dup" bind:value={s.downloads.duplicateAction}>
    <option value="ask">{t('opt.dl.dup.ask')}</option>
    <option value="rename">{t('opt.dl.dup.rename')}</option>
    <option value="overwrite">{t('opt.dl.dup.overwrite')}</option>
  </select>
  <label for="dl-retry">{t('opt.dl.retryCount')}</label>
  <input id="dl-retry" type="number" min="0" max="999" bind:value={s.downloads.retryCount} />
  <label for="dl-delay">{t('opt.dl.retryDelay')}</label>
  <input id="dl-delay" type="number" min="1" max="600" bind:value={s.downloads.retryDelaySec} />
</div>

<hr />
<label for="dl-ua" class="blk">{t('opt.dl.userAgent')}</label>
<input id="dl-ua" type="text" class="mono wide" bind:value={s.downloads.userAgent} placeholder="Mozilla/5.0 …" />
<label for="dl-av" class="blk">{t('opt.dl.antivirus')}</label>
<input id="dl-av" type="text" class="mono wide" bind:value={s.downloads.antivirusCommand} placeholder={'clamscan "[File]"'} />

<style>
  .grid2 {
    display: grid;
    grid-template-columns: 260px 220px;
    gap: 5px 8px;
    align-items: center;
    margin-bottom: 4px;
  }
  .ind {
    padding-left: 20px;
  }
  .blk {
    display: block;
    margin: 6px 0 3px;
  }
  .wide {
    width: 100%;
    font-size: 11px;
  }
</style>
