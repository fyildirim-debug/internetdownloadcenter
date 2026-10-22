<script lang="ts">
  // İlk çalıştırma sihirbazı: dil → klasör → tarayıcılar → sistem → video araçları → bitti.
  import { onMount } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import ToolIcon from '../lib/icons/ToolIcon.svelte';
  import { api, closeWindow, EV, on, pickFolder, setTitle, writeClipboard } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';
  import { app, saveSettings, applySettings } from '../lib/state.svelte';
  import type { BrowserInfo, OsIntegration, Settings, ToolsStatus } from '../lib/types';

  const STEPS = ['language', 'folder', 'browsers', 'system', 'tools', 'done'] as const;
  type Step = (typeof STEPS)[number];

  let step = $state<Step>('language');
  let s = $state<Settings>(structuredClone($state.snapshot(app.settings!) as Settings));
  let browsers = $state<BrowserInfo[]>([]);
  let chosen = $state<Set<string>>(new Set());
  let bridgeBusy = $state(false);
  let os = $state<OsIntegration | null>(null);
  let tools = $state<ToolsStatus | null>(null);
  let copied = $state('');

  const idx = $derived(STEPS.indexOf(step));

  onMount(() => {
    setTitle(t('wiz.title'));
    let off: (() => void) | undefined;
    api.browsersStatus().then((b) => {
      browsers = b;
      chosen = new Set(b.filter((x) => x.installed).map((x) => x.id));
    });
    api.osIntegrationStatus().then((o) => (os = o));
    api.toolsStatus().then((x) => (tools = x));
    on<ToolsStatus>(EV.tools, (x) => (tools = x)).then((u) => (off = u));
    return () => off?.();
  });

  function setLang(l: string) {
    s.language = l;
    // Önizleme: dili hemen uygula (kaydetmeden).
    applySettings({ ...(app.settings as Settings), language: l });
  }

  async function next() {
    if (step === 'system' && os) {
      try {
        os = await api.osIntegrationApply($state.snapshot(os) as OsIntegration);
        s.general.launchOnStartup = os.autostart;
      } catch {
        /* sistem entegrasyonu sonra da yapılabilir */
      }
    }
    if (idx < STEPS.length - 1) step = STEPS[idx + 1];
  }

  function back() {
    if (idx > 0) step = STEPS[idx - 1];
  }

  async function finish() {
    s.firstRunDone = true;
    await saveSettings(s);
    await api.openWindow('main');
    closeWindow();
  }

  async function installBridge() {
    bridgeBusy = true;
    try {
      browsers = await api.browsersInstall([...chosen]);
    } finally {
      bridgeBusy = false;
    }
  }

  function toggle(id: string) {
    const n = new Set(chosen);
    if (n.has(id)) n.delete(id);
    else n.add(id);
    chosen = n;
  }

  async function copy(b: BrowserInfo) {
    await writeClipboard(b.extensionPath);
    copied = b.id;
    setTimeout(() => (copied = ''), 1500);
  }

  async function browseDir() {
    const r = await pickFolder(s.defaultSaveDir);
    if (r) {
      const old = s.defaultSaveDir;
      s.defaultSaveDir = r;
      for (const c of s.categories) {
        if (c.saveDir === old) c.saveDir = r;
        else if (c.saveDir.startsWith(old)) c.saveDir = r + c.saveDir.slice(old.length);
      }
    }
  }

  async function getTools() {
    if (tools) tools.busy = true;
    try {
      tools = await api.toolsUpdate();
    } catch (e) {
      if (tools) {
        tools.busy = false;
        tools.message = String(e);
      }
    }
  }

  const installedBrowsers = $derived(browsers.filter((b) => b.installed));
</script>

