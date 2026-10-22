<script lang="ts">
  // Seçenekler > Dosya türleri: yakalanacak/atlanacak uzantılar ve siteler.
  import { t } from '../../lib/i18n/index.svelte';
  import type { Settings } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();

  const DEFAULT =
    '3gp 7z aac ace aif aiff apk appimage arj asf avi bin bz2 cab deb dmg doc docx epub exe flac flv gz gzip img iso jar lzh lz m4a m4v mkv mov mp3 mp4 mpa mpe mpeg mpg msi msix msu odp ods odt ogg ogv opus pdf pps ppt pptx qt r0* r1* ra rar rm rmvb rpm sea sit sitx snap tar tbz2 tgz tif tiff torrent ts txz vob wav webm wma wmv xls xlsx xz z zip zst';

  const words = (v: string) => v.split(/[\s,;]+/).map((x) => x.replace(/^\./, '').trim().toLowerCase()).filter(Boolean);
  const lines = (v: string) => v.split(/\r?\n/).map((x) => x.trim()).filter(Boolean);
</script>

<label for="ft-cap">{t('opt.ft.capture')}</label>
<textarea id="ft-cap" class="mono" rows="6" value={s.fileTypes.captureExtensions.join(' ').toUpperCase()} oninput={(e) => (s.fileTypes.captureExtensions = words((e.target as HTMLTextAreaElement).value))}></textarea>
<div class="row end"><button class="small" onclick={() => (s.fileTypes.captureExtensions = DEFAULT.split(' '))}>{t('opt.ft.default')}</button></div>

<label for="ft-skip">{t('opt.ft.skipExt')}</label>
<input id="ft-skip" type="text" class="mono" value={s.fileTypes.skipExtensions.join(' ')} oninput={(e) => (s.fileTypes.skipExtensions = words((e.target as HTMLInputElement).value))} />

<label for="ft-sites">{t('opt.ft.skipSites')}</label>
<textarea id="ft-sites" class="mono" rows="4" value={s.fileTypes.skipSites.join('\n')} oninput={(e) => (s.fileTypes.skipSites = lines((e.target as HTMLTextAreaElement).value))}></textarea>

<label for="ft-addr">{t('opt.ft.skipAddresses')}</label>
<textarea id="ft-addr" class="mono" rows="3" value={s.fileTypes.skipAddresses.join('\n')} oninput={(e) => (s.fileTypes.skipAddresses = lines((e.target as HTMLTextAreaElement).value))}></textarea>

<div class="row">
  <label for="ft-min">{t('opt.ft.minSize')}</label>
  <input id="ft-min" type="number" min="0" bind:value={s.fileTypes.minSizeKib} style:flex="0 0 90px" />
</div>

<style>
  label {
    display: block;
    margin: 6px 0 3px;
  }
  textarea,
  input[type='text'] {
    width: 100%;
    font-size: 11px;
  }
  .end {
    justify-content: flex-end;
    margin-top: 3px;
  }
  .row label {
    margin: 0;
  }
</style>
