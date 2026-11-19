<script lang="ts">
  // The front page: the hero (copy beside the cyex.tv logo, in 3D where the device can take it),
  // then the ways in, one under the other in a zig-zag: the extension, cyex deck, the API and
  // cosmetics. Each has a small picture in the logo's language (ShowcaseArt) in its own mount, a
  // short heading and a sentence or two. No emotes here: browsing them is one click away.
  // Nothing loads before first paint that the page doesn't need.
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import ExtensionMenu from '$lib/home/ExtensionMenu.svelte';
  import LogoHero from '$lib/home/LogoHero.svelte';
  import ShowcaseArt from '$lib/home/ShowcaseArt.svelte';
  import { reveal } from '$lib/home/reveal';
  import { deck, refreshDeck } from '$lib/home/deck.svelte';

  onMount(() => {
    refreshDeck();
  });
</script>

<svelte:head>
  <title>cyex.tv · Emotes for Twitch and Kick chat</title>
  <meta name="description" content="Upload emotes, put them in your channel's set, and see them in chat on Twitch and Kick." />
</svelte:head>

<ColumnLayout top>
  <section class="hero" aria-labelledby="hero-title">
    <div class="copy">
      <h1 id="hero-title">Emotes for every chat</h1>
      <p class="hero-lead">Upload your own, put them in your channel's set, and chat sees them on Twitch and Kick.</p>
      <div class="actions">
        <a class="btn btn-primary flat" href="/search">Explore emotes <span class="arrow"><Icon name="arrow-right" /></span></a>
        <a class="btn" href="#start">Get the extension</a>
      </div>
    </div>
    <div class="art"><LogoHero /></div>
  </section>

  <section class="block" id="start" aria-labelledby="start-title">
    <div class="block-head"><h2 class="block-title" id="start-title">Where to start</h2></div>
    <ul class="shows">
      <li class="show" use:reveal>
        <div class="mount"><ShowcaseArt kind="extension" /></div>
        <div class="text">
          <h3>Browser extension</h3>
          <p>Adds cyex.tv emotes, badges and name paints to Twitch and Kick chat on the web. Install it once and every channel's set shows up as you chat.</p>
          <div class="foot">
            <ExtensionMenu />
            <p class="fine">Chrome, Edge, Firefox, Opera and Brave · In early testing</p>
          </div>
        </div>
      </li>
      <li class="show" use:reveal>
        <div class="mount"><ShowcaseArt kind="deck" /></div>
        <div class="text">
          <h3>cyex deck</h3>
          <p>A light Twitch app for Windows from the cyex family. cyex.tv emotes come to it in an update, with nothing to set up.</p>
          <div class="foot">
            <a class="btn btn-s" href={deck.url}><Icon name="download" size={16} />Download for Windows</a>
            <p class="fine">Version {deck.version}{deck.prerelease ? ' · Pre-release' : ''} · <a class="quiet" href="https://cyex.app/deck">About cyex deck</a></p>
          </div>
        </div>
      </li>
      <li class="show" use:reveal>
        <div class="mount"><ShowcaseArt kind="api" /></div>
        <div class="text">
          <h3>Open API</h3>
          <p>GraphQL and REST for bots, overlays and chat clients. Public data needs no key, so your first request is a copy and paste away.</p>
          <div class="foot">
            <a class="btn btn-s" href="/developers">Read the guide <span class="arrow"><Icon name="arrow-right" size={16} /></span></a>
            <p class="fine"><a class="quiet" href="/developers/reference">API reference</a></p>
          </div>
        </div>
      </li>
      <li class="show" use:reveal>
        <div class="mount"><ShowcaseArt kind="cosmetics" /></div>
        <div class="text">
          <h3>Badges and paints</h3>
          <p>A badge beside your name and a paint on it, seen by everyone in chat with cyex.tv. Pick yours in your account and change it whenever you like.</p>
          <div class="foot">
            <a class="btn btn-s" href="/cosmetics">See cosmetics <span class="arrow"><Icon name="arrow-right" size={16} /></span></a>
            <p class="fine">Or open the <a class="quiet" href="/shop">shop</a></p>
          </div>
        </div>
      </li>
    </ul>
  </section>