<div class="wiz">
  <aside class="side">
    <ToolIcon name="app" size={56} />
    <ol>
      {#each STEPS as st, i (st)}
        <li class:cur={st === step} class:done={i < idx}>{t(`wiz.step.${st}`)}</li>
      {/each}
    </ol>
  </aside>
  <main class="body">
    {#if step === 'language'}
      <h2>{t('wiz.welcome')}</h2>
      <p>{t('wiz.welcomeText')}</p>
      <fieldset>
        <legend>{t('wiz.step.language')}</legend>
        <label class="check"><input type="radio" checked={s.language === 'auto'} onchange={() => setLang('auto')} />{t('opt.gen.lang.auto')}</label>
        <label class="check"><input type="radio" checked={s.language === 'tr'} onchange={() => setLang('tr')} />Türkçe</label>
        <label class="check"><input type="radio" checked={s.language === 'en'} onchange={() => setLang('en')} />English</label>
      </fieldset>
      <fieldset>
        <legend>{t('opt.gen.theme')}</legend>
        <div class="row">
          {#each ['light', 'dark', 'system'] as th (th)}
            <label class="check"><input type="radio" bind:group={s.theme} value={th} />{t(`menu.theme${th[0].toUpperCase()}${th.slice(1)}`)}</label>
          {/each}
        </div>
      </fieldset>
    {:else if step === 'folder'}
      <h2>{t('wiz.step.folder')}</h2>
      <p>{t('wiz.folderText')}</p>
      <div class="row">
        <input type="text" bind:value={s.defaultSaveDir} />
        <button onclick={browseDir}>{t('common.browseLong')}</button>
      </div>
      <label class="check mt"><input type="checkbox" bind:checked={s.perCategoryDirs} />{t('opt.st.perCategory')}</label>
      <div class="row mt">
        <span>{t('opt.cn.connections')}</span>
        <select bind:value={s.downloads.defaultConnections} style:flex="0 0 70px">
          {#each Array.from({ length: 16 }, (_, i) => i + 1) as n (n)}<option value={n}>{n}</option>{/each}
        </select>
      </div>
    {:else if step === 'browsers'}
      <h2>{t('wiz.step.browsers')}</h2>
      <p>{t('wiz.browsersText')}</p>
      {#if !installedBrowsers.length}
        <div class="notice warn"><Icon name="warning" />{t('wiz.noBrowsers')}</div>
      {/if}
      <div class="blist">
        {#each installedBrowsers as b (b.id)}
          <div class="bitem">
            <div class="row">
              <label class="check"><input type="checkbox" checked={chosen.has(b.id)} onchange={() => toggle(b.id)} /><b>{b.name}</b></label>
              <span class="sp"></span>
              {#if b.hostRegistered}<span class="ok"><Icon name="check" size={12} /> {t('opt.gen.registered')}</span>{:else}<span class="warn">{t('opt.gen.notRegistered')}</span>{/if}
            </div>
            <div class="small dim steps">{b.family === 'firefox' ? t('wiz.firefoxSteps') : t('wiz.chromiumSteps')}</div>
            <div class="row">
              <input type="text" readonly value={b.extensionPath} class="mono small" />
              <button class="small" onclick={() => copy(b)}><Icon name="copy" size={12} />{copied === b.id ? t('common.copied') : t('common.copy')}</button>
              <button class="small" onclick={() => api.browserOpenExtensions(b.id)}>{t('opt.gen.openExtPage')}</button>
            </div>
            {#if b.sandboxed}<div class="small warn">⚠ {t('opt.gen.sandboxed')}</div>{/if}
          </div>
        {/each}
      </div>
      <div class="row mt">
        <button class="primary" onclick={installBridge} disabled={bridgeBusy || !chosen.size}>{t('wiz.installBridge')}</button>
      </div>
    {:else if step === 'system'}
      <h2>{t('wiz.step.system')}</h2>
      <p>{t('wiz.systemText')}</p>
      {#if os}
        {#if os.desktop === 'kde'}<div class="notice"><Icon name="info" />{t('wiz.kdeDetected')}</div>{/if}
        {#if os.desktop === 'gnome'}<div class="notice"><Icon name="info" />{t('wiz.gnomeDetected')}</div>{/if}
        <fieldset class="mt">
          <label class="check"><input type="checkbox" bind:checked={os.autostart} />{t('opt.sy.autostart')}</label>
          <label class="check"><input type="checkbox" bind:checked={os.urlHandler} />{t('opt.sy.urlHandler')}</label>
          <label class="check"><input type="checkbox" bind:checked={os.contextMenu} />{t('opt.sy.contextMenu')}</label>
          {#if os.kdeDownloadManager != null}
            <label class="check"><input type="checkbox" checked={os.kdeDownloadManager} onchange={(e) => (os!.kdeDownloadManager = (e.target as HTMLInputElement).checked)} />{t('opt.sy.kde')}</label>
          {/if}
          <label class="check"><input type="checkbox" bind:checked={s.general.clipboardMonitor} />{t('opt.gen.clipboard')}</label>
          <label class="check"><input type="checkbox" bind:checked={s.general.closeToTray} />{t('opt.gen.tray')}</label>
        </fieldset>
        {#if os.desktop === 'gnome' || !os.traySupported}
          <div class="notice warn"><Icon name="warning" />{t('opt.sy.trayWarn')}</div>
        {/if}
      {/if}
    {:else if step === 'tools'}
      <h2>{t('wiz.step.tools')}</h2>
      <p>{t('wiz.toolsText')}</p>
      {#if tools}
        <div class="tools">
          {#each [['yt-dlp', tools.ytdlp], ['FFmpeg', tools.ffmpeg], ['Deno', tools.deno]] as const as [n, info] (n)}
            <div class="row"><Icon name={info.installed ? 'check' : 'warning'} /> <b>{n}</b> <span class="dim">{info.version ?? '—'}</span></div>
          {/each}
        </div>
        <div class="row mt">
          <button class="primary" onclick={getTools} disabled={tools.busy}>{t('wiz.downloadTools')}</button>
          {#if tools.busy}<span class="spinner"></span>{/if}
          <span class="dim small">{tools.message ?? ''}</span>
        </div>
      {/if}
      <label class="check mt"><input type="checkbox" bind:checked={s.media.autoUpdateTools} />{t('opt.vd.autoUpdate')}</label>
    {:else}
      <h2>{t('wiz.step.done')}</h2>
      <div class="done">
        <Icon name="check" size={40} />
        <p>{t('wiz.doneText')}</p>
      </div>
    {/if}

    <div class="grow"></div>
    <div class="buttons">
      {#if step === 'tools'}<div class="left"><button onclick={next}>{t('wiz.skip')}</button></div>{/if}
      <button onclick={back} disabled={idx === 0}>{t('common.back')}</button>
      {#if step === 'done'}
        <button class="primary" onclick={finish}>{t('common.finish')}</button>
      {:else}
        <button class="primary" onclick={next}>{t('common.next')}</button>
      {/if}
      <button onclick={finish}>{t('common.cancel')}</button>
    </div>
  </main>
</div>

<style>
  .wiz {
    display: flex;
    height: 100%;
  }
  .side {
    width: 180px;
    background: linear-gradient(180deg, #0b4f9c, #2a83e0 70%, #5fb0ff);
    color: #fff;
    padding: 18px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    flex: none;
  }
  ol {
    margin: 0;
    padding-left: 18px;
    line-height: 2;
  }
  li {
    opacity: 0.7;
  }
  li.cur {
    opacity: 1;
    font-weight: 700;
  }
  li.done {
    opacity: 0.9;
  }
  li.done::marker {
    content: '✓  ';
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 14px 16px 12px;
    gap: 8px;
    overflow: auto;
  }
  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  .grow {
    flex: 1;
  }
  .mt {
    margin-top: 6px;
  }
  .blist {
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: auto;
    max-height: 260px;
  }
  .bitem {
    border: 1px solid var(--border-light);
    background: var(--panel);
    padding: 6px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .steps {
    line-height: 1.4;
  }
  .sp {
    flex: 1;
  }
  .ok {
    color: var(--ok);
  }
  .warn {
    color: var(--warn);
  }
  .tools {
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--panel);
    border: 1px solid var(--border-light);
    padding: 8px;
  }
  .done {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border-light);
    border-top-color: #2a83e0;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
