<script lang="ts">
  import '$lib/docs/docs.css';
  import { onMount } from 'svelte';
  import { initDocs } from '$lib/docs/behaviour';
  import Code from '$lib/docs/Code.svelte';
  import Fields from '$lib/docs/Fields.svelte';
  import Prose from '$lib/docs/Prose.svelte';
  import TypeRef from '$lib/docs/TypeRef.svelte';
  import type { GqlOp } from '$lib/docs/graphql';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  let root: HTMLDivElement;

  onMount(() => initDocs(root));

  const known = $derived(data.gql.types.map((t) => t.name));
  const restCount = $derived(data.rest.reduce((n, g) => n + g.ops.length, 0));
  const gqlCount = $derived(data.gql.queries.length + data.gql.mutations.length + data.gql.subscriptions.length);
  const eventTypes = $derived(
    (data.gql.types.find((t) => t.name === 'EventType')?.members ?? []).map((m) => ({
      // SET_EMOTE_ADDED is written set.emote_added on the event socket.
      wire: m.name.toLowerCase().replace('_', '.'),
      description: m.description,
    })),
  );
  const gqlGroups = $derived([
    { title: 'Queries', chip: 'QUERY', cls: 'm-get', ops: data.gql.queries },
    { title: 'Mutations', chip: 'MUT', cls: 'm-post', ops: data.gql.mutations },
    { title: 'Subscriptions', chip: 'SUB', cls: 'm-ws', ops: data.gql.subscriptions },
  ]);
  const chipOf = (kind: GqlOp['kind']) => ({ query: 'QUERY', mutation: 'MUTATION', subscription: 'SUB' })[kind];
  const chipClass = (kind: GqlOp['kind']) => ({ query: 'm-get', mutation: 'm-post', subscription: 'm-ws' })[kind];
  const title = (s: string) => s[0].toUpperCase() + s.slice(1);
  const signature = (op: GqlOp) => `${op.name}${op.args.length ? `(${op.args.map((a) => `${a.name}: ${a.type}`).join(', ')})` : ''}: ${op.returns}`;

  // graphql-transport-ws frames for a subscription, as a client sends and receives them.
  function frames(op: GqlOp): string {
    const payload = { query: op.document.replace(/\s+/g, ' '), variables: op.variables ? JSON.parse(op.variables) : {} };
    const next = { type: 'next', id: '1', payload: JSON.parse(op.response) };
    return [
      `// connect to ${data.gql.socket} with the "graphql-transport-ws" subprotocol`,
      `→ ${JSON.stringify({ type: 'connection_init' })}`,
      `← ${JSON.stringify({ type: 'connection_ack' })}`,
      `→ ${JSON.stringify({ type: 'subscribe', id: '1', payload })}`,
      `← ${JSON.stringify(next)}`,
    ].join('\n');
  }

  const socketUrl = $derived(`${data.base.replace(/^http/, 'ws')}/v1/events`);
  const socketFrames = [
    ['server → client', 'hello', 'session_id, heartbeat_interval (ms), max_subscriptions'],
    ['client → server', 'subscribe', 'topics: the topics to add'],
    ['client → server', 'unsubscribe', 'topics: the topics to drop'],
    ['server → client', 'ack', 'op, topics, and an error code for each topic that was refused'],
    ['server → client', 'dispatch', 'id (the event ID), type, data'],
    ['both', 'heartbeat', '{}'],
    ['client → server', 'resume', 'session_id, last_event_id'],
    ['server → client', 'reconnect', 'The server is restarting: reconnect and resume.'],
    ['server → client', 'error', 'code, message; then the connection closes.'],
  ];
  const socketClient = $derived(`const ws = new WebSocket('${socketUrl}');
let sessionId = null;
let lastEventId = null;

const send = (op, d = {}) => ws.send(JSON.stringify({ op, d }));

ws.onmessage = ({ data }) => {
  const { op, d } = JSON.parse(data);
  if (op === 'hello') {
    if (sessionId && lastEventId) {
      // A reconnect: replay what was missed on the old topics.
      send('resume', { session_id: sessionId, last_event_id: lastEventId });
    } else {
      send('subscribe', { topics: ['channel:twitch:12345678', 'set:123'] });
    }
    sessionId = d.session_id;
  } else if (op === 'dispatch') {
    lastEventId = d.id;
    console.log(d.type, d.data);
  } else if (op === 'heartbeat') {
    send('heartbeat');
  } else if (op === 'reconnect') {
    ws.close();
  }
};`);
  const socketExample = `// the fields inside data depend on the event type
← { "op": "hello", "d": { "session_id": "s_8f2c", "heartbeat_interval": 20000, "max_subscriptions": 500 } }
→ { "op": "subscribe", "d": { "topics": ["channel:twitch:12345678", "set:123"] } }
← { "op": "ack", "d": { "op": "subscribe", "topics": ["channel:twitch:12345678", "set:123"] } }
← { "op": "dispatch", "d": { "id": "1024", "type": "set.emote_added", "data": { "set_id": "123", "emote_id": "456", "name": "pog" } } }
→ { "op": "resume", "d": { "session_id": "s_8f2c", "last_event_id": "1024" } }`;
</script>

