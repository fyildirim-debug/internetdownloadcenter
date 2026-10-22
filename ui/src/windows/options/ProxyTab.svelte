<script lang="ts">
  // Seçenekler > Vekil sunucu.
  import { t } from '../../lib/i18n/index.svelte';
  import type { Settings } from '../../lib/types';

  let { s = $bindable() }: { s: Settings } = $props();
</script>

<fieldset>
  <legend>{t('opt.px.mode')}</legend>
  <label class="check"><input type="radio" value="system" bind:group={s.proxy.mode} />{t('opt.px.system')}</label>
  <label class="check"><input type="radio" value="none" bind:group={s.proxy.mode} />{t('opt.px.none')}</label>
  <label class="check"><input type="radio" value="manual" bind:group={s.proxy.mode} />{t('opt.px.manual')}</label>
</fieldset>
<fieldset disabled={s.proxy.mode !== 'manual'}>
  <label for="px-url" class="blk">{t('opt.px.url')}</label>
  <input id="px-url" type="text" class="mono wide" bind:value={s.proxy.url} placeholder="socks5://127.0.0.1:1080" />
  <div class="form mt">
    <label for="px-u">{t('opt.px.user')}</label>
    <input id="px-u" type="text" bind:value={s.proxy.username} />
    <label for="px-p">{t('opt.px.pass')}</label>
    <input id="px-p" type="password" bind:value={s.proxy.password} />
  </div>
  <label for="px-by" class="blk">{t('opt.px.bypass')}</label>
  <textarea id="px-by" class="mono wide" rows="4" value={s.proxy.bypass.join('\n')} oninput={(e) => (s.proxy.bypass = (e.target as HTMLTextAreaElement).value.split(/\r?\n/).map((x) => x.trim()).filter(Boolean))}></textarea>
</fieldset>

<style>
  fieldset:disabled {
    opacity: 0.6;
  }
  .blk {
    display: block;
    margin: 6px 0 3px;
  }
  .wide {
    width: 100%;
  }
  .mt {
    margin-top: 6px;
  }
  .form {
    grid-template-columns: 110px 220px;
  }
</style>
