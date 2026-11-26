<script lang="ts">
  // /download: where cyex.tv runs today, said plainly. Inspector shape (InspectorLayout): the
  // targets (Chromium browsers, Firefox, cyex deck) as a contact sheet on the left, cells sharing
  // hairlines; the picked one's install steps and facts in the right panel (under 1100px the panel
  // follows the sheet). Your browser is picked first; ?target= (or a #chrome / #edge / #firefox
  // link from the front page) picks another, so a target can be linked. The extension isn't in the
  // stores yet, so the panel says so and shows how to load the test build. Nothing loads from elsewhere.
  import { onMount } from 'svelte';
  import { replaceState } from '$app/navigation';
  import { copy } from '$lib/clipboard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import InspectorLayout from '$lib/shell/InspectorLayout.svelte';
  import { browserOf, type Browser } from '$lib/components/home/pick';

  type Target = 'chrome' | 'firefox' | 'deck';
  type Info = {
    id: Target;
    name: string;
    also: string;
    status: 'testing' | 'coming';
    store: string;
    steps: { text: string; code?: string; after?: string }[];
    facts: [string, string][];
  };
  const TARGETS: Info[] = [
    {
      id: 'chrome',
      name: 'Chrome',
      also: 'Edge, Brave, Opera and other Chromium browsers on the desktop.',
      status: 'testing',
      store: 'Coming to the Chrome Web Store.',
      steps: [
        { text: 'Open', code: 'chrome://extensions', after: '(Edge: edge://extensions).' },
        { text: 'Switch on Developer mode.' },
        { text: 'Choose Load unpacked and pick the', code: 'dist/chrome', after: 'folder.' },
      ],
      facts: [['Status', 'Early testing'], ['Browsers', 'Chrome, Edge, Brave, Opera'], ['Store', 'Not yet'], ['Price', 'Free'], ['Needs', 'git, Node 22.12+']],
    },
    {
      id: 'firefox',
      name: 'Firefox',
      also: 'The same extension, for Firefox on the desktop.',
      status: 'testing',
      store: 'Coming to Firefox Add-ons.',
      steps: [
        { text: 'Open', code: 'about:debugging#/runtime/this-firefox', after: '.' },
        { text: 'Choose Load Temporary Add-on.' },
        { text: 'Pick', code: 'dist/firefox/manifest.json', after: '.' },
      ],
      facts: [['Status', 'Early testing'], ['Browsers', 'Firefox, desktop'], ['Store', 'Not yet'], ['Price', 'Free'], ['Needs', 'git, Node 22.12+']],
    },
    {
      id: 'deck',
      name: 'cyex deck',
      also: 'The light Twitch app for Windows. cyex.tv emotes are on their way to it.',
      status: 'coming',
      store: 'Already out for Twitch; the emotes come in an update.',
      steps: [],
      facts: [['Status', 'Coming'], ['Platform', 'Windows'], ['Chat', 'Twitch'], ['cyex.tv emotes', 'In an update']],
    },
  ];
  const HASH: Record<string, Target> = { chrome: 'chrome', edge: 'chrome', opera: 'chrome', brave: 'chrome', firefox: 'firefox', deck: 'deck' };

  let browser = $state<Browser>('other');
  let hash = $state<Target | null>(null);
  // The page is prerendered: the address is read in the browser.
  let asked = $state<string | null>(null);
  onMount(() => {
    browser = browserOf(navigator.userAgent);
    hash = HASH[location.hash.slice(1)] ?? null;
    asked = new URLSearchParams(location.search).get('target');
  });
  const yours = $derived<Target | null>(browser === 'chrome' || browser === 'firefox' ? browser : null);
  const picked = $derived<Target>((asked && HASH[asked]) || hash || yours || 'chrome');
  const info = $derived(TARGETS.find((t) => t.id === picked)!);
  // Your browser's cell comes first.
  const order = $derived([...TARGETS].sort((a, b) => (a.id === yours ? -1 : b.id === yours ? 1 : 0)));

  const href = (t: Target) => `/download?target=${t}`;
  function pick(e: MouseEvent, t: Target) {
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.button) return;
    e.preventDefault();
    hash = null;
    asked = t;
    replaceState(href(t), {});
  }

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

