<script lang="ts">
  // Pencere içi onay kutusu (isteğe bağlı onay işaretiyle).
  import Icon from '../icons/Icon.svelte';
  import { t } from '../i18n/index.svelte';

  let {
    message,
    checkLabel = '',
    icon = 'warning',
    onanswer,
  }: { message: string; checkLabel?: string; icon?: string; onanswer: (ok: boolean, checked: boolean) => void } = $props();
  let checked = $state(false);
  let okBtn: HTMLButtonElement | undefined = $state();
  $effect(() => okBtn?.focus());
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onanswer(false, false)} />

<div class="overlay">
  <div class="dlg" role="alertdialog" aria-modal="true">
    <div class="msg">
      <Icon name={icon} size={32} />
      <p>{message}</p>
    </div>
    {#if checkLabel}
      <label class="check"><input type="checkbox" bind:checked />{checkLabel}</label>
    {/if}
    <div class="buttons">
      <button class="primary" bind:this={okBtn} onclick={() => onanswer(true, checked)}>{t('common.yes')}</button>
      <button onclick={() => onanswer(false, false)}>{t('common.no')}</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.18);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .dlg {
    background: var(--bg);
    border: 1px solid var(--border-dark);
    box-shadow: var(--shadow);
    padding: 16px 16px 12px;
    min-width: 340px;
    max-width: 460px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .msg {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  p {
    margin: 4px 0 0;
    line-height: 1.45;
  }
</style>
