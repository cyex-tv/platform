// Light or dark, one state for every control that shows or switches it (the header toggle and
// the theme row in the account menu and the phone menu). Saved in localStorage (cyexTheme), read
// by app.html's inline script before first paint, cross-faded with a view transition unless
// reduced motion is on (read at the moment it runs, cyex-design).

import { now, stillNow } from '$lib/media.svelte';

export const theme = $state({ light: false });

/** Reads the theme the inline script applied (call once mounted). */
export function syncTheme() {
  theme.light = document.documentElement.dataset.theme === 'light';
  pinFavicon();
}

/** The tab icon follows the site's theme: favicon.svg follows the OS, so a different site theme pins its own. */
function pinFavicon() {
  const link = document.querySelector<HTMLLinkElement>('link[rel="icon"][type="image/svg+xml"]');
  if (!link) return;
  const os = now('(prefers-color-scheme: light)');
  const file = os === theme.light ? 'favicon.svg' : theme.light ? 'favicon-light.svg' : 'favicon-dark.svg';
  link.href = new URL(`/${file}`, location.href).href;
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
  pinFavicon();
}

export function toggleTheme() {
  const still = stillNow();
  const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
  if (!still && doc.startViewTransition) {
    const root = document.documentElement;
    root.classList.add('theme-switching');
    const t = doc.startViewTransition(apply) as { finished?: Promise<unknown> };
    const done = () => root.classList.remove('theme-switching');
    if (t.finished) void t.finished.then(done, done);
    else done();
  } else apply();
}
