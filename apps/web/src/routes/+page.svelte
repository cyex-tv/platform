<script lang="ts">
  // The front page: the hero (copy beside the cyex.tv logo, in 3D where the device can take it)
  // and one row of boxes for the ways in: the extension, cyex deck, the API and cosmetics. Each
  // box has a small picture in the logo's language (ShowcaseArt). No emotes here: browsing them is
  // one click away. Nothing loads before first paint that the page doesn't need.
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import ExtensionMenu from '$lib/home/ExtensionMenu.svelte';
  import LogoHero from '$lib/home/LogoHero.svelte';
  import ShowcaseArt from '$lib/home/ShowcaseArt.svelte';
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
      <li class="show">
        <ShowcaseArt kind="extension" />
        <h3>Browser extension</h3>
        <p>Emotes, badges and name paints in Twitch and Kick chat on the web. In early testing.</p>
        <div class="foot">
          <ExtensionMenu />
          <p class="fine">Chrome, Edge, Firefox, Opera and Brave</p>
        </div>
      </li>
      <li class="show">
        <ShowcaseArt kind="deck" />
        <h3>cyex deck</h3>
        <p>A light Twitch app for Windows. cyex.tv emotes come to it in an update.</p>
        <div class="foot">
          <a class="btn btn-s" href={deck.url}><Icon name="download" size={16} />Download for Windows</a>
          <p class="fine">Version {deck.version}{deck.prerelease ? ' · Pre-release' : ''} · <a class="quiet" href="https://cyex.app/deck">About cyex deck</a></p>
        </div>
      </li>
      <li class="show">
        <ShowcaseArt kind="api" />
        <h3>Open API</h3>
        <p>GraphQL and REST for bots, overlays and chat clients. Public data needs no key.</p>
        <div class="foot">
          <a class="btn btn-s" href="/developers">Read the guide <span class="arrow"><Icon name="arrow-right" size={16} /></span></a>
          <p class="fine"><a class="quiet" href="/developers/reference">API reference</a></p>
        </div>
      </li>
      <li class="show">
        <ShowcaseArt kind="cosmetics" />
        <h3>Badges and paints</h3>
        <p>A badge beside your name and a paint on it, seen by everyone in chat with cyex.tv.</p>
        <div class="foot">
          <a class="btn btn-s" href="/cosmetics">See cosmetics <span class="arrow"><Icon name="arrow-right" size={16} /></span></a>
          <p class="fine">Or open the <a class="quiet" href="/shop">shop</a></p>
        </div>
      </li>
    </ul>
  </section>
</ColumnLayout>

<style>
  /* Hero: one surface mount, copy beside the logo (the logo above the copy on phones). */
  .hero { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 420px); align-items: center; gap: 24px 48px;
    padding: clamp(24px, 4vw, 64px); border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface);
    overflow: hidden; }
  .hero h1 { font-size: clamp(38px, 4.8vw, 68px); line-height: 1; letter-spacing: -.04em; max-width: 12ch; }
  .hero-lead { margin: 16px 0 0; font-size: var(--t-lead); line-height: 1.5; color: var(--muted); max-width: 42ch; }
  .actions { margin-top: 28px; }
  .actions .btn { height: 44px; }
  /* The logo carries the page's one glow, so the call to action stays flat. */
  .flat, .flat:hover { box-shadow: none; }
  .art { width: 100%; justify-self: end; }
  @media (max-width: 1099.98px) { .hero { grid-template-columns: minmax(0, 1fr) minmax(0, 320px); gap: 24px 32px; } }
  @media (max-width: 699.98px) {
    .hero { grid-template-columns: minmax(0, 1fr); }
    .art { order: -1; justify-self: center; max-width: 200px; }
  }
  @media (max-width: 520px) {
    .hero-lead { font-size: 16px; }
    .actions { margin-top: 20px; grid-template-columns: minmax(0, 1fr); }
  }
  .arrow { display: inline-flex; }

  .block { margin-top: clamp(40px, 4vw, 56px); scroll-margin-top: calc(var(--nav-h) + 20px); }

  /* The boxes: one card each, the same picture slot, text and a foot pinned to the bottom. */
  .shows { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 16px; }
  @media (max-width: 1199.98px) { .shows { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 599.98px) { .shows { grid-template-columns: minmax(0, 1fr); } }
  .show { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; min-width: 0; padding: 20px;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .show :global(.art) { margin: 4px 0 16px; }
  .show h3 { font-size: 17px; line-height: 1.35; letter-spacing: -.01em; }
  .show > p { margin: 0; font-size: var(--t-small); line-height: 1.55; color: var(--muted); }
  .foot { display: grid; justify-items: start; gap: 10px; margin-top: auto; padding-top: 12px; }
  .fine { margin: 0; font-size: var(--t-fine); color: var(--dim); }
  .quiet { color: var(--muted); text-decoration: underline; text-underline-offset: 3px; }
  @media (hover: hover) and (pointer: fine) { .quiet:hover { color: var(--text); } }
</style>
