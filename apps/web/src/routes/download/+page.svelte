<script lang="ts">
  // /download: where cyex.tv runs today, said plainly. The extension isn't in the stores yet, so
  // the page says so and shows how to load the test build; cyex deck and chat clients get one
  // honest line each. Your browser's cell comes first (bold, accent tick). Nothing here loads from
  // elsewhere. Inspector shape: the page on the left, a panel on the right with the facts at a
  // glance and where to go next; under 1100px the panel follows the page.
  import { onMount } from 'svelte';
  import { copy } from '$lib/clipboard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { browserOf, type Browser } from '$lib/components/home/pick';

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

<div class="wrap dl">
  <div class="main">
    <header class="page-head">
      <h1 class="page-title">Download</h1>
      <p class="line">cyex.tv emotes, badges and paints in Twitch and Kick chat. The extension is in early testing: not in the stores yet, but you can try it today.</p>
    </header>

    <section class="block" aria-labelledby="ext-title">
      <h2 class="label" id="ext-title">The extension</h2>
      <div class="cells">
        <article class="cell" id="chrome" class:yours={browser === 'chrome'} style:order={rank('chrome', 0)}>
          <!-- The front page's extension menu links each Chromium browser here by name. -->
          <span class="anchor" id="edge"></span><span class="anchor" id="opera"></span><span class="anchor" id="brave"></span>
          <p class="status"><span class="dot" aria-hidden="true"></span>Early testing</p>
          <h3>{#if browser === 'chrome'}<Icon name="check" size={14} stroke={2.4} />{/if}Chrome{#if browser === 'chrome'}<span class="mine">Your browser</span>{/if}</h3>
          <p>Also Edge, Brave, Opera and other Chromium browsers on the desktop.</p>
          <p class="foot">Coming to the Chrome Web Store.</p>
        </article>
        <article class="cell" id="firefox" class:yours={browser === 'firefox'} style:order={rank('firefox', 1)}>
          <p class="status"><span class="dot" aria-hidden="true"></span>Early testing</p>
          <h3>{#if browser === 'firefox'}<Icon name="check" size={14} stroke={2.4} />{/if}Firefox{#if browser === 'firefox'}<span class="mine">Your browser</span>{/if}</h3>
          <p>The same extension, for Firefox on the desktop.</p>
          <p class="foot">Coming to Firefox Add-ons.</p>
        </article>
        <article class="cell" style:order={2}>
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
      <h2 class="label" id="test-title">Try the test build</h2>
      <p class="sub">Build it from the source: you need git and Node 22.12 or newer. It takes a minute.</p>
      <div class="code">
        <pre><code>{BUILD}</code></pre>
        <button type="button" class="btn btn-s copy" onclick={copyBuild} aria-describedby="copy-state"><Icon name={copied === 'copied' ? 'check' : 'copy'} size={14} />{copied === 'copied' ? 'Copied' : 'Copy'}</button>
        <span class="sr-only" id="copy-state" role="status">{copied === 'copied' ? 'Copied' : copied === 'failed' ? 'Couldn’t copy' : ''}</span>
      </div>
      <div class="cells steps">
        <div class="cell step" style:order={rank('chrome', 0)}>
          <h3>Chrome, Edge, Brave, Opera</h3>
          <ol>
            <li>Open <code>chrome://extensions</code> (Edge: <code>edge://extensions</code>).</li>
            <li>Switch on Developer mode.</li>
            <li>Choose Load unpacked and pick the <code>dist/chrome</code> folder.</li>
          </ol>
        </div>
        <div class="cell step" style:order={rank('firefox', 1)}>
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
      <h2 class="label" id="clients-title">Chat clients and bots</h2>
      <p class="sub">No chat client ships cyex.tv emotes yet. The API is public and needs no key, so any client, bot or overlay can read a channel’s set. <a class="link inline" href="/developers">Developers</a></p>
    </section>
  </div>

  <aside class="panel" aria-labelledby="glance-h">
    <div class="ip-head"><h2 class="label" id="glance-h">At a glance</h2></div>
    <div class="ip-body">
      <dl class="kv">
        <dt>Status</dt><dd>Early testing</dd>
        <dt>Browsers</dt><dd>Chromium and Firefox, desktop</dd>
        <dt>Phones</dt><dd>Not yet</dd>
        <dt>Price</dt><dd>Free</dd>
        <dt>Test build needs</dt><dd>git, Node 22.12+</dd>
      </dl>
      <h3 class="label sub-label">Next</h3>
      <ul class="links">
        <li><a href="/help#emotes-not-showing"><Icon name="scroll" size={16} /><span>Emotes don’t show in chat</span><Icon name="chevron-right" size={14} /></a></li>
        <li><a href="/developers"><Icon name="file" size={16} /><span>The API for clients and bots</span><Icon name="chevron-right" size={14} /></a></li>
        <li><a href="https://github.com/cyex-tv/extension" rel="noopener"><Icon name="open" size={16} /><span>The extension’s source</span><Icon name="chevron-right" size={14} /></a></li>
      </ul>
    </div>
  </aside>
</div>

<style>
  p, h2, h3 { margin: 0; }
  .dl { display: grid; grid-template-columns: minmax(0, 1fr) 360px; column-gap: 24px; align-items: start; }
  .main { min-width: 0; padding-bottom: 48px; }
  .line { margin-top: 6px; max-width: 62ch; font-size: var(--t-ui); line-height: 1.55; color: var(--muted); }
  .label { font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .block { display: grid; gap: 12px; }
  .block + .block { margin-top: 32px; padding-top: 20px; border-top: 1px solid var(--border); }
  .sub { max-width: 70ch; font-size: var(--t-small); line-height: 1.6; color: var(--muted); }
  .link.inline { display: inline; }

  /* Cells sharing hairlines in one frame, like the shop's grid. */
  .cells { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); border: 1px solid var(--border); border-radius: var(--radius-card); overflow: hidden; }
  .cell { position: relative; display: flex; flex-direction: column; align-items: flex-start; gap: 8px; min-width: 0; padding: 16px;
    box-shadow: 1px 0 0 var(--border), 0 1px 0 var(--border); scroll-margin-top: calc(var(--nav-h) + 20px); }
  .anchor { position: absolute; top: 0; scroll-margin-top: calc(var(--nav-h) + 20px); }
  .cell h3 { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; font-size: 17px; font-weight: 600; line-height: 1.35; letter-spacing: -.01em; }
  .cell h3 :global(svg) { color: var(--accent-text); }
  .yours h3 { font-weight: 700; }
  .mine { font-size: var(--t-fine); font-weight: 500; color: var(--muted); letter-spacing: 0; }
  .cell p { font-size: var(--t-small); line-height: 1.55; color: var(--muted); }
  .cell .foot { margin-top: auto; padding-top: 8px; font-size: var(--t-fine); color: var(--dim); }
  .status { display: flex; align-items: center; gap: 8px; font-weight: 500; }
  .cell .status { color: var(--text); font-size: var(--t-fine); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); }
  .dot.off { background: transparent; border: 1.5px solid var(--muted); }
  .what { margin: 0; padding-left: 20px; font-size: var(--t-small); line-height: 1.8; color: var(--muted); }

  .code { position: relative; max-width: 720px; }
  pre { margin: 0; padding: 14px 16px; padding-right: 108px; overflow-x: auto; border: 1px solid var(--border); border-radius: var(--radius-card);
    background: var(--elevated-2, var(--surface)); font-family: var(--font-mono); font-size: 13px; line-height: 1.7; }
  .copy { position: absolute; top: 10px; right: 10px; min-width: 84px; }
  .copy :global(svg) { color: var(--muted); }
  code { font-family: var(--font-mono); font-size: .9em; overflow-wrap: anywhere; }
  .cells.steps { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .steps code { padding: 1px 5px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); }
  .step h3 { font-size: var(--t-ui); }
  .step ol { margin: 0; padding-left: 20px; font-size: var(--t-small); line-height: 1.8; color: var(--muted); }
  .note { font-size: var(--t-fine); color: var(--dim); max-width: 70ch; }

  .panel { position: sticky; top: var(--nav-h); height: calc(100svh - var(--nav-h)); overflow-y: auto; overscroll-behavior: contain;
    margin-right: calc(-1 * var(--page-pad)); border-left: 1px solid var(--border); background: var(--surface); }
  .ip-head { display: flex; align-items: center; height: 48px; padding: 0 16px; border-bottom: 1px solid var(--border); }
  .ip-body { display: grid; padding: 8px 16px 16px; }
  .kv { display: grid; grid-template-columns: auto minmax(0, 1fr); margin: 0 0 8px; }
  .kv dt, .kv dd { margin: 0; padding: 9px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .kv dt { color: var(--muted); padding-right: 16px; white-space: nowrap; }
  .kv dd { text-align: right; font-weight: 600; }
  .sub-label { display: flex; align-items: center; min-height: 36px; padding-top: 8px; }
  .links { list-style: none; margin: 0; padding: 0; display: grid; }
  .links li + li { border-top: 1px solid var(--border); }
  .links a { display: flex; align-items: center; gap: 10px; min-height: 44px; margin: 0 -6px; padding: 0 6px; border-radius: var(--radius);
    color: var(--text); font-size: var(--t-small); font-weight: 600; transition: background-color var(--fast); }
  .links a span { flex: 1; min-width: 0; }
  .links a :global(svg) { flex: none; color: var(--dim); }
  .links a:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .links a:hover { background: var(--row-hover); } }

  @media (max-width: 1099.98px) {
    .dl { grid-template-columns: minmax(0, 1fr); }
    .panel { position: static; height: auto; overflow: visible; margin: 0 0 48px; border: 1px solid var(--border); border-radius: var(--radius-card); }
    .main { padding-bottom: 24px; }
  }
  @media (max-width: 899.98px) {
    .cells, .cells.steps { grid-template-columns: minmax(0, 1fr); }
  }
</style>
