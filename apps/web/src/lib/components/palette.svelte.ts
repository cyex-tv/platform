// Opens the command palette from anywhere: `openPalette()` (the nav's search field does this),
// or Ctrl K / Cmd K / "/" once <CommandPalette /> is mounted (the root layout mounts it).

export const palette = $state({ open: false, query: '' });

export function openPalette(query = '') {
  palette.query = query;
  palette.open = true;
}

/** "⌘K" on Apple keyboards, "Ctrl K" elsewhere (only the hint text differs). */
export function shortcutHint(): string {
  if (typeof navigator === 'undefined') return 'Ctrl K';
  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const p = nav.userAgentData?.platform ?? navigator.platform ?? '';
  return /mac|iphone|ipad|ipod/i.test(p) ? '⌘K' : 'Ctrl K';
}
