<script lang="ts">
  // /developers: the cyex.tv API on the cyex docs layout (cyex-docs). Guides first, then the REST
  // endpoints from openapi.json, the event socket, and GraphQL from schema.graphql. Everything is
  // rendered from data files in $lib/docs; behaviour.ts adds the scroll-spy, filter and tabs.
  import '../../tool.css';
  import '$lib/docs/docs.css';
  import { initDocs } from '$lib/docs/behaviour';
  import Code from '$lib/docs/Code.svelte';
  import Fields from '$lib/docs/Fields.svelte';
  import Inline from '$lib/docs/Inline.svelte';
  import Prose from '$lib/docs/Prose.svelte';
  import TypeRef from '$lib/docs/TypeRef.svelte';
  import { guides } from '$lib/docs/guides';
  import { eventTypes, socketClient, socketExample, socketFrames, socketTopics, socketUrl, subscriptionFrames } from '$lib/docs/events';
  import { paragraphs } from '$lib/docs/example';
  import type { GqlOp, GqlType } from '$lib/docs/graphql';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  let root: HTMLDivElement;

  // Anchors of the old /developers/reference guides, so links into them still land.
  const OLD: Record<string, string> = { basics: 'start-here', pictures: 'emotes-and-files' };

  $effect(() => {
    const alias = OLD[decodeURIComponent(location.hash.slice(1))];
    if (alias) location.replace(`#${alias}`);
    return initDocs(root);
  });

  const known = $derived(data.gql.types.map((t) => t.name));
  const restCount = $derived(data.rest.reduce((n, g) => n + g.ops.length, 0));
  const gqlCount = $derived(data.gql.queries.length + data.gql.mutations.length + data.gql.subscriptions.length);
  const events = $derived(eventTypes(data.gql));
  const gqlGroups = $derived([
    { id: 'queries', title: 'GraphQL queries', ops: data.gql.queries },
    { id: 'mutations', title: 'GraphQL mutations', ops: data.gql.mutations },
    { id: 'subscriptions', title: 'GraphQL subscriptions', ops: data.gql.subscriptions },
  ]);
  const chip = { query: 'QUERY', mutation: 'MUT', subscription: 'SUB' } as const;
  const chipLong = { query: 'QUERY', mutation: 'MUTATION', subscription: 'SUBSCRIPTION' } as const;
  const chipClass = { query: 'm-get', mutation: 'm-post', subscription: 'm-ws' } as const;
  const typeChip = (t: GqlType) => t.kind.toUpperCase().replace('OBJECT', 'TYPE');
  const args = (op: GqlOp) => (op.args.length ? `(${op.args.map((a) => `${a.name}: ${a.type}`).join(', ')})` : '');
  const accessLabel = { 'signed-in': 'Signed in', restricted: 'Restricted' } as const;
  const accessHint = { 'signed-in': 'Needs a session', restricted: 'Needs a session and a permission' } as const;
  const wsHost = $derived(socketUrl.replace(/\/v1\/events$/, ''));

  // The theme toggle of the docs header (cyex-design): instant under reduced motion.
  function toggleTheme() {
    const apply = () => {
      const html = document.documentElement;
      const light = html.dataset.theme !== 'light';
      if (light) html.dataset.theme = 'light';
      else delete html.dataset.theme;
      try {
        localStorage.setItem('cyexTheme', light ? 'light' : 'dark');
      } catch {
        /* not remembered */
      }
    };
    const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
    if (!matchMedia('(prefers-reduced-motion: reduce)').matches && doc.startViewTransition) doc.startViewTransition(apply);
    else apply();
  }
</script>

<svelte:head>
  <title>API - cyex.tv</title>
  <meta name="description" content="The cyex.tv API: public reads for chat clients, GraphQL, and live events, with every endpoint, parameter and response." />
  <meta property="og:title" content="API reference" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600&family=IBM+Plex+Sans:wght@400;500;600;700&display=swap" />
</svelte:head>

