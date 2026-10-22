<script lang="ts">
  // Basit hash yönlendirici: her Tauri penceresi `index.html#/<tür>?<parametreler>` açar.
  import { onMount, type Component } from 'svelte';
  import { initApp, app } from './lib/state.svelte';
  import { isTauri } from './lib/api';
  import Main from './windows/Main.svelte';

  const loaders: Record<string, () => Promise<{ default: Component }>> = {
    progress: () => import('./windows/Progress.svelte'),
    fileinfo: () => import('./windows/FileInfo.svelte'),
    complete: () => import('./windows/Complete.svelte'),
    addurl: () => import('./windows/AddUrl.svelte'),
    options: () => import('./windows/Options.svelte'),
    scheduler: () => import('./windows/Scheduler.svelte'),
    batch: () => import('./windows/Batch.svelte'),
    grabber: () => import('./windows/Grabber.svelte'),
    media: () => import('./windows/Media.svelte'),
    about: () => import('./windows/About.svelte'),
    wizard: () => import('./windows/Wizard.svelte'),
    properties: () => import('./windows/Properties.svelte'),
    links: () => import('./windows/Links.svelte'),
  };

  function routeOf(): string {
    const h = location.hash.replace(/^#\/?/, '');
    return h.split('?')[0] || 'main';
  }

  let route = $state(routeOf());
  let View = $state<Component | null>(null);
  // Hash değişince bileşeni yeniden kurmak için anahtar.
  let key = $state(location.hash);

  async function load() {
    route = routeOf();
    key = location.hash;
    if (route === 'main' || !loaders[route]) {
      View = Main;
    } else {
      View = (await loaders[route]()).default;
    }
  }

  onMount(() => {
    initApp().catch((e) => console.error('Ayarlar yüklenemedi', e));
    load();
    window.addEventListener('hashchange', load);
    // Tauri'de tarayıcı bağlam menüsünü kapat (giriş alanları hariç).
    const ctx = (e: MouseEvent) => {
      if (isTauri && !(e.target as HTMLElement).closest('input, textarea')) e.preventDefault();
    };
    window.addEventListener('contextmenu', ctx);
    return () => {
      window.removeEventListener('hashchange', load);
      window.removeEventListener('contextmenu', ctx);
    };
  });
</script>

{#if app.ready && View}
  {#key key}
    <View />
  {/key}
{/if}
