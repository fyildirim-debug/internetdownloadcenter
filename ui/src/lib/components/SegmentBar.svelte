<script lang="ts">
  // IDM tarzı parça çubuğu: beyaz zemin, mavi inen bölgeler, pembe başlangıç çizgileri.
  import type { SegmentView } from '../types';

  let { segments, size, height = 16 }: { segments: SegmentView[]; size: number | null; height?: number } = $props();

  const total = $derived(
    size && size > 0 ? size : Math.max(1, ...segments.map((s) => s.end ?? s.pos)),
  );
  const pct = (v: number) => `${(Math.min(v, total) / total) * 100}%`;
</script>

<div class="segbar" style:height="{height}px">
  {#each segments as s, i (i)}
    <div class="done" style:left={pct(s.start)} style:width={pct(Math.max(0, s.pos - s.start))}></div>
  {/each}
  {#each segments as s, i (i)}
    {#if s.start > 0 || i > 0}
      <div class="start" style:left={pct(s.start)}></div>
    {/if}
    {#if s.active}
      <div class="head" style:left={pct(s.pos)}></div>
    {/if}
  {/each}
</div>

<style>
  .segbar {
    position: relative;
    background: var(--seg-bg);
    border: 1px solid var(--progress-border);
    overflow: hidden;
  }
  .done {
    position: absolute;
    top: 0;
    bottom: 0;
    background: var(--seg-fill);
    background-image: linear-gradient(rgba(255, 255, 255, 0.28), rgba(255, 255, 255, 0) 55%);
  }
  .start {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: var(--seg-start);
  }
  .head {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: #0a3f8c;
  }
</style>