<div class="tool docs" bind:this={root}>
  <a class="skip" href="#main">Skip to content</a>
  <header class="top">
    <div class="top-inner">
      <a class="brand" href="/" aria-label="cyex.tv front page">
        <span class="mark" aria-hidden="true"><i></i><i></i><i></i></span>
        <b>cyex.tv</b>
      </a>
      <nav class="links" aria-label="Pages">
        <a href="/search">Emotes</a>
        <a href="/#install">Extension</a>
        <a href="/developers" aria-current="page">Docs</a>
      </nav>
      <span class="top-spacer"></span>
      <button class="theme-toggle" type="button" aria-label="Switch theme" title="Switch theme" onclick={toggleTheme}>
        <svg class="icon-moon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" /></svg>
        <svg class="icon-sun" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" /></svg>
      </button>
    </div>
  </header>

  <div class="docs-shell">
    <aside class="docs-nav">
      <div class="nav-search">
        <svg class="nav-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-4-4" /></svg>
        <input type="text" placeholder="Find an endpoint" aria-label="Find an endpoint" autocomplete="off" spellcheck="false" />
        <kbd class="nav-kbd">Ctrl K</kbd>
        <button type="button" class="input-clear" aria-label="Clear"><svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2 2l6 6M8 2L2 8" /></svg></button>
      </div>
      <nav class="nav-scroll" aria-label="Contents">
        <div class="nav-group">
          <div class="nav-group-title">Notes</div>
          {#each guides as g}
            <a class="nav-item guide" href="#{g.id}" data-search={g.search}><span class="nav-label">{g.title}</span></a>
          {/each}
        </div>
        {#each data.rest as g}
          <div class="nav-group">
            <div class="nav-group-title">{g.title}</div>
            {#each g.ops as op}
              <a class="nav-item" href="#{op.id}" data-search="{op.path} rest">
                <span class="nav-method m-{op.method.toLowerCase()}">{op.method}</span><span class="nav-label">{op.title}</span>
              </a>
            {/each}
          </div>
        {/each}
        <div class="nav-group">
          <div class="nav-group-title">Live events</div>
          <a class="nav-item" href="#ws-events" data-search="/v1/events websocket socket">
            <span class="nav-method m-ws">WS</span><span class="nav-label">Event socket</span>
          </a>
        </div>
        {#each gqlGroups as g}
          <div class="nav-group">
            <div class="nav-group-title">{g.title}</div>
            {#each g.ops as op}
              <a class="nav-item" href="#{op.id}" data-search="graphql {op.kind}">
                <span class="nav-method {chipClass[op.kind]}">{chip[op.kind]}</span><span class="nav-label">{op.name}</span>
              </a>
            {/each}
          </div>
        {/each}
        <div class="nav-group">
          <div class="nav-group-title">GraphQL types</div>
          {#each data.gql.types as t}
            <a class="nav-item" href="#{t.id}" data-search="graphql type {t.kind}">
              <span class="nav-method m-get">{typeChip(t)}</span><span class="nav-label">{t.name}</span>
            </a>
          {/each}
        </div>
        <div class="nav-empty" hidden></div>
      </nav>
    </aside>

    <main class="docs-body" id="main" tabindex="-1">
      <header class="docs-title">
        <div>
          <h1>API</h1>
          <p>The cyex.tv API, as run at <code>{data.base}</code>. Reads are public.</p>
        </div>
        <span class="docs-stat">
          <span class="docs-stat-label">REST</span><span class="docs-stat-count">{restCount}</span>
          <span class="docs-stat-label">GraphQL</span><span class="docs-stat-count">{gqlCount}</span>
        </span>
      </header>

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
            {:else if b.t === 'note'}
              <div class="note"><Prose paras={b.text} /></div>
            {:else if b.t === 'pre'}
              <pre><code>{b.code}</code></pre>
            {/if}
          {/each}
        </section>
      {/each}

      {#each data.rest as g}
        <div class="op-group">
          <h2 class="op-group-title">{g.title}</h2>
          {#each g.ops as op}
            <section class="op" id={op.id}>
              <div class="op-main">
                <div class="op-head">
                  <span class="method m-{op.method.toLowerCase()}">{op.method}</span>
                  <h3 class="op-title">{op.title}</h3>
                  {#if op.signedIn}<span class="auth-chip neutral" title="Needs a session">Signed in</span>{/if}
                </div>
                <div class="op-path"><span class="op-host">{data.base}</span>{op.path}</div>
                {#if op.summaryRest.length || op.description.length}
                  <div class="op-desc"><Prose paras={[...op.summaryRest, ...op.description]} /></div>
                {/if}
                <Fields title="Path parameters" fields={op.pathParams} />
                <Fields title="Query parameters" fields={op.queryParams} />
                <Fields title="Headers" fields={op.headerParams} />
                {#if op.bodyFields.length}
                  <Fields title={`Request body · ${op.bodyType}`} fields={op.bodyFields} />
                {/if}
                <div class="param-group">
                  <div class="param-group-title">Responses</div>
                  {#each op.responses as r}
                    <div class="resp-row">
                      <span class="status-pip" class:ok={r.ok} class:err={!r.ok}></span>
                      <code class="resp-code">{r.status}</code>
                      <span class="resp-desc">{#if r.description.length}<Inline paras={r.description} />{/if}</span>
                    </div>
                  {/each}
                </div>
              </div>
              <div class="op-aside">
                <div class="code-panel">
                  <Code tabs={op.samples} copy="Copy request" />
                  <Code label={op.responseStatus} meta={op.responseType} code={op.responseExample} ok copy="Copy response" />
                </div>
              </div>
            </section>
          {/each}
        </div>
      {/each}

      <div class="op-group">
        <h2 class="op-group-title">Live events</h2>
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
              {#each events as e}
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
      </div>

      {#each gqlGroups as g}
        <div class="op-group">
          <h2 class="op-group-title">{g.title}</h2>
          {#each g.ops as op}
            <section class="op" id={op.id}>
              <div class="op-main">
                <div class="op-head">
                  <span class="method {chipClass[op.kind]}">{chipLong[op.kind]}</span>
                  <h3 class="op-title">{op.name}</h3>
                  {#if op.access !== 'public'}
                    <span class="auth-chip" class:neutral={op.access === 'signed-in'} title={accessHint[op.access]}>{accessLabel[op.access]}</span>
                  {/if}
                  {#if op.deprecated}<span class="auth-chip neutral">Deprecated</span>{/if}
                </div>
                <div class="op-path"><span class="op-host">{`${op.kind} `}</span>{op.name}{args(op)}<span class="op-host">: </span>{op.returns}</div>
                {#if op.description.length}<div class="op-desc"><Prose paras={op.description} /></div>{/if}
                {#if op.deprecated}<p class="op-desc">{op.deprecated}</p>{/if}
                {#if op.args.length}
                  <div class="param-group">
                    <div class="param-group-title">Arguments</div>
                    {#each op.args as a}
                      <div class="param">
                        <div class="param-sig">
                          <code class="param-name">{a.name}</code>
                          <span class="param-type"><TypeRef type={a.type} {known} /></span>
                          {#if a.required}<span class="param-req">required</span>{/if}
                          {#if a.default}<span class="param-default">default <code>{a.default}</code></span>{/if}
                        </div>
                        {#if a.description.length}<div class="param-desc"><Prose paras={a.description} /></div>{/if}
                      </div>
                    {/each}
                  </div>
                {/if}
                <div class="param-group">
                  <div class="param-group-title">Returns</div>
                  <div class="resp-row"><span class="status-pip ok"></span><code class="resp-code"><TypeRef type={op.returns} {known} /></code></div>
                </div>
              </div>
              <div class="op-aside">
                <div class="code-panel">
                  {#if op.kind === 'subscription'}
                    <Code label="Operation" meta="GraphQL" code={op.document} copy="Copy operation" />
                    <Code label="Frames" meta="→ sent, ← received" code={subscriptionFrames(op, data.gql.socket)} copy="Copy frames" />
                  {:else}
                    <Code tabs={op.samples} copy="Copy request" />
                    <Code label="200" meta="application/json" code={op.response} ok copy="Copy response" />
                  {/if}
                </div>
              </div>
            </section>
          {/each}
        </div>
      {/each}

      <div class="op-group">
        <h2 class="op-group-title">GraphQL types</h2>
        {#each data.gql.types as t}
          <section class="op type" id={t.id}>
            <div class="op-main">
              <div class="op-head">
                <span class="method m-get">{typeChip(t)}</span>
                <h3 class="op-title">{t.name}</h3>
              </div>
              {#if t.description.length}<div class="op-desc"><Prose paras={t.description} /></div>{/if}
              <div class="param-group">
                <div class="param-group-title">{t.kind === 'enum' ? 'Values' : 'Fields'}</div>
                {#each t.members as m}
                  <div class="param">
                    <div class="param-sig">
                      <code class="param-name">{m.name}</code>
                      {#if m.type}<span class="param-type"><TypeRef type={m.type} {known} /></span>{/if}
                      {#if m.required}<span class="param-req">required</span>{/if}
                    </div>
                    {#if m.description.length}<div class="param-desc"><Prose paras={m.description} /></div>{/if}
                  </div>
                {/each}
              </div>
            </div>
          </section>
        {/each}
      </div>

      <footer class="docs-foot">
        cyex.tv is open source under AGPL-3.0. Not affiliated with, endorsed by, or sponsored by Twitch or Kick.<br />
        <a href="/">cyex.tv</a> · <a href="https://github.com/cyex-tv/platform">Source</a> · <a href="https://github.com/cyex-tv/platform/blob/main/openapi/openapi.json">openapi.json</a> · <a href="https://github.com/cyex-tv/platform/blob/main/schema/schema.graphql">schema.graphql</a>
      </footer>
    </main>
  </div>
</div>
