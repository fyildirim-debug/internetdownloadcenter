<script lang="ts">
  // Açılır menü listesi; alt menüleri sağa açar.
  import type { MenuItem } from '../menu';
  import Icon from '../icons/Icon.svelte';
  import Self from './MenuList.svelte';

  let { items, onclose }: { items: MenuItem[]; onclose: () => void } = $props();
  let openSub = $state<number | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function enter(i: number, item: MenuItem) {
    clearTimeout(timer);
    timer = setTimeout(() => (openSub = item.submenu && !item.disabled ? i : null), 120);
  }

  function run(item: MenuItem) {
    if (item.disabled || item.separator) return;
    if (item.submenu) return;
    onclose();
    item.action?.();
  }
</script>

<div class="menu" role="menu">
  {#each items as item, i (i)}
    {#if item.separator}
      <div class="sep"></div>
    {:else}
      <div
        class="item"
        class:disabled={item.disabled}
        class:bold={item.bold}
        class:open={openSub === i}
        role="menuitem"
        tabindex="-1"
        aria-disabled={item.disabled}
        onmouseenter={() => enter(i, item)}
        onclick={() => run(item)}
        onkeydown={(e) => e.key === 'Enter' && run(item)}
      >
        <span class="gutter">
          {#if item.checked}
            <span class="mark">{item.radio ? '●' : '✓'}</span>
          {:else if item.icon}
            <Icon name={item.icon} />
          {/if}
        </span>
        <span class="label">{item.label}</span>
        <span class="shortcut">{item.shortcut ?? ''}</span>
        <span class="arrow">{item.submenu ? '▸' : ''}</span>
        {#if item.submenu && openSub === i}
          <div class="sub">
            <Self items={item.submenu} {onclose} />
          </div>
        {/if}
      </div>
    {/if}
  {/each}
</div>

<style>
  .menu {
    background: var(--panel);
    border: 1px solid var(--border-dark);
    box-shadow: var(--menu-shadow);
    padding: 2px;
    min-width: 190px;
    background-image: linear-gradient(90deg, var(--bg-2) 26px, var(--border-light) 26px, var(--border-light) 27px, transparent 27px);
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    height: 22px;
    padding-right: 6px;
    border: 1px solid transparent;
    border-radius: 2px;
    white-space: nowrap;
  }
  .item:hover:not(.disabled),
  .item.open {
    background: var(--hover);
    border-color: var(--hover-border);
  }
  .item.disabled {
    color: var(--text-disabled);
  }
  .item.bold .label {
    font-weight: 600;
  }
  .gutter {
    width: 26px;
    display: flex;
    justify-content: center;
    flex: none;
  }
  .mark {
    font-size: 11px;
  }
  .label {
    flex: 1;
    padding: 0 8px 0 6px;
  }
  .shortcut {
    color: var(--text-dim);
    padding-left: 20px;
  }
  .arrow {
    width: 12px;
    text-align: right;
  }
  .sep {
    height: 1px;
    background: var(--border-light);
    margin: 3px 2px 3px 30px;
  }
  .sub {
    position: absolute;
    left: 100%;
    top: -3px;
    z-index: 2;
  }
</style>
