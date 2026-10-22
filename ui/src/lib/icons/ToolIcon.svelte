<script lang="ts" module>
  let seq = 0;
</script>

<script lang="ts">
  // Araç çubuğu için büyük, gölgeli, 3B görünümlü simgeler (48×48 çizim alanı).
  let { name, size = 40, disabled = false }: { name: string; size?: number; disabled?: boolean } = $props();
  const u = `ti${++seq}`;
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 48 48"
  class:disabled
  aria-hidden="true"
  xmlns="http://www.w3.org/2000/svg"
>
  <defs>
    <radialGradient id="{u}-blue" cx="35%" cy="30%" r="75%">
      <stop offset="0" stop-color="#bfe6ff" />
      <stop offset="0.45" stop-color="#3a9be8" />
      <stop offset="1" stop-color="#0b4f9c" />
    </radialGradient>
    <linearGradient id="{u}-green" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#9ff07a" />
      <stop offset="0.5" stop-color="#38b52a" />
      <stop offset="1" stop-color="#1c7414" />
    </linearGradient>
    <linearGradient id="{u}-red" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ff9a8a" />
      <stop offset="0.5" stop-color="#e12a1c" />
      <stop offset="1" stop-color="#93100a" />
    </linearGradient>
    <linearGradient id="{u}-gray" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" />
      <stop offset="0.5" stop-color="#c9ced6" />
      <stop offset="1" stop-color="#7d8592" />
    </linearGradient>
    <linearGradient id="{u}-gold" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff2a8" />
      <stop offset="0.5" stop-color="#f3b723" />
      <stop offset="1" stop-color="#b06d06" />
    </linearGradient>
    <linearGradient id="{u}-paper" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ffffff" />
      <stop offset="1" stop-color="#d9dee6" />
    </linearGradient>
    <linearGradient id="{u}-shine" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff" stop-opacity="0.85" />
      <stop offset="1" stop-color="#fff" stop-opacity="0" />
    </linearGradient>
    <filter id="{u}-sh" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0.8" dy="1.4" stdDeviation="1.1" flood-color="#000" flood-opacity="0.35" />
    </filter>
  </defs>

  <g filter="url(#{u}-sh)">
    {#if name === 'addUrl'}
      <circle cx="22" cy="22" r="17" fill="url(#{u}-blue)" stroke="#0a3f7d" stroke-width="1" />
      <g fill="none" stroke="#e7f5ff" stroke-width="1.3" opacity="0.85">
        <ellipse cx="22" cy="22" rx="7.5" ry="17" />
        <line x1="5" y1="22" x2="39" y2="22" />
        <path d="M8 13 Q22 17 36 13" />
        <path d="M8 31 Q22 27 36 31" />
      </g>
      <ellipse cx="17" cy="12" rx="9" ry="5" fill="url(#{u}-shine)" />
      <circle cx="36" cy="36" r="9.5" fill="url(#{u}-green)" stroke="#145c0e" stroke-width="1" />
      <path d="M36 30.5v11M30.5 36h11" stroke="#fff" stroke-width="3.2" stroke-linecap="round" />
    {:else if name === 'resume'}
      <rect x="9" y="33" width="30" height="8" rx="2" fill="url(#{u}-gray)" stroke="#5c6470" stroke-width="1" />
      <path d="M17 5h14v15h8L24 35 9 20h8z" fill="url(#{u}-green)" stroke="#145c0e" stroke-width="1.2" stroke-linejoin="round" />
      <path d="M19 7h10v4H19z" fill="url(#{u}-shine)" />
    {:else if name === 'stop'}
      <path
        d="M17 5h14l10 10v14L31 39H17L7 29V15z"
        fill="url(#{u}-red)"
        stroke="#7a0b06"
        stroke-width="1.2"
        stroke-linejoin="round"
      />
      <rect x="14" y="19" width="20" height="6.5" rx="1.5" fill="#fff" />
      <path d="M18 7h12l6 6H12z" fill="url(#{u}-shine)" />
    {:else if name === 'stopAll'}
      <path
        d="M21 3h11l8 8v11l-8 8H21l-8-8V11z"
        fill="url(#{u}-red)"
        stroke="#7a0b06"
        stroke-width="1"
        stroke-linejoin="round"
        opacity="0.8"
      />
      <path
        d="M15 13h13l9 9v13l-9 9H15l-9-9V22z"
        fill="url(#{u}-red)"
        stroke="#7a0b06"
        stroke-width="1.2"
        stroke-linejoin="round"
      />
      <rect x="12" y="25.5" width="19" height="6" rx="1.5" fill="#fff" />
      <path d="M16 15h11l5 5H11z" fill="url(#{u}-shine)" />
    {:else if name === 'delete'}
      <path d="M11 13h26l-2.5 29H13.5z" fill="url(#{u}-gray)" stroke="#4d5560" stroke-width="1.1" />
      <rect x="8" y="8" width="32" height="6" rx="2" fill="url(#{u}-gray)" stroke="#4d5560" stroke-width="1.1" />
      <rect x="19" y="4.5" width="10" height="4" rx="1.5" fill="none" stroke="#4d5560" stroke-width="1.6" />
      <g stroke="#6b737e" stroke-width="1.4"><line x1="18" y1="18" x2="18.6" y2="38" /><line x1="24" y1="18" x2="24" y2="38" /><line x1="30" y1="18" x2="29.4" y2="38" /></g>
      <path d="M27 25l13 13M40 25L27 38" stroke="#b10f07" stroke-width="5.5" stroke-linecap="round" />
      <path d="M27 25l13 13M40 25L27 38" stroke="#ff4a3a" stroke-width="3" stroke-linecap="round" />
    {:else if name === 'deleteCompleted'}
      <path d="M9 4h20l9 9v31H9z" fill="url(#{u}-paper)" stroke="#5c6470" stroke-width="1.1" />
      <path d="M29 4v9h9" fill="#cfd5de" stroke="#5c6470" stroke-width="1.1" />
      <path d="M13 24l6 6 11-13" fill="none" stroke="#1c7414" stroke-width="5" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M13 24l6 6 11-13" fill="none" stroke="#4fd23a" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" />
      <circle cx="36" cy="37" r="8.5" fill="url(#{u}-red)" stroke="#7a0b06" />
      <path d="M32.5 33.5l7 7M39.5 33.5l-7 7" stroke="#fff" stroke-width="2.6" stroke-linecap="round" />
    {:else if name === 'options'}
      <g transform="translate(24 24)">
        <path
          d="M-3.5-19h7l1 5.2 3.6 1.5 4.4-3 5 5-3 4.4 1.5 3.6 5.2 1v7l-5.2 1-1.5 3.6 3 4.4-5 5-4.4-3-3.6 1.5-1 5.2h-7l-1-5.2-3.6-1.5-4.4 3-5-5 3-4.4-1.5-3.6-5.2-1v-7l5.2-1 1.5-3.6-3-4.4 5-5 4.4 3 3.6-1.5z"
          fill="url(#{u}-gray)"
          stroke="#4d5560"
          stroke-width="1.1"
          stroke-linejoin="round"
        />
        <circle r="7.5" fill="url(#{u}-blue)" stroke="#0a3f7d" />
        <circle r="3" fill="#e8f4ff" />
      </g>
    {:else if name === 'scheduler'}
      <circle cx="24" cy="25" r="18" fill="url(#{u}-blue)" stroke="#0a3f7d" stroke-width="1.1" />
      <circle cx="24" cy="25" r="14.5" fill="url(#{u}-paper)" stroke="#5c6470" stroke-width="0.8" />
      <g stroke="#384150" stroke-width="1.6" stroke-linecap="round">
        <line x1="24" y1="12.5" x2="24" y2="15" /><line x1="24" y1="35" x2="24" y2="37.5" /><line x1="11.5" y1="25" x2="14" y2="25" /><line x1="34" y1="25" x2="36.5" y2="25" />
      </g>
      <path d="M24 25V16.5M24 25l7 4" stroke="#1f2733" stroke-width="2.4" stroke-linecap="round" />
      <circle cx="24" cy="25" r="1.8" fill="#d0201a" />
      <ellipse cx="18" cy="13" rx="8" ry="4" fill="url(#{u}-shine)" />
    {:else if name === 'startQueue' || name === 'stopQueue'}
      <rect x="6" y="5" width="28" height="34" rx="2" fill="url(#{u}-paper)" stroke="#5c6470" stroke-width="1.1" />
      <g stroke="#3a9be8" stroke-width="3" stroke-linecap="round">
        <line x1="11" y1="12" x2="28" y2="12" /><line x1="11" y1="19" x2="28" y2="19" /><line x1="11" y1="26" x2="22" y2="26" />
      </g>
      {#if name === 'startQueue'}
        <circle cx="35" cy="35" r="10" fill="url(#{u}-green)" stroke="#145c0e" />
        <path d="M32 29.5v11l9-5.5z" fill="#fff" />
      {:else}
        <circle cx="35" cy="35" r="10" fill="url(#{u}-red)" stroke="#7a0b06" />
        <rect x="30.5" y="30.5" width="9" height="9" rx="1" fill="#fff" />
      {/if}
    {:else if name === 'grabber'}
      <g stroke="#6b737e" stroke-width="1.1" fill="none">
        <circle cx="20" cy="20" r="15" />
        <circle cx="20" cy="20" r="10" />
        <circle cx="20" cy="20" r="5" />
        <line x1="20" y1="3" x2="20" y2="37" /><line x1="3" y1="20" x2="37" y2="20" />
        <line x1="8" y1="8" x2="32" y2="32" /><line x1="32" y1="8" x2="8" y2="32" />
      </g>
      <circle cx="30" cy="30" r="9" fill="url(#{u}-blue)" stroke="#0a3f7d" stroke-width="1.2" opacity="0.92" />
      <ellipse cx="27.5" cy="26.5" rx="5" ry="3" fill="url(#{u}-shine)" />
      <path d="M36.5 36.5l7 7" stroke="#7a4a05" stroke-width="6" stroke-linecap="round" />
      <path d="M36.5 36.5l7 7" stroke="url(#{u}-gold)" stroke-width="4" stroke-linecap="round" />
    {:else if name === 'video'}
      <rect x="4" y="9" width="36" height="28" rx="3" fill="#2a2f38" stroke="#111" stroke-width="1" />
      <g fill="#e9edf2">
        <rect x="7" y="12" width="4" height="3" rx="0.6" /><rect x="7" y="19" width="4" height="3" rx="0.6" /><rect x="7" y="26" width="4" height="3" rx="0.6" /><rect x="7" y="32" width="4" height="2.5" rx="0.6" />
        <rect x="33" y="12" width="4" height="3" rx="0.6" /><rect x="33" y="19" width="4" height="3" rx="0.6" /><rect x="33" y="26" width="4" height="3" rx="0.6" /><rect x="33" y="32" width="4" height="2.5" rx="0.6" />
      </g>
      <rect x="13" y="12" width="18" height="22" fill="url(#{u}-blue)" />
      <path d="M19 17.5v11l9-5.5z" fill="#fff" />
      <circle cx="37" cy="37" r="9" fill="url(#{u}-green)" stroke="#145c0e" />
      <path d="M37 32v8.5M33 37l4 4 4-4" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" fill="none" />
    {:else if name === 'app'}
      <circle cx="24" cy="24" r="20" fill="url(#{u}-blue)" stroke="#0a3f7d" stroke-width="1.2" />
      <ellipse cx="18" cy="12" rx="11" ry="6" fill="url(#{u}-shine)" />
      <path d="M19 9h10v14h7L24 37 12 23h7z" fill="url(#{u}-green)" stroke="#145c0e" stroke-width="1.2" stroke-linejoin="round" />
      <rect x="12" y="38" width="24" height="3" rx="1.5" fill="#e8f4ff" opacity="0.9" />
    {/if}
  </g>
</svg>

<style>
  svg {
    display: block;
    overflow: visible;
  }
  .disabled {
    filter: grayscale(1) opacity(0.45);
  }
</style>
