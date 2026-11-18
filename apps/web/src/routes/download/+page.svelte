<script lang="ts">
  // /download: where cyex.tv runs today, said plainly. The extension isn't in the stores yet, so
  // the page says so and shows how to load the test build; cyex deck and chat clients get one
  // honest line each. Your browser's card comes first. Nothing here loads from elsewhere.
  import { onMount } from 'svelte';
  import { copy } from '$lib/clipboard.svelte';
  import { browserOf, type Browser } from '$lib/components/home/pick';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

  let browser = $state<Browser>('other');
  onMount(() => (browser = browserOf(navigator.userAgent)));
  const rank = (b: Browser, i: number) => (browser === b ? -1 : i);

  const BUILD = 'git clone https://github.com/cyex-tv/extension\ncd extension\nnpm ci\nnpm run build';
  let copied = $state<'idle' | 'copied' | 'failed'>('idle');
  let timer: ReturnType<typeof setTimeout> | undefined;
  async function copyBuild() {
    copied = (await copy(BUILD)) ? 'copied' : 'failed';
    clearTimeout(timer);
    timer = setTimeout(() => (copied = 'idle'), 1600);
  }
</script>

<svelte:head>
  <title>Download - cyex.tv</title>
  <meta name="description" content="The cyex.tv extension for Chrome and Firefox: where it stands, and how to try the test build today." />
  <meta property="og:title" content="Download" />
</svelte:head>

