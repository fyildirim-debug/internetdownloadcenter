<script lang="ts" module>
  // Uzantıdan dosya türü ve renk/etiket.
  const TYPES: Record<string, { color: string; label: string; kind: string }> = {};
  const reg = (exts: string, color: string, kind: string, label?: string) =>
    exts.split(' ').forEach((e) => (TYPES[e] = { color, kind, label: label ?? e.toUpperCase() }));
  reg('zip 7z rar tar gz bz2 xz zst tgz txz tbz2 lz lzh arj cab ace z sit sitx sea', '#e2a510', 'archive');
  reg('iso img bin dmg', '#8a6bbe', 'disc');
  reg('pdf', '#d3261b', 'doc');
  reg('doc docx odt rtf', '#2b65c4', 'doc');
  reg('xls xlsx ods csv', '#1f8a3b', 'doc');
  reg('ppt pptx pps odp', '#d9611c', 'doc');
  reg('txt md epub log json xml', '#6b737e', 'doc');
  reg('mp3 m4a aac flac wav wma ogg opus aif aiff ra mpa', '#7a3fc2', 'audio');
  reg('mp4 mkv webm avi mov wmv flv m4v mpg mpeg mpe 3gp ogv ts vob rm rmvb asf qt', '#1d6fd0', 'video');
  reg('exe msi msix msu bat cmd', '#3a4b66', 'app');
  reg('apk', '#3ddc84', 'app');
  reg('deb rpm appimage snap flatpakref run sh jar', '#c2410c', 'app');
  reg('jpg jpeg png gif webp bmp svg tif tiff ico heic avif', '#16a34a', 'image');
  reg('torrent', '#4a9a2a', 'doc');
  reg('html htm', '#e8590c', 'web');

  export function fileType(name: string) {
    const base = name.split(/[\\/]/).pop() ?? '';
    const i = base.lastIndexOf('.');
    const ext = i > 0 ? base.slice(i + 1).toLowerCase() : '';
    return TYPES[ext] ?? { color: '#8a94a3', kind: 'file', label: ext.toUpperCase().slice(0, 4) };
  }
</script>

<script lang="ts">
  // Dosya türüne göre renkli sayfa simgesi; 16, 32 ya da 48 piksel.
  let { name, size = 16, media = false }: { name: string; size?: number; media?: boolean } = $props();
  const ft = $derived(media && fileType(name).kind === 'file' ? { color: '#1d6fd0', kind: 'video', label: 'MP4' } : fileType(name));
  const big = $derived(size >= 28);
</script>

<svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
  {#if ft.kind === 'archive'}
    <rect x="4" y="5" width="24" height="23" rx="2" fill={ft.color} stroke="#8a5e05" />
    <rect x="4" y="5" width="24" height="5" fill="#fff" opacity="0.25" />
    <rect x="14" y="5" width="4" height="23" fill="#6b4a0c" opacity="0.45" />
    <g stroke="#fff" stroke-width="1.3"><path d="M14 9h4M14 13h4M14 17h4" /></g>
    <rect x="12.5" y="19" width="7" height="5" rx="1" fill="#fff" stroke="#6b4a0c" />
  {:else if ft.kind === 'disc'}
    <circle cx="16" cy="16" r="13" fill="#d8dde6" stroke="#5a6676" />
    <circle cx="16" cy="16" r="13" fill={ft.color} opacity="0.35" />
    <circle cx="16" cy="16" r="4" fill="#fff" stroke="#5a6676" />
    <path d="M8 9a11 11 0 015-3" stroke="#fff" stroke-width="2" fill="none" opacity="0.8" />
  {:else if ft.kind === 'audio'}
    <path d="M7 2h13l6 6v22H7z" fill="#fff" stroke="#5a6676" />
    <path d="M20 2v6h6" fill="#e5e9ef" stroke="#5a6676" />
    <path d="M13 24V12l9-2v12" fill="none" stroke={ft.color} stroke-width="2" />
    <ellipse cx="11.5" cy="24" rx="3" ry="2.3" fill={ft.color} />
    <ellipse cx="20.5" cy="22" rx="3" ry="2.3" fill={ft.color} />
  {:else if ft.kind === 'video'}
    <rect x="3" y="6" width="26" height="20" rx="2" fill="#2a2f38" />
    <g fill="#e9edf2">
      <rect x="5" y="8" width="2.5" height="2" /><rect x="5" y="13" width="2.5" height="2" /><rect x="5" y="18" width="2.5" height="2" /><rect x="5" y="22.5" width="2.5" height="2" />
      <rect x="24.5" y="8" width="2.5" height="2" /><rect x="24.5" y="13" width="2.5" height="2" /><rect x="24.5" y="18" width="2.5" height="2" /><rect x="24.5" y="22.5" width="2.5" height="2" />
    </g>
    <rect x="9.5" y="8" width="13" height="16" fill={ft.color} />
    <path d="M14 12v8l6-4z" fill="#fff" />
  {:else if ft.kind === 'app'}
    <rect x="3" y="5" width="26" height="22" rx="2" fill="#eef2f7" stroke="#3a4b66" />
    <rect x="3" y="5" width="26" height="5" rx="2" fill={ft.color} />
    <circle cx="16" cy="18.5" r="5" fill="none" stroke={ft.color} stroke-width="2.4" stroke-dasharray="2.6 1.6" />
    <circle cx="16" cy="18.5" r="1.8" fill={ft.color} />
  {:else if ft.kind === 'image'}
    <rect x="3" y="5" width="26" height="22" rx="2" fill="#e8f6ff" stroke="#5a6676" />
    <circle cx="11" cy="12" r="2.6" fill="#f3b723" />
    <path d="M4 25l8-8 5 5 4-4 7 7z" fill={ft.color} />
  {:else}
    <path d="M7 2h13l6 6v22H7z" fill="#fff" stroke="#5a6676" />
    <path d="M20 2v6h6" fill="#e5e9ef" stroke="#5a6676" />
    {#if ft.kind === 'doc'}
      <g stroke="#b9c1cc" stroke-width="1.2"><path d="M10 12h12M10 15h12M10 18h9" /></g>
    {:else if ft.kind === 'web'}
      <circle cx="16.5" cy="15" r="5" fill="none" stroke={ft.color} stroke-width="1.5" />
    {/if}
    {#if ft.label}
      <rect x="4" y="20" width={big ? 22 : 24} height="8" rx="1" fill={ft.color} />
      {#if big}
        <text x="15" y="26.5" text-anchor="middle" font-size="6.5" font-weight="700" fill="#fff" font-family="Segoe UI, Arial, sans-serif">{ft.label}</text>
      {/if}
    {/if}
  {/if}
</svg>

<style>
  svg {
    display: inline-block;
    vertical-align: middle;
    flex: none;
  }
</style>
