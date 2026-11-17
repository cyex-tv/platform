<script lang="ts">
  // The front page, on the same ground, wrapper and block rhythm as every other page
  // (ColumnLayout, 22px block titles, 6px surface cards, 48px between blocks). Playful only where
  // it counts: the hero's emote cluster (the page's one soft glow) and the live chat previews.
  // Emote rows use the one EmoteTile. No entrance animations; nothing waits to appear.
  import { onMount } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import ChatPreview from '$lib/components/ChatPreview.svelte';
  import EmoteTile from '$lib/components/EmoteTile.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import HeroCluster from '$lib/components/home/HeroCluster.svelte';
  import { browserOf, chat, cluster, strip, type Browser, type Sort } from '$lib/components/home/pick';
  import { count } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

  let { data } = $props();

  const art = $derived(cluster(data.emotes));
  const hasArt = $derived(art.tiles.length >= 3);
  let sort = $state<Sort>('top');
  // Up to two full rows at 8 across; the CSS shows as many as fit the row count per width.
  const row = $derived(strip(sort === 'new' ? data.fresh : data.emotes, 16));
  const twitch = $derived(chat(data.emotes, 'twitch'));
  const kick = $derived(chat(data.emotes, 'kick'));

  // Install cards: the browser the page is open in leads. `order` only, so nothing resizes.
  let browser = $state<Browser>('other');
  onMount(() => (browser = browserOf(navigator.userAgent)));
  const rank = (b: Browser, i: number) => (browser === b ? -1 : i);

  // Phones stack the two chats, so each shows fewer lines.
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