</ColumnLayout>

<style>
  /* Hero: one surface mount, copy beside the logo (the logo above the copy on phones). */
  .hero { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 460px); align-items: center; gap: 24px 48px;
    padding: clamp(24px, 4vw, 64px); border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface);
    overflow: hidden; }
  .hero h1 { font-size: clamp(38px, 4.8vw, 68px); line-height: 1; letter-spacing: -.04em; max-width: 12ch; }
  .hero-lead { margin: 16px 0 0; font-size: var(--t-lead); line-height: 1.5; color: var(--muted); max-width: 42ch; }
  .actions { margin-top: 28px; }
  .actions .btn { height: 44px; }
  /* The logo carries the page's one glow, so the call to action stays flat. */
  .flat, .flat:hover { box-shadow: none; }
  .art { width: 100%; justify-self: end; }
  @media (max-width: 1099.98px) { .hero { grid-template-columns: minmax(0, 1fr) minmax(0, 340px); gap: 24px 32px; } }
  @media (max-width: 699.98px) {
    .hero { grid-template-columns: minmax(0, 1fr); }
    .art { order: -1; justify-self: center; max-width: 240px; }
  }
  @media (max-width: 520px) {
    .hero-lead { font-size: 16px; }
    .actions { margin-top: 20px; grid-template-columns: minmax(0, 1fr); }
  }
  .arrow { display: inline-flex; }

  .block { margin-top: clamp(40px, 4vw, 56px); scroll-margin-top: calc(var(--nav-h) + 20px); }

  /* The ways in: one under the other, the picture and the text trading sides each row. */
  .shows { list-style: none; margin: 0; padding: 0; display: grid; gap: clamp(32px, 4.5vw, 64px); }
  .show { display: grid; grid-template-columns: minmax(0, 5fr) minmax(0, 6fr); align-items: center; gap: 24px clamp(32px, 5vw, 72px); }
  .show:nth-child(even) { grid-template-columns: minmax(0, 6fr) minmax(0, 5fr); }
  .show:nth-child(even) .mount { order: 2; }
  .mount { display: grid; place-items: center; aspect-ratio: 2 / 1; min-width: 0;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  /* The same picture in every mount, at the same size. */
  .mount :global(.art) { width: clamp(150px, 42%, 252px); height: auto; }
  .text { min-width: 0; max-width: 46ch; }
  .show:nth-child(even) .text { justify-self: end; }
  .show h3 { font-size: var(--t-h3-sub); line-height: 1.1; letter-spacing: -.03em; text-wrap: balance; }
  .text > p { margin: 14px 0 0; font-size: var(--t-body); line-height: 1.6; color: var(--muted); text-wrap: pretty; }
  .foot { display: grid; justify-items: start; gap: 10px; margin-top: 24px; }
  .fine { margin: 0; font-size: var(--t-fine); color: var(--dim); }
  .quiet { color: var(--muted); text-decoration: underline; text-underline-offset: 3px; }
  @media (hover: hover) and (pointer: fine) { .quiet:hover { color: var(--text); } }

  /* Phones: one column, the picture above its text in every row. */
  @media (max-width: 719.98px) {
    .show, .show:nth-child(even) { grid-template-columns: minmax(0, 1fr); }
    .show:nth-child(even) .mount { order: 0; }
    .show:nth-child(even) .text { justify-self: start; }
    .mount :global(.art) { width: clamp(140px, 44%, 200px); }
    .text > p { font-size: 16px; }
  }

  /* Reveal once on scroll (reveal.ts marks a row while it waits). Under reduced motion: none. */
  @media (prefers-reduced-motion: no-preference) {
    .show:global([data-reveal]) > * { transition: opacity var(--dur-reveal) var(--ease), translate var(--dur-reveal) var(--ease); }
    .show:global([data-reveal]) > .text { transition-delay: var(--stagger); }
    .show:global([data-reveal='wait']) > * { opacity: 0; translate: 0 28px; }
  }
  :global(html[data-motion='reduce']) .show > * { transition: none; opacity: 1; translate: none; }
</style>
