<script lang="ts">
  // The site footer: brand, three link columns (product, developers, legal) and the base line
  // with the non-affiliation note. /terms, /privacy and /contact are their own pages. Signed in,
  // the product column offers Your emotes; signed out, Sign in.
  import Mark from './Mark.svelte';

  // `slim` is one line (the non-affiliation note, the year, three links) for pages that scroll a long
  // way and should end where their content does, like /developers.
  let { signedIn = false, slim = false }: { signedIn?: boolean; slim?: boolean } = $props();
  const year = new Date().getFullYear();

  const columns = $derived([
    {
      name: 'Product',
      links: [
        { href: '/search', label: 'Browse emotes' },
        { href: '/#install', label: 'Get the extension' },
        signedIn ? { href: '/me', label: 'Your emotes' } : { href: '/signin', label: 'Sign in' },
      ],
    },
    {
      name: 'Developers',
      links: [
        { href: '/developers', label: 'Developers' },
        { href: '/developers/reference', label: 'API reference' },
        { href: 'https://github.com/cyex-tv/platform', label: 'Source' },
      ],
    },
    {
      name: 'Legal',
      links: [
        { href: '/terms', label: 'Terms' },
        { href: '/privacy', label: 'Privacy' },
        { href: 'https://github.com/cyex-tv/platform/blob/main/docs/takedown.md', label: 'Takedowns' },
        { href: '/contact', label: 'Contact' },
      ],
    },
  ]);
</script>

{#if slim}
  <footer class="footer slim">
    <div class="page">
      <p>
        <span>Not affiliated with, endorsed by, or sponsored by Twitch or Kick.</span>
        <span class="dot" aria-hidden="true">·</span>
        <span>© {year} cyex</span>
      </p>
      <nav aria-label="Legal">
        <a href="/terms">Terms</a><a href="/privacy">Privacy</a><a href="/contact">Contact</a>
      </nav>
    </div>
  </footer>
{:else}
<footer class="footer">
  <div class="page">
    <div class="top">
      <div class="about">
        <a class="brand" href="/"><Mark size={18} /><span>cyex.tv</span></a>
        <p>Emotes for Twitch and Kick chat.</p>
      </div>
      {#each columns as c}
        <nav class="col" aria-label={c.name}>
          <h2>{c.name}</h2>
          <ul>
            {#each c.links as l}<li><a href={l.href}>{l.label}</a></li>{/each}
          </ul>
        </nav>
      {/each}
    </div>
    <div class="base">
      <span>Not affiliated with, endorsed by, or sponsored by Twitch or Kick.</span>
      <span>© {year} cyex</span>
    </div>
  </div>
</footer>
{/if}

<style>
  .footer { flex: none; border-top: 1px solid var(--border); margin-top: var(--footer-gap);
    padding-bottom: env(safe-area-inset-bottom, 0px); }
  .top { display: grid; grid-template-columns: minmax(0, 1.4fr) repeat(3, minmax(0, 1fr)); gap: 32px 24px; padding: 48px 0 40px; }
  .brand { display: inline-flex; align-items: center; gap: 9px; font-size: 16px; font-weight: 600; letter-spacing: -.01em; }
  .about p { margin: 12px 0 0; font-size: var(--t-small); color: var(--muted); max-width: 28ch; }
  h2 { margin: 0 0 12px; font-size: var(--t-small); font-weight: 500; color: var(--text); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 0; }
  .col a { display: inline-flex; align-items: center; min-height: 40px; font-size: var(--t-small); color: var(--muted);
    transition: color var(--dur-color); }
  .base { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 8px 24px; border-top: 1px solid var(--border);
    padding: 20px 0 36px; font-size: var(--t-fine); color: var(--muted); }
  .slim { margin-top: 0; }
  .slim .page { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 4px 24px; padding-block: 10px; }
  .slim p { margin: 0; display: flex; flex-wrap: wrap; gap: 0 8px; font-size: var(--t-fine); color: var(--muted); }
  .slim nav { display: flex; gap: 4px; }
  .slim nav a { display: inline-flex; align-items: center; min-height: 40px; padding: 0 8px; font-size: var(--t-fine); color: var(--muted);
    transition: color var(--dur-color); }
  @media (hover: hover) and (pointer: fine) { .slim nav a:hover { color: var(--text); } }
  @media (hover: hover) and (pointer: fine) { .col a:hover { color: var(--text); } }
  @media (max-width: 900px) { .top { grid-template-columns: repeat(3, minmax(0, 1fr)); } .about { grid-column: 1 / -1; } }
  @media (max-width: 520px) { .top { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
