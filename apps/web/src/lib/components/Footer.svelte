<script lang="ts">
  // The one footer, on every route: slim, in the full-width wrapper. Desktop: the mark and the product,
  // developer and legal links in one row, then the non-affiliation note and the year. Phones: the
  // links wrap, the note stays under them. It never changes with the session or the route.
  // `slim` (the docs at /developers, from the page's load) also drops the gap above it, so a long
  // page ends where its content does instead of running into the footer.
  import Mark from './Mark.svelte';
  import { openShortcuts } from '$lib/shell/shortcuts.svelte';

  let { slim = false }: { slim?: boolean } = $props();
  const year = new Date().getFullYear();

  const links = [
    { href: '/search', label: 'Browse emotes' },
    { href: '/download', label: 'Download' },
    { href: '/shop', label: 'Shop' },
    { href: '/help', label: 'Help' },
    { href: '/guidelines', label: 'Guidelines' },
    { href: '/developers', label: 'Developers' },
    { href: '/changelog', label: 'Changelog' },
    { href: 'https://github.com/cyex-tv/platform', label: 'Source' },
    // Terms, Privacy and Contact come back when their pages exist.
    { href: 'https://github.com/cyex-tv/platform/blob/main/docs/takedown.md', label: 'Takedowns' },
  ];
</script>

<footer class="footer" class:slim>
  <div class="wrap">
    <div class="row">
      <a class="brand" href="/"><Mark size={16} /><span>cyex.tv</span></a>
      <nav aria-label="Footer">
        {#each links as l (l.href)}<a href={l.href}>{l.label}</a>{/each}
        <!-- Keyboard shortcuts: also `?` anywhere outside a field. Not on touch-only screens. -->
        <button class="keys" type="button" aria-haspopup="dialog" onclick={openShortcuts}>Keyboard shortcuts</button>
      </nav>
    </div>
    <p class="base">
      <span>Not affiliated with, endorsed by, or sponsored by Twitch or Kick.</span>
      <span>© {year} cyex</span>
    </p>
  </div>
</footer>

<style>
  .footer { flex: none; margin-top: var(--footer-gap); border-top: 1px solid var(--border);
    padding-bottom: env(safe-area-inset-bottom, 0px); font-size: var(--t-fine); color: var(--muted); }
  .slim { margin-top: 0; }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 28px; padding-top: 14px; }
  .brand { display: inline-flex; align-items: center; gap: 8px; min-height: 40px; font-size: var(--t-small); font-weight: 600;
    letter-spacing: -.01em; color: var(--text); }
  nav { display: flex; flex-wrap: wrap; gap: 0 20px; }
  nav a, .keys { display: inline-flex; align-items: center; min-height: 40px; color: var(--muted); transition: color var(--fast); }
  nav a:active, .keys:active { opacity: .6; transition: none; }
  .keys { padding: 0; border: 0; background: none; font: inherit; cursor: pointer; }
  @media (hover: none) and (pointer: coarse) { .keys { display: none; } }
  .base { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 4px 24px; margin: 0; padding: 6px 0 20px; color: var(--dim); }
  @media (hover: hover) and (pointer: fine) { nav a:hover, .keys:hover { color: var(--text); } }
  /* Touch: 44px to hit; short words like Help get their width from a hit area that reaches into the gap. */
  @media (pointer: coarse) { .brand, nav a { min-height: 44px; } nav a { position: relative; } nav a::after { content: ''; position: absolute; inset: 0 -8px; } }
  @media (max-width: 720px) { nav { gap: 0 16px; } .row { gap: 0; } }
</style>
