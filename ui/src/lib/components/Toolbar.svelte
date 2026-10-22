<script lang="ts">
  // IDM tarzı büyük simgeli araç çubuğu; etiketler simgenin altında.
  import ToolIcon from '../icons/ToolIcon.svelte';
  import type { ToolButton } from '../menu';

  let { buttons, compact = false }: { buttons: ToolButton[]; compact?: boolean } = $props();
</script>

<div class="toolbar" class:compact>
  {#each buttons as b (b.id)}
    {#if b.sepBefore}<div class="sep"></div>{/if}
    <button class="tool" title={b.label} disabled={b.disabled} onclick={b.action}>
      <ToolIcon name={b.icon} size={compact ? 26 : 38} disabled={b.disabled} />
      <span>{b.label}</span>
    </button>
  {/each}
  <div class="fill"></div>
  <div class="brand" aria-hidden="true">
    <ToolIcon name="app" size={compact ? 26 : 40} />
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: stretch;
    background: var(--toolbar);
    border-bottom: 1px solid var(--toolbar-border);
    padding: 3px 4px 2px;
    gap: 1px;
    flex: none;
    overflow-x: hidden;
  }
  .tool {
    flex-direction: column;
    justify-content: flex-start;
    min-width: 66px;
    max-width: 92px;
    height: auto;
    min-height: 70px;
    padding: 3px 3px 2px;
    gap: 3px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
  }
  .compact .tool {
    min-height: 46px;
    min-width: 58px;
  }
  .tool span {
    font-size: 11px;
    line-height: 1.12;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: normal;
    text-align: center;
  }
  .tool:hover:not(:disabled) {
    background: linear-gradient(#f3f9ff, #d9ecfc);
    border-color: #8cc0ee;
  }
  :global([data-theme='dark']) .tool:hover:not(:disabled) {
    background: linear-gradient(#34465a, #2b394a);
    border-color: #4a76a6;
  }
  .tool:active:not(:disabled) {
    background: linear-gradient(#cfe4f8, #e6f1fb);
  }
  .tool:disabled {
    background: transparent;
    border-color: transparent;
  }
  .sep {
    width: 1px;
    margin: 6px 4px;
    background: var(--toolbar-border);
    box-shadow: 1px 0 0 rgba(255, 255, 255, 0.6);
  }
  .fill {
    flex: 1;
  }
  .brand {
    display: flex;
    align-items: center;
    padding: 0 10px 0 6px;
    opacity: 0.95;
  }
</style>
