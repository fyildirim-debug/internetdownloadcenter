<script lang="ts">
  // Son hız örneklerinin küçük alan grafiği.
  import { formatSpeed } from '../format';

  let { samples, height = 54 }: { samples: number[]; height?: number } = $props();

  const W = 300;
  const max = $derived(Math.max(1, ...samples));
  const pts = $derived.by(() => {
    const n = Math.max(2, samples.length);
    return samples.map((v, i) => `${((i / (n - 1)) * W).toFixed(1)},${(height - 2 - (v / max) * (height - 6)).toFixed(1)}`);
  });
</script>

<div class="graph" style:height="{height}px">
  <svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" width="100%" height="100%">
    {#each [0.25, 0.5, 0.75] as g (g)}
      <line x1="0" x2={W} y1={height * g} y2={height * g} class="grid" />
    {/each}
    {#if pts.length > 1}
      <polygon points="0,{height} {pts.join(' ')} {W * ((samples.length - 1) / Math.max(1, samples.length - 1))},{height}" class="area" />
      <polyline points={pts.join(' ')} class="line" />
    {/if}
  </svg>
  <span class="max">{formatSpeed(max)}</span>
</div>

<style>
  .graph {
    position: relative;
    background: #0d1a12;
    border: 1px solid var(--progress-border);
  }
  svg {
    display: block;
  }
  .grid {
    stroke: #1f3a28;
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }
  .area {
    fill: rgba(60, 220, 90, 0.22);
  }
  .line {
    fill: none;
    stroke: #4dff6e;
    stroke-width: 1.4;
    vector-effect: non-scaling-stroke;
  }
  .max {
    position: absolute;
    top: 1px;
    right: 4px;
    color: #8be59c;
    font-size: 10px;
  }
</style>
