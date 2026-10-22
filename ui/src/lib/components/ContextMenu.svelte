<script lang="ts">
  // Fare konumunda açılan bağlam menüsü; ekrandan taşmayı önler.
  import type { MenuItem } from '../menu';
  import MenuList from './MenuList.svelte';

  let { items, x, y, onclose }: { items: MenuItem[]; x: number; y: number; onclose: () => void } = $props();
  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });

  $effect(() => {
    pos = { left: x, top: y };
    requestAnimationFrame(() => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      pos = {
        left: Math.max(0, Math.min(x, window.innerWidth - r.width - 2)),
        top: Math.max(0, Math.min(y, window.innerHeight - r.height - 2)),
      };
    });
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={onKey} onblur={onclose} />

<div class="backdrop" role="presentation" onmousedown={onclose} oncontextmenu={(e) => { e.preventDefault(); onclose(); }}></div>
<div class="ctx" bind:this={el} style:left="{pos.left}px" style:top="{pos.top}px">
  <MenuList {items} {onclose} />
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 900;
  }
  .ctx {
    position: fixed;
    z-index: 901;
  }
</style>
