<script lang="ts">
  // /developers: the cyex.tv API on the cyex docs layout (cyex-docs). Guides first, then the REST
  // endpoints from openapi.json, the event socket, and GraphQL from schema.graphql. Everything is
  // rendered from data files in $lib/docs; behaviour.ts adds the scroll-spy, filter and tabs.
  import '$lib/docs/docs.css';
  import { initDocs } from '$lib/docs/behaviour';
  import { defaultOpen } from '$lib/docs/groups';
  import { lazyGroups } from '$lib/docs/lazy-groups';
  import { isLazy } from '$lib/docs/lazy';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import { registerShortcuts } from '$lib/shell/shortcuts.svelte';
  import { DOCS_KEYS } from '$lib/docs/keys';
  import Code from '$lib/docs/Code.svelte';
  import Fields from '$lib/docs/Fields.svelte';
  import Inline from '$lib/docs/Inline.svelte';
  import Prose from '$lib/docs/Prose.svelte';
  import { guides } from '$lib/docs/guides';
  import { socketClient, socketExample, socketFrames, socketTopics, socketUrl } from '$lib/docs/events';
  import { paragraphs } from '$lib/docs/example';
  import { base } from '$app/paths';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  let root: HTMLDivElement;

  // Anchors of the old /developers/reference guides, so links into them still land.
  const OLD: Record<string, string> = { basics: 'start-here', pictures: 'emotes-and-files' };

  $effect(() => {
    const alias = OLD[decodeURIComponent(location.hash.slice(1))];
    if (alias) location.replace(`#${alias}`);
    const lazy = lazyGroups(root, data.known, data.base, base);
    const stop = initDocs(root, { load: lazy.load });
    return () => {
      stop();
      lazy.destroy();
    };
  });
  $effect(() => registerShortcuts(DOCS_KEYS));

  const wsHost = $derived(socketUrl.replace(/\/v1\/events$/, ''));
</script>

