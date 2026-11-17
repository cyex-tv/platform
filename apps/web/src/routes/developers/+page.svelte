<script lang="ts">
  // /developers: the cyex.tv API on the cyex docs layout (cyex-docs). Guides first, then the REST
  // endpoints from openapi.json, the event socket, and GraphQL from schema.graphql. Everything is
  // rendered from data files in $lib/docs; behaviour.ts adds the scroll-spy, filter and tabs.
  import '$lib/docs/docs.css';
  import { initDocs } from '$lib/docs/behaviour';
  import { defaultOpen, restGroupId } from '$lib/docs/groups';
  import RailLayout from '$lib/shell/RailLayout.svelte';
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
</script>

{#snippet chev()}
  <svg class="chev" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 1.5 6.5 5 3 8.5" /></svg>
{/snippet}
<!-- The header of a group, in the index and in the body. behaviour.ts owns aria-expanded and data-open;
     the markup is the default state (the notes open, the rest closed), so it reads right before the script runs. -->
{#snippet navHead(id: string, title: string, count: number)}
  <button type="button" class="group-toggle nav-group-title" data-group-toggle={id} aria-expanded={defaultOpen(id)} aria-controls="nav-{id}">
    {@render chev()}<span class="group-name">{title}</span><span class="group-count" data-total={count} aria-hidden="true">{count}</span>
  </button>
{/snippet}
{#snippet bodyHead(id: string, title: string, count: number)}
  <h2 class="op-group-title">
    <button type="button" class="group-toggle" data-group-toggle={id} aria-expanded={defaultOpen(id)} aria-controls="body-{id}">
      {@render chev()}<span class="group-name">{title}</span><span class="group-count" aria-hidden="true">{count}</span>
    </button>
  </h2>
{/snippet}

<svelte:head>
  <noscript><style>.docs .collapse { grid-template-rows: 1fr; visibility: visible; } .docs .nav-tools, .docs .group-toggle .chev { display: none; }</style></noscript>
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
        {#each data.rest as g}
          <div class="nav-group" data-group={restGroupId(g.tag)} data-open="false">
            {@render navHead(restGroupId(g.tag), g.title, g.ops.length)}
            <div class="collapse" id="nav-{restGroupId(g.tag)}"><div class="collapse-inner">
            {#each g.ops as op}
              <a class="nav-item" href="#{op.id}" data-search="{op.path} rest">
                <span class="nav-method m-{op.method.toLowerCase()}">{op.method}</span>{' '}<span class="nav-label">{op.title}</span>
              </a>
            {/each}
            </div></div>
          </div>
        {/each}
        <div class="nav-group" data-group="live" data-open="false">
          {@render navHead('live', 'Live events', 1)}
          <div class="collapse" id="nav-live"><div class="collapse-inner">
          <a class="nav-item" href="#ws-events" data-search="/v1/events websocket socket">
            <span class="nav-method m-ws">WS</span>{' '}<span class="nav-label">Event socket</span>
          </a>
          </div></div>
        </div>
        {#each gqlGroups as g}
          <div class="nav-group" data-group="gql-{g.id}" data-open="false">
            {@render navHead(`gql-${g.id}`, g.title, g.ops.length)}
            <div class="collapse" id="nav-gql-{g.id}"><div class="collapse-inner">
            {#each g.ops as op}
              <a class="nav-item" href="#{op.id}" data-search="graphql {op.kind}">
                <span class="nav-method {chipClass[op.kind]}">{chip[op.kind]}</span>{' '}<span class="nav-label">{op.name}</span>
              </a>
            {/each}
            </div></div>
          </div>
        {/each}
        <div class="nav-group" data-group="gql-types" data-open="false">
          {@render navHead('gql-types', 'GraphQL types', data.gql.types.length)}
          <div class="collapse" id="nav-gql-types"><div class="collapse-inner">
          {#each data.gql.types as t}
            <a class="nav-item" href="#{t.id}" data-search="graphql type {t.kind}">
              <span class="nav-method m-get">{typeChip(t)}</span>{' '}<span class="nav-label">{t.name}</span>
            </a>
          {/each}
          </div></div>
        </div>
        <div class="nav-empty" hidden></div>
        <p class="sr-only" role="status" data-filter-status></p>
      </nav>
      </details>
    </aside>
    {/snippet}

    <div class="docs-body">
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

      {#each data.rest as g}
        <div class="op-group" data-group={restGroupId(g.tag)} data-open="false">
          {@render bodyHead(restGroupId(g.tag), g.title, g.ops.length)}
          <div class="collapse" id="body-{restGroupId(g.tag)}"><div class="collapse-inner">
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
          </div></div>
        </div>
      {/each}

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
        </div></div>
      </div>

      {#each gqlGroups as g}
        <div class="op-group" data-group="gql-{g.id}" data-open="false">
          {@render bodyHead(`gql-${g.id}`, g.title, g.ops.length)}
          <div class="collapse" id="body-gql-{g.id}"><div class="collapse-inner">
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
          </div></div>
        </div>
      {/each}

      <div class="op-group" data-group="gql-types" data-open="false">
        {@render bodyHead('gql-types', 'GraphQL types', data.gql.types.length)}
        <div class="collapse" id="body-gql-types"><div class="collapse-inner">
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
                <div class="fields">
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
            </div>
          </section>
        {/each}
        </div></div>
      </div>

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