<svelte:head>
  <title>API reference - cyex.tv</title>
  <meta name="description" content="Every cyex.tv REST endpoint and GraphQL operation, and the event socket, with parameters, responses and samples." />
  <meta property="og:title" content="API reference" />
</svelte:head>

<div class="docs" bind:this={root}>
  <div class="docs-shell">
    <aside class="docs-nav">
      <div class="nav-search">
        <svg class="nav-search-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14" stroke-linecap="round"/></svg>
        <input type="text" placeholder="Find an endpoint" aria-label="Find an endpoint" autocomplete="off" spellcheck="false" />
        <span class="nav-kbd" aria-hidden="true">Ctrl K</span>
        <button type="button" class="input-clear" aria-label="Clear"><svg viewBox="0 0 8 8" aria-hidden="true"><path d="M1 1l6 6M7 1L1 7"/></svg></button>
      </div>
      <nav class="nav-scroll" aria-label="Reference">
        <div class="nav-group">
          <div class="nav-group-title">Notes</div>
          <a class="nav-item guide" href="#basics"><span class="nav-label">Basics</span></a>
          <a class="nav-item guide" href="#signing-in"><span class="nav-label">Signing in</span></a>
          <a class="nav-item guide" href="#errors"><span class="nav-label">Errors and caching</span></a>
          <a class="nav-item guide" href="#pictures"><span class="nav-label">Emote pictures</span></a>
        </div>
        {#each data.rest as g}
          <div class="nav-group">
            <div class="nav-group-title">REST: {g.title}</div>
            {#each g.ops as op}
              <a class="nav-item" href="#{op.id}" data-search="{op.path} rest">
                <span class="nav-method m-{op.method.toLowerCase()}">{op.method}</span><span class="nav-label">{op.title}</span>
              </a>
            {/each}
          </div>
        {/each}
        <div class="nav-group">
          <div class="nav-group-title">Live updates</div>
          <a class="nav-item" href="#ws-events" data-search="/v1/events websocket socket">
            <span class="nav-method m-ws">WS</span><span class="nav-label">Event socket</span>
          </a>
        </div>
        {#each gqlGroups as g}
          <div class="nav-group">
            <div class="nav-group-title">GraphQL: {g.title}</div>
            {#each g.ops as op}
              <a class="nav-item" href="#{op.id}" data-search="graphql {op.kind}">
                <span class="nav-method {g.cls}">{g.chip}</span><span class="nav-label">{op.name}</span>
              </a>
            {/each}
          </div>
        {/each}
        <div class="nav-group">
          <div class="nav-group-title">GraphQL: Types</div>
          {#each data.gql.types as t}
            <a class="nav-item" href="#{t.id}" data-search="graphql type {t.kind}"><span class="nav-label">{t.name}</span></a>
          {/each}
        </div>
        <div class="nav-empty" hidden></div>
      </nav>
    </aside>

    <div class="docs-body">
      <div class="docs-title">
        <div>
          <h1>API reference</h1>
          <p>The cyex.tv API as run at <code>{data.base}</code>. Reads are public.</p>
        </div>
        <div class="docs-stat">
          <span class="docs-stat-label">REST</span><span class="docs-stat-count">{restCount}</span>
          <span class="docs-stat-label">GraphQL</span><span class="docs-stat-count">{gqlCount}</span>
        </div>
      </div>

      <section class="guide" id="basics">
        <h2>Basics</h2>
        <p class="guide-lede">Three ways in, one data model: REST for the reads a chat client polls, GraphQL for everything else, and a socket for live changes.</p>
        <div class="base-url"><span class="base-url-label">Base URL</span><code>{data.base}</code></div>
        <div class="fmt-grid">
          <div class="fmt"><code>/v1/…</code><span>REST: the cacheable reads, sign-in and uploads. JSON in snake_case.</span></div>
          <div class="fmt"><code>/v1/gql</code><span>GraphQL: <code>POST</code> runs queries and mutations; open it in a browser to explore. Fields are camelCase.</span></div>
          <div class="fmt"><code>/v1/events</code><span>The event socket, for chat clients. <code>/v1/gql/ws</code> carries the same events as GraphQL subscriptions.</span></div>
        </div>
        <p>Everything lives under <code>/v1</code>. Inside v1 we only add fields, types and endpoints; fields are marked deprecated before they go, and only go in v2.</p>
        <div class="note"><strong>Chat client?</strong> Fetch a channel and its active set over REST, then keep it current on the event socket. Both are public and need no key.</div>
      </section>

      <section class="guide" id="signing-in">
        <h2>Signing in</h2>
        <p>Public reads need nothing. Anything that acts for an account (uploads, sets, moderation) needs a session, which cyex.tv sets as a cookie after sign-in with Twitch or Kick.</p>
        <p>A request that carries the cookie must come from <code>https://cyex.tv</code>; from any other site it is refused with <code>bad_origin</code>. Admin and moderation operations also need a named permission, and answer <code>signed_out</code> or <code>forbidden</code> without it.</p>
      </section>

      <section class="guide" id="errors">
        <h2>Errors and caching</h2>
        <p>REST errors are <code>{'{'} "error": "not_found", "message": "…" {'}'}</code>. <code>error</code> is a stable code to match on; <code>message</code> is for people. GraphQL errors carry the same codes in <code>extensions.code</code>.</p>
        <p>GraphQL queries are limited to a depth of 12 and a complexity of 1.000.</p>
        <p>The chat-client reads send an <code>ETag</code>. Send it back as <code>If-None-Match</code> and you get a cheap <code>304</code> while nothing changed.</p>
      </section>

      <section class="guide" id="pictures">
        <h2>Emote pictures</h2>
        <p>An emote has a <code>host</code> and a list of files. Each file is at <code>&lt;host&gt;/&lt;name&gt;</code>, for example <code>2x.webp</code>. Still emotes also come as <code>.avif</code>; animated ones also have <code>&lt;n&gt;x_static.webp</code>, the first frame.</p>
      </section>

      <h2 class="op-group-title">REST</h2>
      {#each data.rest as g}
        <h3 class="op-group-title sub">{g.title}{#if g.description}<span>{g.description}</span>{/if}</h3>
        {#each g.ops as op}
          <section class="op" id={op.id}>
            <div class="op-main">
              <div class="op-head">
                <span class="method m-{op.method.toLowerCase()}">{op.method}</span>
                <h3 class="op-title">{op.title}</h3>
                {#if op.signedIn}<span class="auth-chip">Signed in</span>{/if}
              </div>
              <div class="op-path"><span class="op-host">{data.base}</span>{op.path}</div>
              {#if op.summaryRest.length}<div class="op-desc"><Prose paras={op.summaryRest} /></div>{/if}
              {#if op.description.length}<div class="op-desc"><Prose paras={op.description} /></div>{/if}
              <Fields title="Path" fields={op.pathParams} />
              <Fields title="Query" fields={op.queryParams} />
              <Fields title="Headers" fields={op.headerParams} />
              {#if op.bodyFields.length}
                <Fields title={`Request body (${op.bodyType}${op.bodyRequired ? ', required' : ''})`} fields={op.bodyFields} />
              {/if}
              <div class="param-group">
                <div class="param-group-title">Responses</div>
                {#each op.responses as r}
                  <div class="resp-row">
                    <span class="status-pip" class:ok={r.ok} class:err={!r.ok}></span>
                    <code class="resp-code">{r.status}</code>
                    <span class="resp-desc">{#if r.description.length}<Prose paras={r.description} />{/if}</span>
                  </div>
                {/each}
              </div>
            </div>
            <div class="op-aside">
              <div class="code-panel">
                <Code tabs={op.samples} />
                {#if op.responseExample}
                  <Code label={op.responseStatus} meta={op.responseType} code={op.responseExample} />
                {/if}
              </div>
            </div>
          </section>
        {/each}
      {/each}

      <h2 class="op-group-title">Live updates</h2>
      <section class="op" id="ws-events">
        <div class="op-main">
          <div class="op-head">
            <span class="method m-ws">WS</span>
            <h3 class="op-title">Event socket</h3>
          </div>
          <div class="op-path"><span class="op-host">{socketUrl.replace(/\/v1\/events$/, '')}</span>/v1/events</div>
          <p class="op-desc">One connection carries many subscriptions, and you change them without reconnecting. Every frame is JSON, <code>{'{'} "op": "&lt;name&gt;", "d": {'{'} … {'}'} {'}'}</code>, so a client needs no GraphQL library. For GraphQL clients the same events arrive on <a href="#gql-subscription-events">the <code>events</code> subscription</a>.</p>

          <div class="param-group">
            <div class="param-group-title">Frames</div>
            {#each socketFrames as [dir, op, d]}
              <div class="param">
                <div class="param-sig"><code class="param-name">{op}</code><span class="param-type">{dir}</span></div>
                <div class="param-desc">{d}</div>
              </div>
            {/each}
          </div>

          <div class="param-group">
            <div class="param-group-title">Subscribe</div>
            <p class="op-desc">Send <code>subscribe</code> with topics. The <code>ack</code> lists the topics it took and an <code>error</code> code for each it refused.</p>
            <div class="param"><code class="param-name">channel:&lt;platform&gt;:&lt;id&gt;</code><div class="param-desc">Changes to a channel: its active set, and the emotes in it. <code>platform</code> is <code>twitch</code> or <code>kick</code>.</div></div>
            <div class="param"><code class="param-name">set:&lt;id&gt;</code><div class="param-desc">Changes to one set.</div></div>
            <div class="param"><code class="param-name">user:&lt;platform&gt;:&lt;id&gt;</code><div class="param-desc">A user's personal set.</div></div>
          </div>

          <div class="param-group">
            <div class="param-group-title">Dispatch</div>
            <p class="op-desc">Each change arrives as a <code>dispatch</code> with a monotonically increasing <code>id</code>, a <code>type</code> and its <code>data</code>.</p>
            {#each eventTypes as e}
              <div class="param"><code class="param-name">{e.wire}</code>{#if e.description.length}<div class="param-desc"><Prose paras={e.description} /></div>{/if}</div>
            {/each}
          </div>

          <div class="param-group">
            <div class="param-group-title">Keep it alive and resume</div>
            <p class="op-desc">The server sends a <code>heartbeat</code> every 20 s; a client that misses 3 should reconnect. After a reconnect, <code>resume</code> with your old <code>session_id</code> and the last event <code>id</code> you saw: missed events for the old topics replay for 24 h. Anything older gets a <code>resync</code>, which means refetch what you show.</p>
          </div>

          <div class="param-group">
            <div class="param-group-title">Limits</div>
            <div class="param"><code class="param-name">500</code><div class="param-desc">topics per connection.</div></div>
            <div class="param"><code class="param-name">10</code><div class="param-desc">frames per second from a client.</div></div>
            <div class="param"><code class="param-name">16 KB</code><div class="param-desc">per frame.</div></div>
          </div>
        </div>
        <div class="op-aside">
          <div class="code-panel">
            <Code label="Client" meta="JavaScript" code={socketClient} />
            <Code label="Frames" meta="→ sent, ← received" code={socketExample} />
          </div>
        </div>
      </section>

      <h2 class="op-group-title">GraphQL</h2>
      <section class="guide" id="graphql">
        <h2>Using GraphQL</h2>
        <p>Send queries and mutations as <code>POST {data.gql.endpoint}</code> with a JSON body of <code>query</code> and <code>variables</code>. Open the same address in a browser for GraphiQL. Introspection is on. Operations that need a session or a permission are in the schema too, and answer <code>signed_out</code> or <code>forbidden</code> without them. Subscriptions use <code>graphql-transport-ws</code> at <code>{data.gql.socket}</code>.</p>
        <p>The samples below are built from <a href="https://github.com/cyex-tv/platform/blob/main/schema/schema.graphql">schema.graphql</a>, and the REST ones from <a href="https://github.com/cyex-tv/platform/blob/main/openapi/openapi.json">openapi.json</a>. IDs and names in samples are placeholders.</p>
      </section>

      {#each gqlGroups as g}
        <h3 class="op-group-title sub">{g.title}</h3>
        {#each g.ops as op}
          <section class="op" id={op.id}>
            <div class="op-main">
              <div class="op-head">
                <span class="method {chipClass(op.kind)}">{chipOf(op.kind)}</span>
                <h3 class="op-title">{op.name}</h3>
                {#if op.deprecated}<span class="auth-chip">Deprecated</span>{/if}
              </div>
              <div class="op-path">{signature(op)}</div>
              <div class="op-desc"><Prose paras={op.description} /></div>
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
                        {#if a.default}<span class="param-default">default {a.default}</span>{/if}
                      </div>
                      {#if a.description.length}<div class="param-desc"><Prose paras={a.description} /></div>{/if}
                    </div>
                  {/each}
                </div>
              {/if}
              <div class="param-group">
                <div class="param-group-title">Returns</div>
                <div class="param"><span class="param-type big"><TypeRef type={op.returns} {known} /></span></div>
              </div>
            </div>
            <div class="op-aside">
              <div class="code-panel">
                {#if op.kind === 'subscription'}
                  <Code label="Operation" meta="GraphQL" code={op.document} />
                  <Code label="Frames" meta="→ sent, ← received" code={frames(op)} />
                {:else}
                  <Code tabs={op.samples} />
                  <Code label="Response" meta="application/json" code={op.response} />
                {/if}
              </div>
            </div>
          </section>
        {/each}
      {/each}

      <h3 class="op-group-title sub">Types</h3>
      {#each data.gql.types as t}
        <section class="type-block" id={t.id}>
          <div class="op-head">
            <span class="method m-get">{t.kind.toUpperCase()}</span>
            <h3 class="op-title">{t.name}</h3>
          </div>
          <div class="op-desc"><Prose paras={t.description} /></div>
          <div class="param-group">
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
        </section>
      {/each}

    </div>
  </div>
</div>
