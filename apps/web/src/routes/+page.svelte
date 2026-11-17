<script lang="ts">
  import { onMount } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import ChatPreview from '$lib/components/ChatPreview.svelte';
  import EmoteCard from '$lib/components/EmoteCard.svelte';
  import Eyebrow from '$lib/components/Eyebrow.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import SectionHead from '$lib/components/SectionHead.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import HeroCluster from '$lib/components/home/HeroCluster.svelte';
  import { browserOf, chat, cluster, strip, type Browser, type Sort } from '$lib/components/home/pick';
  import { reveal } from '$lib/components/home/reveal';
  import { count } from '$lib/format';

  let { data } = $props();

  const art = $derived(cluster(data.emotes));
  const hasArt = $derived(art.tiles.length >= 3);
  let sort = $state<Sort>('top');
  let stripEl = $state<HTMLUListElement>();
  const row = $derived(strip(data.emotes, sort));
  const twitch = $derived(chat(data.emotes, 'twitch'));
  const kick = $derived(chat(data.emotes, 'kick'));

  // Install cards: the browser the page is open in leads. `order` only, so nothing resizes.
  let browser = $state<Browser>('other');
  onMount(() => (browser = browserOf(navigator.userAgent)));
  const rank = (b: Browser, i: number) => (browser === b ? -1 : i);

  // Phones stack the two chats, so each shows fewer lines (set before the band is in view).
  let rows = $state(6);
  onMount(() => {
    if (matchMedia('(max-width: 900px)').matches) rows = 4;
  });

  let retrying = $state(false);
  async function retry() {
    retrying = true;
    await invalidateAll();
    retrying = false;
  }

  const SAMPLE = `curl -s https://api.cyex.tv/v1/gql \\
  -H 'content-type: application/json' \\
  -d '{"query":"{ globalSet { emotes { name } } }"}'`;
</script>

<svelte:head>
  <title>cyex.tv · Emotes for Twitch and Kick chat</title>
  <meta name="description" content="Upload emotes, put them in your channel's set, and see them in chat on Twitch and Kick." />
</svelte:head>