<ColumnLayout top>
  <section class="hero" class:solo={!hasArt} aria-labelledby="hero-title">
    <div class="copy">
      <h1 id="hero-title">Emotes for every chat</h1>
      <p class="hero-lead">Upload your own, put them in your channel's set, and chat sees them on Twitch and Kick.</p>
      <div class="actions">
        <a class="btn btn-primary flat" href="/search">Explore emotes <span class="arrow"><Icon name="arrow-right" /></span></a>
        <a class="btn" href="#install">Get the extension</a>
      </div>
    </div>
    {#if hasArt}
      <div class="art"><HeroCluster {art} /></div>
    {/if}
  </section>

  <section class="block" aria-labelledby="emotes-title">
    <div class="block-head">
      <h2 class="block-title" id="emotes-title">{sort === 'top' ? 'Top emotes' : 'New emotes'}</h2>
      <div class="tools">
        <Segmented label="Sort" size="s" options={[{ value: 'top', label: 'Top' }, { value: 'new', label: 'New' }]} value={sort} onchange={(v) => (sort = v)} />
        <a class="link" href={sort === 'new' ? '/search?sort=new' : '/search'}>
          Browse all{data.total ? ` ${count(data.total)}` : ''}
          <span class="arrow"><Icon name="arrow-right" /></span>
        </a>
      </div>
    </div>
    {#if row.length}
      <ul class="tiles" aria-labelledby="emotes-title">
        {#each row as e, i (e.id)}
          <li><EmoteTile emote={e} eager={i < 8} /></li>
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
    <section class="block" aria-labelledby="chat-title">
      <div class="block-head">
        <div>
          <h2 class="block-title" id="chat-title">What your viewers see</h2>
          <p class="sub">Emotes sit inline at chat size, animated ones play, zero-width ones stack. Same set on both platforms.</p>
        </div>
      </div>
      <div class="chats">
        <ChatPreview platform="twitch" lines={twitch} live visible={rows} interval={2600} />
        <ChatPreview platform="kick" lines={kick} live visible={rows} interval={3100} />
      </div>
      <p class="note">Made-up viewers, real emotes from the library.</p>
    </section>
  {/if}

  <section class="block" aria-labelledby="how-title">
    <div class="block-head"><h2 class="block-title" id="how-title">How it works</h2></div>
    <ol class="cards how">
      <li class="card"><span class="num">1</span><h3>Install the extension</h3><p>It adds cyex.tv emotes to Twitch and Kick in your browser.</p></li>
      <li class="card"><span class="num">2</span><h3>Add emotes to your set</h3><p>Sign in, pick what your channel shows. Your editors can too.</p></li>
      <li class="card"><span class="num">3</span><h3>Chat</h3><p>Type the name. Everyone with cyex.tv sees the emote.</p></li>
    </ol>
  </section>

  <section class="block" id="install" aria-labelledby="install-title">
    <div class="block-head">
      <div>
        <h2 class="block-title" id="install-title">Get it where you chat</h2>
        <p class="sub">cyex.tv is new. Here's where it runs today and what's still on its way.</p>
      </div>
    </div>
    <div class="cards">
      <article class="card" style:order={rank('chrome', 0)}>
        <p class="status"><span class="dot" aria-hidden="true"></span>Early testing{#if browser === 'chrome'}<span class="yours"> · your browser</span>{/if}</p>
        <h3>Extension for Chrome</h3>
        <p>Emotes in Twitch and Kick chat on the web. Works in Edge, Brave and Opera too.</p>
        <p class="foot">Coming to the Chrome Web Store.</p>
      </article>
      <article class="card" style:order={rank('firefox', 1)}>
        <p class="status"><span class="dot" aria-hidden="true"></span>Early testing{#if browser === 'firefox'}<span class="yours"> · your browser</span>{/if}</p>
        <h3>Extension for Firefox</h3>
        <p>The same extension, for Firefox on the desktop.</p>
        <p class="foot">Coming to Firefox Add-ons.</p>
      </article>
      <article class="card" style:order={2}>
        <p class="status"><span class="dot off" aria-hidden="true"></span>Coming</p>
        <h3>cyex deck</h3>
        <p>A light Twitch app for Windows. cyex.tv emotes are on the way to it.</p>
        <p class="foot">Already out for Twitch; the emotes come in an update.</p>
      </article>
    </div>
  </section>

  <section class="block" aria-labelledby="who-title">
    <div class="block-head"><h2 class="block-title" id="who-title">For channels, creators and developers</h2></div>
    <div class="cards">
      <div class="card">
        <h3>One set per channel</h3>
        <p>Your team keeps it. Add, rename or drop an emote and open chats get the change within a second.</p>
        <a class="link foot" href="/me/channels">Set up your channel <span class="arrow"><Icon name="arrow-right" /></span></a>
      </div>
      <div class="card">
        <h3>Upload, get reviewed, get used</h3>
        <p>PNG, GIF and WebP up to 5 MB. A moderator checks each one before others see it, which keeps stolen art out.</p>
        <p class="fine">Your art here without your say? <a class="inline" href="https://github.com/cyex-tv/platform/blob/main/docs/takedown.md">Ask for a takedown</a>.</p>
        <a class="link foot" href="/me/upload">Upload an emote <span class="arrow"><Icon name="arrow-right" /></span></a>
      </div>
      <div class="card">
        <h3>An open API</h3>
        <p>GraphQL and REST. Public data needs no key.</p>
        <pre class="code" aria-label="Example request"><code>{SAMPLE}</code></pre>
        <div class="foot links">
          <a class="link" href="/developers">Read the guide <span class="arrow"><Icon name="arrow-right" /></span></a>
          <a class="link" href="/developers/reference">API reference <span class="arrow"><Icon name="arrow-right" /></span></a>
        </div>
      </div>
    </div>
  </section>
</ColumnLayout>

<style>
  /* Hero: one surface mount, copy beside the cluster (stacked below 900px). */
  .hero { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 440px); align-items: center; gap: 24px 48px;
    padding: clamp(20px, 3.5vw, 48px); border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface);
    overflow: hidden; }
  .hero.solo { grid-template-columns: minmax(0, 1fr); }
  .hero h1 { font-size: clamp(38px, 4.8vw, 68px); line-height: 1; letter-spacing: -.04em; max-width: 12ch; }
  .hero-lead { margin: 16px 0 0; font-size: var(--t-lead); line-height: 1.5; color: var(--muted); max-width: 42ch; }
  .actions { margin-top: 28px; }
  .actions .btn { height: 44px; }
  /* The page's one glow is the cluster's: the call to action stays flat. */
  .flat, .flat:hover { box-shadow: none; }
  .art { width: 100%; justify-self: end; }
  @media (max-width: 1099.98px) { .hero { grid-template-columns: minmax(0, 1fr) minmax(0, 320px); gap: 24px 32px; } }
  @media (max-width: 699.98px) {
    .hero { grid-template-columns: minmax(0, 1fr); }
    .art { justify-self: center; max-width: 340px; }
  }
  @media (max-width: 520px) {
    .hero-lead { font-size: 16px; }
    .actions { margin-top: 20px; grid-template-columns: minmax(0, 1fr); }
  }
  .arrow { display: inline-flex; }

  /* Blocks: the rhythm every page uses (block title, 16px under it, 48px between blocks). */
  .block { margin-top: clamp(40px, 4vw, 56px); scroll-margin-top: calc(var(--nav-h) + 20px); }
  .block-head { align-items: flex-end; }
  .sub { margin: 4px 0 0; font-size: var(--t-small); color: var(--muted); max-width: 70ch; }
  .tools { display: flex; align-items: center; gap: 20px; }

  /* Emote rows: the browse grid's tile and gap, two rows at most (three on phones). */
  .tiles { --cols: 8; --rows: 2; list-style: none; margin: 0; padding: 0; display: grid; gap: 16px;
    grid-template-columns: repeat(var(--cols), minmax(0, 1fr)); }
  .tiles li { min-width: 0; }
  @media (max-width: 1359.98px) { .tiles { --cols: 7; } .tiles li:nth-child(n + 15) { display: none; } }
  @media (max-width: 1099.98px) { .tiles { --cols: 6; } .tiles li:nth-child(n + 13) { display: none; } }
  @media (max-width: 899.98px) { .tiles { --cols: 4; } .tiles li:nth-child(n + 9) { display: none; } }
  @media (max-width: 599.98px) { .tiles { --cols: 2; gap: 8px; } .tiles li:nth-child(n + 7) { display: none; } }
  .empty { display: flex; flex-wrap: wrap; align-items: center; gap: 16px 24px; padding: 24px;
    border: 1px dashed var(--border-strong); border-radius: var(--radius-card); color: var(--muted); }
  .empty p { margin: 0; }
  .empty strong { color: var(--text); font-weight: 500; }

  /* Chat previews */
  .chats { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
  @media (max-width: 899.98px) { .chats { grid-template-columns: minmax(0, 1fr); } }
  .note { margin: 10px 0 0; font-size: var(--t-fine); color: var(--dim); }

  /* Cards: one surface card for steps, installs and the three audiences. */
  .cards { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
  @media (max-width: 899.98px) { .cards { grid-template-columns: minmax(0, 1fr); } }
  .card { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; min-width: 0; padding: 20px;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .card h3 { font-size: 17px; line-height: 1.35; letter-spacing: -.01em; }
  .card p { margin: 0; font-size: var(--t-small); line-height: 1.55; color: var(--muted); }
  .card .foot { margin-top: auto; padding-top: 8px; }
  .card p.foot { font-size: var(--t-fine); color: var(--dim); }
  .num { display: grid; place-items: center; width: 28px; height: 28px; margin-bottom: 4px; border: 1px solid var(--border-strong);
    border-radius: var(--radius); font-size: var(--t-fine); font-weight: 600; color: var(--muted); font-variant-numeric: tabular-nums; }
  /* Phones: a step is one compact row, its number beside the text. */
  @media (max-width: 899.98px) {
    .how .card { display: grid; grid-template-columns: auto minmax(0, 1fr); column-gap: 14px; row-gap: 2px; padding: 16px; }
    .how .num { grid-row: span 2; margin: 0; }
  }
  .status { display: flex; align-items: center; gap: 8px; font-weight: 500; }
  .card .status { color: var(--text); font-size: var(--t-fine); }
  .yours { color: var(--muted); font-weight: 400; white-space: pre; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); }
  .dot.off { background: transparent; border: 1.5px solid var(--muted); }
  .links { display: flex; flex-wrap: wrap; gap: 8px 20px; }
  .card .link { font-size: var(--t-small); }
  .inline { color: var(--accent-text); text-decoration: underline; text-underline-offset: 3px; }
  .code { width: 100%; margin: 4px 0 0; padding: 12px 14px; overflow-x: auto; border: 1px solid var(--border);
    border-radius: var(--radius); background: var(--bg); font-size: 12.5px; line-height: 1.6; color: var(--text); }
  .code code { font-family: var(--font-mono); }
  @media (max-width: 520px) { .code { white-space: pre-wrap; overflow-wrap: anywhere; } }
</style>
