// Opens the command palette from anywhere: `openPalette()` (the nav's search field does this),
// or Ctrl K / Cmd K / "/" (no on-screen hint; the shortcut just works) once <CommandPalette /> is mounted (the root layout mounts it).

export const palette = $state({ open: false, query: '' });

export function openPalette(query = '') {
  palette.query = query;
  palette.open = true;
}
