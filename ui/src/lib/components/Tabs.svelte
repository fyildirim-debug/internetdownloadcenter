<script lang="ts">
  // Klasik sekme başlıkları.
  let { tabs, active = $bindable() }: { tabs: { id: string; label: string }[]; active: string } = $props();
</script>

<div class="tabs" role="tablist">
  {#each tabs as tab (tab.id)}
    <button role="tab" aria-selected={active === tab.id} class:active={active === tab.id} onclick={() => (active = tab.id)}>
      {tab.label}
    </button>
  {/each}
</div>
