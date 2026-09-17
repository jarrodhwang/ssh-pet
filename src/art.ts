export function icon(name: string, size = 20): string {
  const paths: Record<string, string> = {
    terminal: '<rect x="3" y="4" width="18" height="16" rx="4"/><path d="m7 9 3 3-3 3m6 0h4"/>',
    drop: '<path d="M12 3C10 7 5 11 5 15a7 7 0 0 0 14 0c0-4-5-8-7-12Z"/><path d="M9 15h.01M15 15h.01m-5 2a3 3 0 0 0 4 0"/>',
    sliders: '<path d="M4 7h7m4 0h5M4 17h3m4 0h9"/><circle cx="13" cy="7" r="2"/><circle cx="9" cy="17" r="2"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    arrow: '<path d="M6 18 18 6M7 6h11v11"/>',
    star: '<path d="m12 3 2.8 5.7 6.3.9-4.6 4.5 1.1 6.3-5.6-3-5.6 3 1.1-6.3L3 9.6l6.2-.9Z"/>',
    edit: '<path d="m15 5 4 4M4 20l5-1L20 8a2.8 2.8 0 0 0-4-4L5 15Z"/>',
    close: '<path d="m6 6 12 12M6 18 18 6"/>',
    search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 4 4"/>',
    key: '<circle cx="8" cy="9" r="4"/><path d="m11 12 9 9m-5-5 3-3m-1 5 3-3"/>',
    check: '<path d="m5 12 4 4L19 6"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10h.01"/>',
    screen: '<rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/>',
    moon: '<path d="M20 14a8 8 0 0 1-10-10A8.5 8.5 0 1 0 20 14Z"/>',
    login: '<path d="M10 4H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h4m6-14 6 6-6 6m6-6H9"/>',
    reset: '<path d="M4 10a8 8 0 1 1 1 8M4 4v6h6"/>',
    shield: '<path d="m12 3 8 3v6c0 4-4 7-8 9-4-2-8-5-8-9V6Z"/><path d="m8 12 3 3 5-6"/>',
    chevron: '<path d="m9 5 7 7-7 7"/>',
    heart: '<path d="M12 20 4 12a5 5 0 0 1 8-6 5 5 0 0 1 8 6Z"/>',
  };
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] ?? paths.drop}</svg>`;
}

let sequence = 0;
export function droplet(className = ''): string {
  const id = `water-${sequence++}`;
  return `<svg class="droplet-art ${className}" viewBox="0 0 220 230" fill="none" aria-hidden="true">
    <defs>
      <linearGradient id="${id}-body" x1="66" y1="48" x2="159" y2="194" gradientUnits="userSpaceOnUse"><stop stop-color="#c0f2ff"/><stop offset=".4" stop-color="#7cd7f4"/><stop offset="1" stop-color="#399bd9"/></linearGradient>
      <radialGradient id="${id}-shine" cx="0" cy="0" r="1" gradientTransform="translate(85 115) rotate(57) scale(82 69)" gradientUnits="userSpaceOnUse"><stop stop-color="#ddfaff" stop-opacity=".8"/><stop offset="1" stop-color="#b0edff" stop-opacity="0"/></radialGradient>
      <linearGradient id="${id}-edge" x1="79" y1="57" x2="156" y2="196" gradientUnits="userSpaceOnUse"><stop stop-color="white" stop-opacity=".9"/><stop offset="1" stop-color="#177bbd" stop-opacity=".25"/></linearGradient>
      <radialGradient id="${id}-shadow"><stop stop-color="#3878a4" stop-opacity=".19"/><stop offset="1" stop-color="#3878a4" stop-opacity="0"/></radialGradient>
    </defs>
    <ellipse class="drop-shadow" cx="110" cy="211" rx="65" ry="12" fill="url(#${id}-shadow)"/>
    <g class="drop-body">
      <path d="M111 25C106 59 54 84 45 126c-10 45 17 75 63 77 42 2 73-24 71-62-2-38-42-65-54-96-3-7-5-14-5-18-1-8-7-10-9-2Z" fill="url(#${id}-body)" stroke="url(#${id}-edge)" stroke-width="2"/>
      <path d="M111 25C106 59 54 84 45 126c-10 45 17 75 63 77 42 2 73-24 71-62-2-38-42-65-54-96-3-7-5-14-5-18-1-8-7-10-9-2Z" fill="url(#${id}-shine)"/>
      <path d="M72 106c-8 10-12 19-13 27" stroke="white" stroke-opacity=".7" stroke-width="9" stroke-linecap="round"/>
      <ellipse cx="84" cy="89" rx="4" ry="6" transform="rotate(35 84 89)" fill="white" fill-opacity=".7"/>
      <path d="M60 171c12 20 52 26 78 15" stroke="#bcf0ff" stroke-opacity=".3" stroke-width="4" stroke-linecap="round"/>
      <g class="drop-face">
        <g class="drop-eyes"><ellipse cx="88" cy="146" rx="5" ry="7" fill="#245a7b"/><ellipse cx="137" cy="146" rx="5" ry="7" fill="#245a7b"/><circle cx="89" cy="143.5" r="1.5" fill="white"/><circle cx="138" cy="143.5" r="1.5" fill="white"/></g>
        <ellipse cx="74" cy="158" rx="10" ry="5.5" fill="#b6e7f7" fill-opacity=".65"/><ellipse cx="152" cy="158" rx="10" ry="5.5" fill="#b6e7f7" fill-opacity=".65"/>
        <path d="M105 158q7 8 14 0" stroke="#245a7b" stroke-width="3" stroke-linecap="round"/>
      </g>
    </g>
  </svg>`;
}

export const escapeHtml = (value: unknown): string => String(value).replace(/[&<>"']/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]!);
