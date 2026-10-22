<script lang="ts">
  // "URL Ekle" penceresi: adres (panodan dolar), isteğe bağlı yetkilendirme ve gelişmiş başlıklar.
  import { onMount } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { api, autoFit, closeWindow, hashParams, readClipboard, setTitle } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { isValidUrl } from '../lib/format';
  import { emptyAddRequest } from '../lib/types';

  const MEDIA_SITES = /(^|\.)(youtube\.com|youtu\.be|vimeo\.com|dailymotion\.com|twitch\.tv|tiktok\.com|instagram\.com|facebook\.com|fb\.watch|x\.com|twitter\.com|soundcloud\.com|bilibili\.com|reddit\.com|streamable\.com)$/i;

  let url = $state(hashParams().get('url') ?? '');
  let useAuth = $state(false);
  let login = $state('');
  let password = $state('');
  let advanced = $state(false);
  let referer = $state('');
  let headers = $state('');
  let cookie = $state('');
  let invalid = $state(false);
  let input: HTMLInputElement | undefined = $state();

  onMount(async () => {
    setTitle(t('addurl.title'));
    if (!url) {
      const clip = (await readClipboard()).trim();
      if (isValidUrl(clip)) url = clip;
    }
    input?.focus();
    input?.select();
  });

  onMount(() => autoFit(540, 140));

  async function ok() {
    const u = url.trim();
    if (!isValidUrl(u)) {
      invalid = true;
      return;
    }
    let host = '';
    try {
      host = new URL(u).hostname;
    } catch {
      /* geçersiz */
    }
    if (MEDIA_SITES.test(host)) {
      await api.openWindow('media', { url: u });
      closeWindow();
      return;
    }
    const req = emptyAddRequest(u);
    req.request.referer = referer.trim() || null;
    const hdrs: [string, string][] = headers
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter((l) => l.includes(':'))
      .map((l) => [l.slice(0, l.indexOf(':')).trim(), l.slice(l.indexOf(':') + 1).trim()]);
    if (useAuth && login) {
      hdrs.push(['Authorization', 'Basic ' + btoa(unescape(encodeURIComponent(`${login}:${password}`)))]);
    }
    req.request.headers = hdrs;
    req.request.rawCookie = cookie.trim() || null;
    const token = await api.createPending(req);
    await api.openWindow('fileinfo', { token });
    closeWindow();
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') closeWindow(); if (e.key === 'Enter' && !(e.target as HTMLElement).matches('textarea')) ok(); }} />

<div class="window">
  <div class="form">
    <label for="au-url"><Icon name="globe" /> {t('addurl.address')}</label>
    <input id="au-url" type="text" bind:this={input} bind:value={url} oninput={() => (invalid = false)} placeholder="https://" />
    <span></span>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={useAuth} />{t('addurl.useAuth')}</label>
      <span class="grow"></span>
      <label class="check"><input type="checkbox" bind:checked={advanced} />{t('addurl.advanced')}</label>
    </div>
    {#if useAuth}
      <span></span>
      <div class="row">
        <Icon name="lock" />
        <input type="text" bind:value={login} placeholder={t('addurl.login')} />
        <input type="password" bind:value={password} placeholder={t('addurl.password')} />
      </div>
    {/if}
    {#if advanced}
      <label for="au-ref">{t('addurl.referer')}</label>
      <input id="au-ref" type="text" bind:value={referer} />
      <label for="au-ck">{t('addurl.cookies')}</label>
      <input id="au-ck" type="text" bind:value={cookie} class="mono" placeholder="ad=değer; ad2=değer2" />
      <span class="full dim small">{t('addurl.headers')}</span>
      <textarea class="full mono" rows="4" bind:value={headers} placeholder="X-Token: abc123"></textarea>
    {/if}
  </div>
  {#if invalid}
    <div class="notice error"><Icon name="error" />{t('addurl.invalid')}</div>
  {/if}
  <div class="grow"></div>
  <div class="buttons">
    <button class="primary" onclick={ok}>{t('common.ok')}</button>
    <button onclick={closeWindow}>{t('common.cancel')}</button>
  </div>
</div>

<style>
  .form {
    grid-template-columns: 110px 1fr;
  }
  .form label:first-child {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .grow {
    flex: 1;
  }
</style>