<section class="hero">
  <div class="page hero-grid" class:solo={!hasArt}>
    <div class="copy">
      <h1>Emotes for every chat</h1>
      <p class="lead">Upload your own, put them in your channel's set, and chat sees them on Twitch and Kick.</p>
      <div class="actions">
        <a class="btn btn-primary" href="/search">
          Explore emotes
          <span class="arrow"><Icon name="arrow-right" /></span>
        </a>
        <a class="btn" href="#install">Get the extension</a>
      </div>
    </div>
    {#if hasArt}
      <div class="art"><HeroCluster {art} /></div>
    {/if}
  </div>
</section>

<section class="section page" aria-labelledby="strip-title">
  <div class="strip-head" use:reveal>
    <SectionHead eyebrow="Emotes" lit="top" title="Fresh from the library." id="strip-title" />
    <div class="strip-tools">
      <Segmented label="Sort" options={[{ value: 'top', label: 'Top' }, { value: 'new', label: 'New' }]} bind:value={sort} onchange={() => stripEl?.scrollTo({ left: 0 })} />
      <a class="link" href="/search">
        Browse all{data.total ? ` ${count(data.total)}` : ''}
        <span class="arrow"><Icon name="arrow-right" /></span>
      </a>
    </div>
  </div>
  {#if row.length}
    <ul class="strip" bind:this={stripEl} aria-label="{sort === 'top' ? 'Top' : 'New'} emotes" use:reveal={1}>
      {#each row as e, i (e.id)}
        <li class:late={i >= 14}><EmoteCard emote={e} eager={i < 6} /></li>
      {/each}
    </ul>
  {:else}
    <div class="empty">
      {#if data.failed}
        <p><strong>The emotes didn't load.</strong> The API didn't answer; it's usually back within a minute.</p>
        <button class="btn btn-s" type="button" onclick={retry} disabled={retrying}>{retrying ? 'Trying' : 'Try again'}</button>
      {:else}
        <p><strong>No emotes yet.</strong> The library is empty. The first upload gets this whole row.</p>
        <a class="btn btn-s" href="/me/upload">Upload one</a>
      {/if}
    </div>
  {/if}
</section>

{#if twitch.length && kick.length}
  <section class="section page chat-band" aria-labelledby="chat-title">
    <div use:reveal>
      <SectionHead
        eyebrow="In chat"
        id="chat-title"
        title="What your viewers see."
        lead="Emotes sit inline at chat size, animated ones play, and zero-width ones stack on the emote before them. Same set on both platforms."
      />
    </div>
    <div class="chats">
      <div use:reveal={1}><ChatPreview platform="twitch" lines={twitch} live visible={rows} interval={2600} /></div>
      <div use:reveal={2}><ChatPreview platform="kick" lines={kick} live visible={rows} interval={3100} /></div>
    </div>
    <p class="fine caption">Made-up viewers, real emotes from the library.</p>
  </section>
{/if}

<section class="section page" aria-labelledby="how-title">
  <div use:reveal>
    <SectionHead eyebrow="How it works" lit="bottom" id="how-title" title="Three steps, no settings page." />
  </div>
  <ol class="steps how">
    <li use:reveal={0}><span class="num">1</span><h3>Install the extension</h3><p>It adds cyex.tv emotes to Twitch and Kick in your browser.</p></li>
    <li use:reveal={1}><span class="num">2</span><h3>Add emotes to your set</h3><p>Sign in, pick what your channel shows. Your editors can too.</p></li>
    <li use:reveal={2}><span class="num">3</span><h3>Chat</h3><p>Type the name. Everyone with cyex.tv sees the emote.</p></li>
  </ol>
</section>

<section class="section page" id="install" aria-labelledby="install-title">
  <div use:reveal>
    <SectionHead
      eyebrow="Install"
      id="install-title"
      title="Get it where you chat."
      lead="cyex.tv is new. Here's where it runs today and what's still on its way."
    />
  </div>
  <div class="installs">
    <article class="install" style:order={rank('chrome', 0)} use:reveal={0}>
      <p class="status"><span class="dot" aria-hidden="true"></span>Early testing{#if browser === 'chrome'}<span class="yours"> · your browser</span>{/if}</p>
      <h3>Extension for Chrome</h3>
      <p>Emotes in Twitch and Kick chat on the web. Works in Edge, Brave and Opera too.</p>
      <p class="fine">Coming to the Chrome Web Store.</p>
    </article>
    <article class="install" style:order={rank('firefox', 1)} use:reveal={1}>
      <p class="status"><span class="dot" aria-hidden="true"></span>Early testing{#if browser === 'firefox'}<span class="yours"> · your browser</span>{/if}</p>
      <h3>Extension for Firefox</h3>
      <p>The same extension, for Firefox on the desktop.</p>
      <p class="fine">Coming to Firefox Add-ons.</p>
    </article>
    <article class="install" style:order={2} use:reveal={2}>
      <p class="status"><span class="dot off" aria-hidden="true"></span>Coming</p>
      <h3>cyex deck</h3>
      <p>A light Twitch app for Windows. cyex.tv emotes are on the way to it.</p>
      <p class="fine">Already out for Twitch; the emotes come in an update.</p>
    </article>
  </div>
</section>

<section class="section page" aria-label="For channels, creators and developers">
  <div class="trio">
    <div class="part" use:reveal={0}>
      <Eyebrow lit="top">For channels</Eyebrow>
      <h3>One set per channel.</h3>
      <p>Your team keeps it. Add, rename or drop an emote and open chats get the change within a second.</p>
      <a class="link" href="/me/channels">Set up your channel <span class="arrow"><Icon name="arrow-right" /></span></a>
    </div>
    <div class="part" use:reveal={1}>
      <Eyebrow lit="middle">For creators</Eyebrow>
      <h3>Upload, get reviewed, get used.</h3>
      <p>PNG, GIF and WebP up to 5 MB. A moderator checks each one before others see it, which keeps stolen art out.</p>
      <a class="link" href="/me/upload">Upload an emote <span class="arrow"><Icon name="arrow-right" /></span></a>
      <p class="fine">Your art here without your say? <a class="link-inline" href="https://github.com/cyex-tv/platform/blob/main/docs/takedown.md">Ask for a takedown</a>.</p>
    </div>
    <div class="part" use:reveal={2}>
      <Eyebrow lit="bottom">For developers</Eyebrow>
      <h3>An open API.</h3>
      <p>GraphQL and REST. Public data needs no key.</p>
      <pre class="code" aria-label="Example request"><code>{SAMPLE}</code></pre>
      <div class="links">
        <a class="link" href="/developers">Read the guide <span class="arrow"><Icon name="arrow-right" /></span></a>
        <a class="link" href="/developers/reference">API reference <span class="arrow"><Icon name="arrow-right" /></span></a>
      </div>
    </div>
  </div>
</section>

<section class="section page closing">
  <div class="band" use:reveal>
    <div>
      <h2>Find one you like.</h2>
      <p class="fine band-sub">Then put it in your set. That's the whole trick.</p>
    </div>
    <div class="actions">
      <a class="btn btn-primary" href="/search">Explore emotes <span class="arrow"><Icon name="arrow-right" /></span></a>
      <a class="btn" href="#install">Get the extension</a>
    </div>
  </div>
</section>

<style>
  /* Hero: copy beside the cluster, stacked below 900px. Short enough that the cluster
     starts on the first screen of a 375 x 667 phone. */
  .hero { padding: clamp(24px, 5vw, 72px) 0 0; }
  .hero-grid { display: grid; grid-template-columns: minmax(0, .95fr) minmax(0, 1.05fr); gap: 32px 72px; align-items: center; }
  .hero-grid.solo { grid-template-columns: minmax(0, 1fr); }
  .hero h1 { font-size: var(--t-h1); line-height: .98; letter-spacing: -.045em; max-width: 11ch; }
  .hero .lead { margin-top: 24px; }
  .hero .actions { margin-top: 32px; }
  .art { width: 100%; max-width: 720px; justify-self: end; }
  @media (max-width: 900px) {
    .hero-grid { grid-template-columns: minmax(0, 1fr); }
    .art { justify-self: center; max-width: 560px; }
  }
  @media (max-width: 520px) {
    .hero h1 { font-size: 44px; }
    .hero .lead { margin-top: 16px; font-size: 17px; }
    .hero .actions { margin-top: 24px; grid-template-columns: 1fr 1fr; }
    .hero .actions .btn { padding: 0 12px; }
  }
  @media (max-width: 359px) { .hero .actions { grid-template-columns: minmax(0, 1fr); } }

  .arrow { display: inline-flex; }

  /* Strip: one snapping row on phones and tablets, two rows of 10 (7 at 1024) on desktop. */
  .strip-head { display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: space-between; gap: 24px 40px; }
  .strip-tools { display: flex; align-items: center; gap: 24px; }
  .strip { list-style: none; margin: 40px 0 0; padding: 0; display: grid; grid-template-columns: repeat(10, minmax(0, 1fr)); gap: 8px 0; }
  .strip li { min-width: 0; display: flex; justify-content: center; }
  @media (max-width: 1199px) { .strip { grid-template-columns: repeat(7, minmax(0, 1fr)); } .late { display: none; } }
  @media (max-width: 1023px) {
    .strip { grid-template-columns: none; grid-auto-flow: column; grid-auto-columns: 128px; overflow-x: auto;
      margin-inline: calc(-1 * var(--page-pad)); padding: 0 var(--page-pad) 8px; scroll-padding-inline: var(--page-pad);
      scroll-snap-type: x mandatory; overscroll-behavior-x: contain; scrollbar-width: thin; }
    .strip li { scroll-snap-align: start; }
    .late { display: flex; }
  }
  .empty { margin-top: 40px; display: flex; flex-wrap: wrap; align-items: center; gap: 16px 24px; padding: 28px;
    border: 1px dashed var(--border-strong); border-radius: var(--radius-m); color: var(--muted); }
  .empty p { margin: 0; }
  .empty strong { color: var(--text); font-weight: 500; }

  /* Chat band */
  .chats { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; margin-top: 48px; }
  @media (max-width: 900px) { .chats { grid-template-columns: minmax(0, 1fr); } }
  .caption { margin: 14px 0 0; }

  /* Steps */
  .how { list-style: none; padding: 0; margin-bottom: 0; }
  .how li { border-top: 1px solid var(--border-strong); padding-top: 20px; }
  .num { display: block; margin-bottom: 14px; font-size: var(--t-figure); font-weight: 500; line-height: 1.05;
    letter-spacing: -.035em; font-variant-numeric: tabular-nums; color: var(--muted); }

  /* Install */
  .installs { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; margin-top: 48px; }
  @media (max-width: 900px) { .installs { grid-template-columns: minmax(0, 1fr); } }
  .install { display: flex; flex-direction: column; gap: 10px; padding: 24px; border: 1px solid var(--border);
    border-radius: var(--radius-m); background: var(--surface); }
  .install h3 { font-size: var(--t-h3); line-height: 1.3; letter-spacing: -.015em; }
  .install p { margin: 0; color: var(--muted); }
  .install .fine { margin-top: auto; padding-top: 6px; }
  .status { display: flex; align-items: center; gap: 8px; font-size: var(--t-small); font-weight: 500; }
  .install .status { color: var(--text); }
  .yours { color: var(--muted); font-weight: 400; white-space: pre; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 10px var(--accent-glow); }
  .dot.off { background: transparent; border: 1.5px solid var(--muted); box-shadow: none; }

  /* Trio */
  .trio { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 48px 40px; }
  @media (max-width: 900px) { .trio { grid-template-columns: minmax(0, 1fr); } }
  .part { display: flex; flex-direction: column; align-items: flex-start; gap: 12px; min-width: 0;
    border-top: 1px solid var(--border-strong); padding-top: 24px; }
  .part :global(.eyebrow) { margin-bottom: 4px; }
  .part h3 { font-size: var(--t-h3-sub); line-height: 1.1; letter-spacing: -.03em; }
  .part p { margin: 0; color: var(--muted); max-width: 44ch; }
  .part .link { margin-top: 4px; }
  .link-inline { color: var(--accent-text); text-decoration: underline; text-underline-offset: 3px; }
  .links { display: flex; flex-wrap: wrap; gap: 8px 24px; }
  .code { width: 100%; margin: 4px 0; padding: 14px 16px; overflow-x: auto; border: 1px solid var(--border);
    border-radius: var(--radius-s); background: var(--surface); font-size: var(--t-fine); line-height: 1.6; color: var(--text); }
  @media (max-width: 520px) { .code { white-space: pre-wrap; overflow-wrap: anywhere; } }
  .code code { font-family: ui-monospace, 'Cascadia Code', Menlo, Consolas, monospace; }

  /* Closing band (the page's second and last glow lives in .band) */
  .closing { padding-bottom: var(--section-pad); }
  .band-sub { margin: 10px 0 0; font-size: var(--t-small); }

  /* Reveal once (reveal.ts sets data-reveal only below the fold, never under reduced motion). */
  :global([data-reveal]) { transition: opacity var(--dur-reveal) var(--ease), translate var(--dur-reveal) var(--ease);
    transition-delay: calc(var(--step, 0) * var(--stagger)); }
  :global([data-reveal='wait']) { opacity: 0; translate: 0 28px; }
</style>
