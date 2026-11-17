// Light or dark, one state for every control that shows or switches it (the header toggle and
// the theme row in the account menu and the phone menu). Saved in localStorage (cyexTheme), read
// by app.html's inline script before first paint, cross-faded with a view transition unless
// reduced motion is on (read at the moment it runs, cyex-design).

export const theme = $state({ light: false });

/** Reads the theme the inline script applied (call once mounted). */
export function syncTheme() {
  theme.light = document.documentElement.dataset.theme === 'light';
}

function apply() {
  theme.light = !theme.light;
  const root = document.documentElement;
  if (theme.light) root.dataset.theme = 'light';
  else delete root.dataset.theme;
  try {
    localStorage.setItem('cyexTheme', theme.light ? 'light' : 'dark');
  } catch {}
  const bg = getComputedStyle(root).getPropertyValue('--bg').trim();
  document.querySelectorAll('meta[name="theme-color"]').forEach((m) => m.setAttribute('content', bg));
}

export function toggleTheme() {
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
  if (!still && doc.startViewTransition) doc.startViewTransition(apply);
  else apply();
}