{#snippet chev()}
  <svg class="chev" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 1.5 6.5 5 3 8.5" /></svg>
{/snippet}
<!-- The header of a group, in the index and in the body. behaviour.ts owns aria-expanded and data-open;
     the markup is the default state (the notes open, the rest closed), so it reads right before the script runs. -->
{#snippet navHead(id: string, title: string, count: number)}
  <button type="button" class="group-toggle nav-group-title" data-group-toggle={id} aria-expanded={defaultOpen(id)} aria-controls="nav-{id}">
    {@render chev()}<span class="group-name" {title}>{title}</span><span class="group-count" data-total={count} aria-hidden="true">{count}</span>
  </button>
{/snippet}
{#snippet bodyHead(id: string, title: string, count: number)}
  <h2 class="op-group-title">
    <button type="button" class="group-toggle" data-group-toggle={id} aria-expanded={defaultOpen(id)} aria-controls="body-{id}">
      {@render chev()}<span class="group-name" {title}>{title}</span><span class="group-count" aria-hidden="true">{count}</span>
    </button>
  </h2>
{/snippet}

<svelte:head>
  <noscript><style>.docs .collapse { grid-template-rows: 1fr; visibility: visible; } .docs .collapse > .collapse-inner { content-visibility: visible; } .docs .nav-tools, .docs .group-toggle .chev { display: none; }</style></noscript>
  <title>API - cyex.tv</title>
  <meta name="description" content="The cyex.tv API: public reads for chat clients, GraphQL, and live events, with every endpoint, parameter and response." />
  <meta property="og:title" content="API reference" />
</svelte:head>

<div class="docs" bind:this={root}>
  <RailLayout>
    {#snippet rail()}
    <aside class="docs-nav" aria-label="API index">
      <div class="nav-search">
        <svg class="nav-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-4-4" /></svg>
        <input type="search" enterkeyhint="search" placeholder="Find an endpoint" aria-label="Find an endpoint" autocomplete="off" autocapitalize="off" spellcheck="false" />
        <kbd class="nav-kbd" aria-hidden="true">Ctrl K</kbd>
        <button type="button" class="input-clear" aria-label="Clear"><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2 2l6 6M8 2L2 8" /></svg></button>
      </div>
      <div class="nav-tools"><button type="button" class="group-all" data-group-all>Expand all</button></div>
      <details class="nav-toc">
      <summary>Contents</summary>
      <nav class="nav-scroll" aria-label="Contents">
        <div class="nav-group" data-group="notes" data-open={defaultOpen('notes')}>
          {@render navHead('notes', 'Notes', guides.length)}
          <div class="collapse" id="nav-notes"><div class="collapse-inner">
          {#each guides as g}
            <a class="nav-item guide" href="#{g.id}" data-search={g.search}><span class="nav-label">{g.title}</span></a>
          {/each}
          </div></div>
        </div>
        {#each data.nav as g}
          <div class="nav-group" data-group={g.id} data-open="false">
            {@render navHead(g.id, g.title, g.entries.length)}
            <div class="collapse" id="nav-{g.id}"><div class="collapse-inner">
            {#each g.entries as e}
              <a class="nav-item" href="#{e.id}" data-search={e.search}>
                <span class="nav-method {e.cls}">{e.chip}</span>{' '}<span class="nav-label">{e.label}</span>
              </a>
            {/each}
            </div></div>
          </div>
        {/each}
        <div class="nav-empty" hidden></div>
        <p class="sr-only" role="status" data-filter-status></p>
      </nav>
      </details>
    </aside>
    {/snippet}

    <div class="docs-body" id="api-contents" tabindex="-1">
      <header class="docs-title">
        <div>
          <h1>API</h1>
          <p>The cyex.tv API, as run at <code>{data.base}</code>. Reads are public.</p>
        </div>
        <span class="docs-stat">
          <span class="docs-stat-label">REST</span><span class="docs-stat-count">{data.restCount}</span>
          <span class="docs-stat-label">GraphQL</span><span class="docs-stat-count">{data.gqlCount}</span>
        </span>
      </header>

      <div class="op-group guides" data-group="notes" data-open={defaultOpen('notes')}>
        {@render bodyHead('notes', 'Notes', guides.length)}
        <div class="collapse" id="body-notes"><div class="collapse-inner">
      {#each guides as g}
        <section class="guide" id={g.id}>
          <h2>{g.title}</h2>
          {#each g.blocks as b}
            {#if b.t === 'lede'}
              <Prose paras={b.text} class="guide-lede" />
            {:else if b.t === 'p'}
              <Prose paras={b.text} />
            {:else if b.t === 'ol'}
              <ol>{#each b.items as item}<li><Inline paras={item} /></li>{/each}</ol>
            {:else if b.t === 'ul'}
              <ul>{#each b.items as item}<li><Inline paras={item} /></li>{/each}</ul>
            {:else if b.t === 'base'}
              <div class="base-url" class:rows={b.rows.length > 1}>
                {#each b.rows as [label, url]}<span class="base-url-label">{label}</span><code>{url}</code>{/each}
              </div>
            {:else if b.t === 'fmt'}
              <div class="fmt-grid" class:wide={!b.plain && b.rows.some(([k]) => k.length > 16)}>
                {#each b.rows as [k, v]}<div class="fmt">{#if b.plain}<b>{k}</b>{:else}<code>{k}</code>{/if}<span><Inline paras={v} /></span></div>{/each}
              </div>
            {:else if b.t === 'codes'}
              <div class="codes-wrap"><table class="codes">
                <thead><tr><th>Code</th><th>Meaning</th><th>When</th></tr></thead>
                <tbody>{#each b.rows as r (r.code)}<tr id="code-{r.code}"><td><code>{r.code}</code></td><td>{r.meaning}</td><td>{r.when}</td></tr>{/each}</tbody>
              </table></div>
            {:else if b.t === 'note'}
              <div class="note"><Prose paras={b.text} /></div>
            {:else if b.t === 'pre'}
              <pre><code>{b.code}</code></pre>
            {/if}
          {/each}
        </section>
      {/each}
        </div></div>
      </div>

      {#each data.nav as g}
        {#if isLazy(g.id)}
          <div class="op-group" data-group={g.id} data-open="false">
            {@render bodyHead(g.id, g.title, g.entries.length)}
            <div class="collapse" id="body-{g.id}" data-lazy><div class="collapse-inner"><noscript><p class="op-desc">This section needs JavaScript. The full reference is in <a href="https://github.com/cyex-tv/platform/blob/main/openapi/openapi.json">openapi.json</a> and <a href="https://github.com/cyex-tv/platform/blob/main/schema/schema.graphql">schema.graphql</a>.</p></noscript></div></div>
          </div>
        {:else if g.id === 'live'}
        <div class="op-group" data-group="live" data-open="false">
          {@render bodyHead('live', 'Live events', 1)}
          <div class="collapse" id="body-live"><div class="collapse-inner">
          <section class="op" id="ws-events">
            <div class="op-main">
              <div class="op-head">
                <span class="method m-ws">WS</span>
                <h3 class="op-title">Event socket</h3>
              </div>
              <div class="op-path"><span class="op-host">{wsHost}</span>/v1/events</div>
              <p class="op-desc">One connection carries many subscriptions, changed without reconnecting. Every frame is JSON, <code>{'{'} "op": "…", "d": {'{'} … {'}'} {'}'}</code>, so a client needs no GraphQL library. GraphQL clients get the same events from <a href="#gql-subscription-events">the events subscription</a>.</p>
  
              <div class="param-group">
                <div class="param-group-title">Topics</div>
                {#each socketTopics as [topic, d]}
                  <div class="param"><div class="param-sig"><code class="param-name">{topic}</code></div><div class="param-desc"><Inline paras={paragraphs(d)} /></div></div>
                {/each}
              </div>
  
              <div class="param-group">
                <div class="param-group-title">Frames</div>
                {#each socketFrames as [dir, op, d]}
                  <div class="param">
                    <div class="param-sig"><code class="param-name">{op}</code><span class="param-type">{dir}</span></div>
                    <div class="param-desc"><Inline paras={paragraphs(d)} /></div>
                  </div>
                {/each}
              </div>
  
              <div class="param-group">
                <div class="param-group-title">Event types</div>
                {#each data.events as e}
                  <div class="param"><div class="param-sig"><code class="param-name">{e.wire}</code></div>{#if e.description.length}<div class="param-desc"><Inline paras={e.description} /></div>{/if}</div>
                {/each}
              </div>
  
              <div class="param-group">
                <div class="param-group-title">Resume</div>
                <p class="op-desc">After a reconnect, send <code>resume</code> with the old <code>session_id</code> and the last event <code>id</code> you saw: missed events for the old topics are sent again, from the last 24 h. Anything older gets <code>resync</code>, which means fetch again what you show.</p>
              </div>
  
              <div class="param-group">
                <div class="param-group-title">Responses</div>
                <div class="resp-row"><span class="status-pip ok"></span><code class="resp-code">101</code><span class="resp-desc">Switching protocols: the socket is open and <code>hello</code> follows.</span></div>
                <div class="resp-row"><span class="status-pip err"></span><code class="resp-code">429</code><span class="resp-desc">Over 60 connections a minute from this IP.</span></div>
              </div>
            </div>
            <div class="op-aside">
              <div class="code-panel">
                <Code label="Client" meta="JavaScript" code={socketClient} copy="Copy client" />
                <Code label="Frames" meta="→ sent, ← received" code={socketExample} copy="Copy frames" />
              </div>
            </div>
          </section>
          </div></div>
        </div>
        {/if}
      {/each}

      <p class="docs-end">
        <a href="https://github.com/cyex-tv/platform/blob/main/openapi/openapi.json">openapi.json</a> · <a href="https://github.com/cyex-tv/platform/blob/main/schema/schema.graphql">schema.graphql</a> · <a href="https://github.com/cyex-tv/platform">Source</a>
      </p>
    </div>
  </RailLayout>
  <button type="button" class="to-top" hidden aria-label="Back to top">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 19V5M5 12l7-7 7 7" /></svg>
    Top
  </button>
</div>