{#snippet panel()}
  <div class="who">
    <p class="pname">{#if picked === yours}<Icon name="check" size={14} stroke={2.6} />{/if}{info.name}</p>
    <p class="pstat"><span class="dot" class:off={info.status === 'coming'} aria-hidden="true"></span>{info.status === 'testing' ? 'Early testing' : 'Coming'}{#if picked === yours}<span class="mine">Your browser</span>{/if}</p>
  </div>
  {#if info.steps.length}
    <PanelBlock title="Build it">
      <p class="pb-hint">From the source: you need git and Node 22.12 or newer. It takes a minute.</p>
      <div class="code">
        <pre><code>{BUILD}</code></pre>
        <button type="button" class="btn btn-s copy" onclick={copyBuild} aria-describedby="copy-state"><Icon name={copied === 'copied' ? 'check' : 'copy'} size={14} />{copied === 'copied' ? 'Copied' : 'Copy'}</button>
        <span class="sr-only" id="copy-state" role="status">{copied === 'copied' ? 'Copied' : copied === 'failed' ? 'Couldn’t copy' : ''}</span>
      </div>
    </PanelBlock>
    <PanelBlock title="Load it" count={info.steps.length}>
      <ol class="steps">
        {#each info.steps as s, i (i)}
          <li><span class="i">{i + 1}</span><span>{s.text}{#if s.code}{' '}<code>{s.code}</code>{/if}{#if s.after}{s.after.startsWith('.') ? '' : ' '}{s.after}{/if}</span></li>
        {/each}
      </ol>
      <p class="pb-hint">Test builds don’t update themselves: pull and build again for a new one.{#if picked === 'firefox'} Firefox removes a temporary add-on when it closes.{/if}</p>
    </PanelBlock>
  {:else}
    <PanelBlock title="When">
      <p class="pb-hint">{info.store}</p>
    </PanelBlock>
  {/if}
  <PanelBlock title="Facts">
    <dl class="kv">
      {#each info.facts as [k, v] (k)}<dt>{k}</dt><dd>{v}</dd>{/each}
    </dl>
  </PanelBlock>
  <PanelBlock title="Next">
    <LinkRows
      items={[
        { href: '/help#emotes-not-showing', label: 'Emotes don’t show in chat', icon: 'scroll' },
        { href: '/developers', label: 'The API for clients and bots', icon: 'file' },
        { href: 'https://github.com/cyex-tv/extension', label: 'The extension’s source', icon: 'open', external: true },
      ]}
    />
  </PanelBlock>
{/snippet}

<InspectorLayout title="Install" label="{info.name}: how to install" {panel} narrow="after">
  {#snippet head()}
    <header class="page-head">
      <h1 class="page-title">Download</h1>
      <p class="line">cyex.tv emotes, badges and paints in Twitch and Kick chat. The extension is in early testing: not in the stores yet, but you can try it today.</p>
    </header>
  {/snippet}

  <section class="sec" aria-labelledby="where-h">
    <h2 class="label" id="where-h">Where it runs<span class="n">{TARGETS.length}</span></h2>
    <ul class="sheet">
      {#each order as t (t.id)}
        <li class="cell" class:on={picked === t.id}>
          <a id={t.id} href={href(t.id)} aria-current={picked === t.id ? 'true' : undefined} onclick={(e) => pick(e, t.id)}>
            {#if t.id === 'chrome'}<!-- The front page's extension menu links each Chromium browser here by name. --><span class="anchor" id="edge"></span><span class="anchor" id="opera"></span><span class="anchor" id="brave"></span>{/if}
            <span class="stage" aria-hidden="true"><span class="word">{t.id === 'deck' ? 'deck' : t.name}</span></span>
            <span class="nm">{#if picked === t.id}<Icon name="check" size={13} stroke={2.6} />{/if}{t.name}{#if t.id === yours}<span class="mine">Your browser</span>{/if}</span>
            <span class="also">{t.also}</span>
            <span class="st"><span class="dot" class:off={t.status === 'coming'} aria-hidden="true"></span>{t.status === 'testing' ? 'Early testing' : 'Coming'}</span>
          </a>
        </li>
      {/each}
    </ul>
  </section>

  <section class="sec" aria-labelledby="what-h">
    <h2 class="label" id="what-h">What it does</h2>
    <dl class="rows">
      <dt>Emotes</dt><dd>cyex.tv emotes, and 7TV, BTTV and FFZ too, each with its own switch.</dd>
      <dt>Cosmetics</dt><dd>Badges and name paints, live as people change them.</dd>
      <dt>Tidy chat</dt><dd>Hide the parts of Twitch and Kick you never use. A short setup you can skip.</dd>
    </dl>
  </section>

  <section class="sec" aria-labelledby="clients-title">
    <h2 class="label" id="clients-title">Chat clients and bots</h2>
    <p class="sub">No chat client ships cyex.tv emotes yet. The API is public and needs no key, so any client, bot or overlay can read a channel’s set. <a class="link inline" href="/developers">Developers</a></p>
  </section>
</InspectorLayout>

<style>
  p, h2 { margin: 0; }
  .page-head { padding-bottom: 20px; }
  .line { margin-top: 6px; max-width: 62ch; font-size: var(--t-ui); line-height: 1.55; color: var(--muted); }
  /* Flat sections: an uppercase label on a hairline, no boxes. */
  .sec { display: grid; gap: 0; }
  .sec + .sec { margin-top: 32px; }
  .label { display: flex; align-items: center; gap: 8px; min-height: 40px; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em;
    text-transform: uppercase; color: var(--dim); }
  .n { font-family: var(--font-mono); letter-spacing: 0; font-variant-numeric: tabular-nums; }
  .sub { padding-top: 10px; border-top: 1px solid var(--border); max-width: 70ch; font-size: var(--t-small); line-height: 1.6; color: var(--muted); }
  .link.inline { display: inline; }

  /* The contact sheet: cells sharing hairlines, the picked one bold with an accent tick. */
  .sheet { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr));
    border-top: 1px solid var(--border); border-left: 1px solid var(--border); }
  .cell { min-width: 0; box-shadow: 1px 0 0 var(--border), 0 1px 0 var(--border); }
  .cell a { position: relative; display: grid; grid-template-rows: auto auto 1fr auto; gap: 6px; height: 100%; padding: 0 0 14px; color: var(--text);
    text-decoration: none; scroll-margin-top: calc(var(--nav-h) + 20px); transition: background-color var(--fast); }
  .cell a:focus-visible { outline-offset: -2px; }
  .anchor { position: absolute; top: 0; scroll-margin-top: calc(var(--nav-h) + 20px); }
  .stage { display: grid; place-items: center; height: 120px; margin-bottom: 6px; border-bottom: 1px solid var(--border); }
  .word { font-size: 26px; font-weight: 700; letter-spacing: -.03em; color: var(--muted); }
  .on .word { color: var(--text); }
  .nm { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; padding: 0 14px; font-size: var(--t-ui); font-weight: 600; }
  .nm :global(svg) { color: var(--accent-text); }
  .on .nm { font-weight: 700; }
  .mine { margin-left: auto; font-size: var(--t-fine); font-weight: 500; color: var(--muted); }
  .also { padding: 0 14px; font-size: var(--t-small); line-height: 1.5; color: var(--muted); }
  .st { display: flex; align-items: center; gap: 8px; padding: 4px 14px 0; font-family: var(--font-mono); font-size: var(--t-fine); color: var(--dim); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); flex: none; }
  .dot.off { background: transparent; border: 1.5px solid var(--muted); }

  .rows { display: grid; grid-template-columns: 120px minmax(0, 1fr); margin: 0; border-top: 1px solid var(--border); }
  .rows dt, .rows dd { margin: 0; padding: 11px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); line-height: 1.5; }
  .rows dt { color: var(--muted); padding-right: 16px; }
  .rows dd { color: var(--text); }

  /* The panel. */
  .who { display: grid; gap: 4px; padding: 8px 0 12px; }
  .pname { display: flex; align-items: center; gap: 6px; font-size: 20px; font-weight: 700; letter-spacing: -.015em; }
  .pname :global(svg) { color: var(--accent-text); }
  .pstat { display: flex; align-items: center; gap: 8px; font-size: var(--t-small); color: var(--muted); }
  .code { position: relative; min-width: 0; max-width: 100%; margin-top: 8px; overflow: hidden; border: 1px solid var(--border); background: var(--bg); }
  pre { margin: 0; padding: 12px; padding-right: 92px; overflow-x: auto; font-family: var(--font-mono); font-size: 12.5px; line-height: 1.7; }
  .copy { position: absolute; top: 6px; right: 6px; min-width: 76px; }
  .copy :global(svg) { color: var(--muted); }
  code { font-family: var(--font-mono); font-size: .92em; overflow-wrap: anywhere; }
  .steps { list-style: none; margin: 0; padding: 0; display: grid; }
  .steps li { display: grid; grid-template-columns: 24px minmax(0, 1fr); gap: 8px; padding: 9px 0; border-bottom: 1px solid var(--border);
    font-size: var(--t-small); line-height: 1.55; }
  .steps code { padding: 1px 4px; border-radius: var(--radius); background: var(--neutral-chip, var(--surface-2)); }
  .i { font-family: var(--font-mono); font-weight: 600; color: var(--accent-text); font-variant-numeric: tabular-nums; }
  @media (hover: hover) and (pointer: fine) { .cell a:hover { background: var(--row-hover); } }
  @media (max-width: 639.98px) { .rows { grid-template-columns: minmax(0, 1fr); } .rows dt { padding-bottom: 0; border-bottom: 0; } .rows dd { padding-top: 2px; } }
</style>
