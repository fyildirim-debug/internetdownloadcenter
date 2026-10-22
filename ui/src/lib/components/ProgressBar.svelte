<script lang="ts">
  // Klasik parlak ilerleme çubuğu. `value` 0–100; null ise belirsiz (kayan) çubuk.
  let {
    value,
    variant = 'active',
    height = 18,
    label = '',
  }: { value: number | null; variant?: 'active' | 'done' | 'paused' | 'error'; height?: number; label?: string } = $props();
</script>

<div class="bar" style:height="{height}px" role="progressbar" aria-valuenow={value ?? undefined} aria-valuemin="0" aria-valuemax="100">
  {#if value == null}
    <div class="fill indeterminate {variant}"></div>
  {:else}
    <div class="fill {variant}" style:width="{Math.max(0, Math.min(100, value))}%"></div>
  {/if}
  {#if label}<span class="label">{label}</span>{/if}
</div>

<style>
  .bar {
    position: relative;
    background: var(--progress-bg);
    border: 1px solid var(--progress-border);
    border-radius: 2px;
    overflow: hidden;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.12);
  }
  .fill {
    height: 100%;
    background: var(--progress-fill);
    transition: width 0.25s linear;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.25);
  }
  .fill.done {
    background: var(--progress-done);
  }
  .fill.paused {
    background: var(--progress-paused);
  }
  .fill.error {
    background: var(--progress-error);
  }
  .indeterminate {
    width: 30%;
    animation: slide 1.4s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
  .label {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--text);
    text-shadow: 0 0 3px var(--progress-bg), 0 0 2px var(--progress-bg);
  }
</style>
