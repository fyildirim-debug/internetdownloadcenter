<script lang="ts">
  // Ağaç, menü ve düğmelerde kullanılan küçük simgeler (16×16 çizim alanı).
  let { name, size = 16, title = '' }: { name: string; size?: number; title?: string } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 16 16" aria-hidden={title ? undefined : 'true'} role={title ? 'img' : undefined}>
  {#if title}<title>{title}</title>{/if}
  {#if name === 'folder' || name === 'cat-general'}
    <path d="M1 3.5h5l1.5 1.5H15v8.5H1z" fill="#f6cf5a" stroke="#b38710" stroke-width="0.8" />
    <path d="M1 6h14v7.5H1z" fill="#fbdc7c" stroke="#b38710" stroke-width="0.8" />
  {:else if name === 'folder-open'}
    <path d="M1 3.5h5l1.5 1.5H14v2H1z" fill="#f6cf5a" stroke="#b38710" stroke-width="0.8" />
    <path d="M1 13.5L3 7h12.5l-2 6.5z" fill="#fbdc7c" stroke="#b38710" stroke-width="0.8" />
  {:else if name === 'all'}
    <path d="M1 3.5h5l1.5 1.5H15v8.5H1z" fill="#8cc3f5" stroke="#1d5fa8" stroke-width="0.8" />
    <path d="M6.5 6h3v3h2L8 12.5 4.5 9h2z" fill="#2c8a1e" />
  {:else if name === 'unfinished'}
    <path d="M1 3.5h5l1.5 1.5H15v8.5H1z" fill="#f6cf5a" stroke="#b38710" stroke-width="0.8" />
    <path d="M6.5 6h3v3h2L8 12.5 4.5 9h2z" fill="#1d6fd0" />
  {:else if name === 'finished'}
    <path d="M1 3.5h5l1.5 1.5H15v8.5H1z" fill="#f6cf5a" stroke="#b38710" stroke-width="0.8" />
    <path d="M4.5 9l2.3 2.3L11.5 6.5" fill="none" stroke="#1c8a12" stroke-width="2" stroke-linecap="round" />
  {:else if name === 'queues' || name === 'queue'}
    <rect x="2" y="2" width="12" height="12" rx="1.5" fill="#fff" stroke="#5a6676" />
    <path d="M4.5 5.5h7M4.5 8h7M4.5 10.5h4.5" stroke="#2f7fd6" stroke-width="1.4" />
  {:else if name === 'grabber'}
    <circle cx="8" cy="8" r="6" fill="none" stroke="#6b737e" />
    <circle cx="8" cy="8" r="3" fill="none" stroke="#6b737e" />
    <path d="M8 1v14M1 8h14M3 3l10 10M13 3L3 13" stroke="#6b737e" stroke-width="0.7" />
  {:else if name === 'cat-compressed'}
    <rect x="2" y="2.5" width="12" height="11" rx="1" fill="#f3c13a" stroke="#9c6c06" />
    <path d="M7 2.5h2v11H7z" fill="#7b5a12" opacity="0.5" />
    <path d="M7 4h2M7 6h2M7 8h2" stroke="#fff" stroke-width="0.8" />
  {:else if name === 'cat-documents'}
    <path d="M3 1.5h7l3 3v10H3z" fill="#fff" stroke="#5a6676" stroke-width="0.9" />
    <path d="M5 6h6M5 8h6M5 10h6M5 12h4" stroke="#2f7fd6" stroke-width="0.9" />
  {:else if name === 'cat-music'}
    <path d="M6 12V3.5l7-1.5V10.5" fill="none" stroke="#7a3fc2" stroke-width="1.5" />
    <ellipse cx="4.5" cy="12" rx="2.3" ry="1.8" fill="#7a3fc2" />
    <ellipse cx="11.5" cy="10.5" rx="2.3" ry="1.8" fill="#7a3fc2" />
  {:else if name === 'cat-programs'}
    <rect x="1.5" y="2.5" width="13" height="11" rx="1" fill="#e9eef5" stroke="#43506a" />
    <rect x="1.5" y="2.5" width="13" height="2.5" fill="#2f7fd6" />
    <circle cx="8" cy="9.5" r="2.3" fill="none" stroke="#43506a" stroke-width="1.4" />
  {:else if name === 'cat-video'}
    <rect x="1.5" y="3" width="13" height="10" rx="1" fill="#2a2f38" />
    <path d="M6.5 5.5v5l4-2.5z" fill="#7cc4ff" />
  {:else if name === 'play'}
    <path d="M4 2.5v11l9-5.5z" fill="#2c9a1e" />
  {:else if name === 'pause'}
    <rect x="3.5" y="3" width="3" height="10" fill="#c22" /><rect x="9.5" y="3" width="3" height="10" fill="#c22" />
  {:else if name === 'stop'}
    <rect x="3" y="3" width="10" height="10" rx="1" fill="#d0201a" />
  {:else if name === 'plus'}
    <path d="M8 3v10M3 8h10" stroke="#2c9a1e" stroke-width="2.2" stroke-linecap="round" />
  {:else if name === 'minus'}
    <path d="M3 8h10" stroke="#d0201a" stroke-width="2.2" stroke-linecap="round" />
  {:else if name === 'x'}
    <path d="M4 4l8 8M12 4l-8 8" stroke="#d0201a" stroke-width="2" stroke-linecap="round" />
  {:else if name === 'check'}
    <path d="M3 8.5l3 3L13 4.5" fill="none" stroke="#1c8a12" stroke-width="2" stroke-linecap="round" />
  {:else if name === 'up'}
    <path d="M8 3l5 6H3z" fill="currentColor" />
  {:else if name === 'down'}
    <path d="M8 13L3 7h10z" fill="currentColor" />
  {:else if name === 'chev-right'}
    <path d="M6 3.5L10.5 8 6 12.5" fill="none" stroke="currentColor" stroke-width="1.4" />
  {:else if name === 'chev-down'}
    <path d="M3.5 6L8 10.5 12.5 6" fill="none" stroke="currentColor" stroke-width="1.4" />
  {:else if name === 'gear'}
    <circle cx="8" cy="8" r="5" fill="none" stroke="#5a6676" stroke-width="3" stroke-dasharray="2.2 1.7" />
    <circle cx="8" cy="8" r="2" fill="#5a6676" />
  {:else if name === 'warning'}
    <path d="M8 1.5l7 13H1z" fill="#f6c01e" stroke="#9c6c06" stroke-width="0.8" stroke-linejoin="round" />
    <path d="M8 6v4" stroke="#222" stroke-width="1.6" stroke-linecap="round" /><circle cx="8" cy="12.2" r="0.9" fill="#222" />
  {:else if name === 'info'}
    <circle cx="8" cy="8" r="6.5" fill="#2f7fd6" />
    <path d="M8 7v4.5" stroke="#fff" stroke-width="1.6" stroke-linecap="round" /><circle cx="8" cy="4.8" r="1" fill="#fff" />
  {:else if name === 'error'}
    <circle cx="8" cy="8" r="6.5" fill="#d0201a" />
    <path d="M5.5 5.5l5 5M10.5 5.5l-5 5" stroke="#fff" stroke-width="1.6" stroke-linecap="round" />
  {:else if name === 'globe'}
    <circle cx="8" cy="8" r="6.5" fill="#5aa9ef" stroke="#1d5fa8" />
    <path d="M1.5 8h13M8 1.5c-3 3.5-3 9.5 0 13M8 1.5c3 3.5 3 9.5 0 13" fill="none" stroke="#e8f4ff" stroke-width="0.8" />
  {:else if name === 'lock'}
    <rect x="3" y="7" width="10" height="7" rx="1" fill="#f3b723" stroke="#9c6c06" />
    <path d="M5 7V5a3 3 0 016 0v2" fill="none" stroke="#6b737e" stroke-width="1.5" />
  {:else if name === 'refresh'}
    <path d="M13 8a5 5 0 11-1.5-3.5" fill="none" stroke="#2c9a1e" stroke-width="1.8" />
    <path d="M12.5 1.5v4h-4" fill="none" stroke="#2c9a1e" stroke-width="1.8" />
  {:else if name === 'copy'}
    <rect x="5" y="5" width="9" height="9" rx="1" fill="#fff" stroke="#5a6676" />
    <rect x="2" y="2" width="9" height="9" rx="1" fill="#e9eef5" stroke="#5a6676" />
  {:else if name === 'link'}
    <path d="M7 9.5l2.5-2.5M6 6.5L4.2 8.3a2.3 2.3 0 003.3 3.3L9.3 9.8M10 9.5l1.8-1.8a2.3 2.3 0 00-3.3-3.3L6.7 6.2" fill="none" stroke="#2f7fd6" stroke-width="1.5" stroke-linecap="round" />
  {:else if name === 'speed'}
    <path d="M2 12a6 6 0 1112 0" fill="none" stroke="#5a6676" stroke-width="1.6" />
    <path d="M8 12l3.5-5" stroke="#d0201a" stroke-width="1.8" stroke-linecap="round" />
  {:else if name === 'browser'}
    <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" fill="#fff" stroke="#43506a" />
    <path d="M1.5 5.5h13" stroke="#43506a" /><circle cx="3.5" cy="4" r="0.7" fill="#d0201a" /><circle cx="5.5" cy="4" r="0.7" fill="#f3b723" />
  {:else}
    <path d="M3 1.5h7l3 3v10H3z" fill="#fff" stroke="#5a6676" stroke-width="0.9" />
  {/if}
</svg>

<style>
  svg {
    display: inline-block;
    vertical-align: middle;
    flex: none;
  }
</style>