<ColumnLayout>
  <header class="page-head">
    <h1 class="page-title">Download</h1>
    <p class="lead">cyex.tv emotes, badges and paints in Twitch and Kick chat. The extension is in early testing: not in the stores yet, but you can try it today.</p>
  </header>

  <section class="block" aria-labelledby="ext-title">
    <h2 class="block-title" id="ext-title">The extension</h2>
    <div class="cards">
      <article class="card" style:order={rank('chrome', 0)}>
        <p class="status"><span class="dot" aria-hidden="true"></span>Early testing{#if browser === 'chrome'}<span class="yours"> · your browser</span>{/if}</p>
        <h3>Chrome</h3>
        <p>Also Edge, Brave, Opera and other Chromium browsers on the desktop.</p>
        <p class="foot">Coming to the Chrome Web Store.</p>
      </article>
      <article class="card" style:order={rank('firefox', 1)}>
        <p class="status"><span class="dot" aria-hidden="true"></span>Early testing{#if browser === 'firefox'}<span class="yours"> · your browser</span>{/if}</p>
        <h3>Firefox</h3>
        <p>The same extension, for Firefox on the desktop.</p>
        <p class="foot">Coming to Firefox Add-ons.</p>
      </article>
      <article class="card" style:order={2}>
        <p class="status"><span class="dot off" aria-hidden="true"></span>Coming</p>
        <h3>cyex deck</h3>
        <p>The light Twitch app for Windows. cyex.tv emotes are on their way to it.</p>
        <p class="foot">Already out for Twitch; the emotes come in an update.</p>
      </article>
    </div>
    <ul class="what">
      <li>cyex.tv emotes, and 7TV, BTTV and FFZ too, each with its own switch.</li>
      <li>Badges and name paints, live as people change them.</li>
      <li>Hide the parts of Twitch and Kick you never use. A short setup you can skip.</li>
    </ul>
  </section>

  <section class="block" aria-labelledby="test-title">
    <h2 class="block-title" id="test-title">Try the test build</h2>
    <p class="sub">Build it from the source: you need git and Node 22.12 or newer. It takes a minute.</p>
    <div class="code">
      <pre><code>{BUILD}</code></pre>
      <button type="button" class="ui-btn secondary copy" onclick={copyBuild} aria-describedby="copy-state">{copied === 'copied' ? 'Copied' : 'Copy'}</button>
      <span class="sr-only" id="copy-state" role="status">{copied === 'copied' ? 'Copied' : copied === 'failed' ? 'Couldn’t copy' : ''}</span>
    </div>
    <div class="steps">
      <div class="step" style:order={rank('chrome', 0)}>
        <h3>Chrome, Edge, Brave, Opera</h3>
        <ol>
          <li>Open <code>chrome://extensions</code> (Edge: <code>edge://extensions</code>).</li>
          <li>Switch on Developer mode.</li>
          <li>Choose Load unpacked and pick the <code>dist/chrome</code> folder.</li>
        </ol>
      </div>
      <div class="step" style:order={rank('firefox', 1)}>
        <h3>Firefox</h3>
        <ol>
          <li>Open <code>about:debugging#/runtime/this-firefox</code>.</li>
          <li>Choose Load Temporary Add-on.</li>
          <li>Pick <code>dist/firefox/manifest.json</code>.</li>
        </ol>
      </div>
    </div>
    <p class="note">Test builds don’t update themselves: pull and build again for a new one. Firefox removes a temporary add-on when it closes.</p>
  </section>

  <section class="block" aria-labelledby="clients-title">
    <h2 class="block-title" id="clients-title">Chat clients and bots</h2>
    <p class="sub">No chat client ships cyex.tv emotes yet. The API is public and needs no key, so any client, bot or overlay can read a channel’s set. <a class="link inline" href="/developers">Developers</a></p>
  </section>

  <p class="end">Stuck? <a class="link inline" href="/help#emotes-not-showing">Help</a> has the common questions.</p>
</ColumnLayout>

<style>
  .lead { margin: 12px 0 0; max-width: 62ch; font-size: var(--t-body, 16px); line-height: 1.6; color: var(--muted); }
  .block { margin-top: clamp(32px, 4vw, 48px); }
  .block:first-of-type { margin-top: 0; }
  .block-title { margin-bottom: 16px; }
  .sub { margin: -8px 0 16px; max-width: 70ch; font-size: var(--t-small); line-height: 1.6; color: var(--muted); }
  .link.inline { display: inline; }

  .cards { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
  .card { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; min-width: 0; padding: 20px;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .card h3, .step h3 { margin: 0; font-size: 17px; line-height: 1.35; letter-spacing: -.01em; }
  .card p { margin: 0; font-size: var(--t-small); line-height: 1.55; color: var(--muted); }
  .card .foot { margin-top: auto; padding-top: 8px; font-size: var(--t-fine); color: var(--dim); }
  .status { display: flex; align-items: center; gap: 8px; font-weight: 500; }
  .card .status { color: var(--text); font-size: var(--t-fine); }
  .yours { color: var(--muted); font-weight: 400; white-space: pre; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); }
  .dot.off { background: transparent; border: 1.5px solid var(--muted); }

  .what { margin: 16px 0 0; padding-left: 20px; font-size: var(--t-small); line-height: 1.8; color: var(--muted); }

  .code { position: relative; max-width: 720px; }
  pre { margin: 0; padding: 14px 16px; padding-right: 96px; overflow-x: auto; border: 1px solid var(--border); border-radius: var(--radius-card);
    background: var(--elevated-2, var(--surface)); font-family: var(--font-mono); font-size: 13px; line-height: 1.7; }
  .copy { position: absolute; top: 10px; right: 10px; min-width: 72px; }
  code { font-family: var(--font-mono); font-size: .9em; overflow-wrap: anywhere; }
  .steps code { padding: 1px 5px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); }

  .steps { display: grid; grid-template-columns: repeat(2, minmax(0, 360px)); gap: 16px 40px; margin-top: 24px; }
  .step ol { margin: 10px 0 0; padding-left: 20px; font-size: var(--t-small); line-height: 1.8; color: var(--muted); }
  .note { margin: 20px 0 0; font-size: var(--t-fine); color: var(--dim); max-width: 70ch; }
  .end { margin: clamp(32px, 4vw, 48px) 0 0; font-size: var(--t-small); color: var(--muted); }

  @media (max-width: 899.98px) {
    .cards { grid-template-columns: minmax(0, 1fr); }
    .steps { grid-template-columns: minmax(0, 1fr); }
  }
</style>
