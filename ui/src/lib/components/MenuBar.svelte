<script lang="ts">
  // Pencerenin üstündeki klasik menü çubuğu. Bir menü açıkken fareyle gezinilebilir.
  import type { MenuItem } from '../menu';
  import MenuList from './MenuList.svelte';

  let { menus }: { menus: { label: string; items: MenuItem[] }[] } = $props();
  let open = $state<number | null>(null);

  function toggle(i: number) {
    open = open === i ? null : i;
  }
  function close() {
    open = null;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
    if (e.altKey && !e.ctrlKey && !e.shiftKey && e.key.length === 1) {
      const i = menus.findIndex((m) => m.label.toLowerCase().startsWith(e.key.toLowerCase()));
      if (i >= 0) {
        e.preventDefault();
        open = i;
      }
    }
  }
</script>

<svelte:window onkeydown={onKey} onblur={close} />

{#if open !== null}
  <div class="backdrop" role="presentation" onmousedown={close}></div>
{/if}
<nav class="menubar">
  {#each menus as m, i (m.label)}
    <div class="top" class:open={open === i}>
      <button
        class="title"
        onmousedown={(e) => { e.preventDefault(); toggle(i); }}
        onmouseenter={() => open !== null && (open = i)}
      >
        <span><span class="u">{m.label.charAt(0)}</span>{m.label.slice(1)}</span>
      </button>
      {#if open === i}
        <div class="drop">
          <MenuList items={m.items} onclose={close} />
        </div>
      {/if}
    </div>
  {/each}
</nav>

<style>
  .menubar {
    display: flex;
    background: var(--menubar);
    border-bottom: 1px solid var(--border-light);
    height: 22px;
    padding: 0 2px;
    position: relative;
    z-index: 950;
    flex: none;
  }
  .top {
    position: relative;
  }
  .title {
    background: none;
    border: 1px solid transparent;
    border-radius: 0;
    min-width: 0;
    height: 21px;
    padding: 0 8px;
  }
  .title:hover:not(:disabled) {
    background: var(--hover);
    border-color: var(--hover-border);
  }
  .open .title,
  .open .title:hover:not(:disabled) {
    background: var(--sel);
    border-color: var(--sel-border);
  }
  .u {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .drop {
    position: absolute;
    top: 100%;
    left: 0;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 940;
  }
</style>
