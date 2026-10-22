<script lang="ts">
  // Ana pencerenin altındaki durum çubuğu.
  import Icon from '../icons/Icon.svelte';
  import { t } from '../i18n/index.svelte';
  import { formatSpeed } from '../format';

  let {
    items,
    selected,
    active,
    speed,
    limitKib,
    ontogglelimit,
  }: {
    items: number;
    selected: number;
    active: number;
    speed: number;
    limitKib: number | null;
    ontogglelimit: () => void;
  } = $props();
</script>

<footer class="statusbar">
  <div class="cell grow">{t('sb.items', { n: items })}{selected ? ` — ${t('sb.selected', { n: selected })}` : ''}</div>
  <div class="cell">{t('sb.active', { n: active })}</div>
  <div class="cell w">{t('sb.speed', { v: formatSpeed(speed) || '0 B' + t('unit.perSec') })}</div>
  <button class="cell lim" class:on={limitKib != null} onclick={ontogglelimit} title={t('menu.speedLimiter')}>
    <Icon name="speed" size={14} />
    {limitKib != null ? t('sb.limit', { v: formatSpeed(limitKib * 1024) }) : t('sb.noLimit')}
  </button>
</footer>

<style>
  .statusbar {
    display: flex;
    height: 22px;
    background: var(--bg);
    border-top: 1px solid var(--border-light);
    padding: 1px 16px 1px 2px;
    gap: 2px;
    flex: none;
    position: relative;
  }
  .statusbar::after {
    content: '';
    position: absolute;
    right: 2px;
    bottom: 2px;
    width: 10px;
    height: 10px;
    background: radial-gradient(circle, var(--border-dark) 1px, transparent 1.3px) 0 0 / 4px 4px;
    opacity: 0.6;
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    border: 1px solid var(--border-light);
    border-color: var(--border-light) var(--panel) var(--panel) var(--border-light);
    white-space: nowrap;
  }
  .grow {
    flex: 1;
  }
  .w {
    min-width: 170px;
  }
  .lim {
    min-width: 0;
    height: auto;
    background: none;
    border-radius: 0;
  }
  .lim.on {
    color: #b45309;
    font-weight: 600;
  }
</style>
