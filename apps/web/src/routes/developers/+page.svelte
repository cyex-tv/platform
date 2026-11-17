<script lang="ts">
  import { apiUrl } from '$lib/api';
  const api = apiUrl();
</script>

<svelte:head>
  <title>Developers - cyex.tv</title>
  <meta name="description" content="The cyex.tv API: public reads for chat clients, GraphQL, and live updates." />
</svelte:head>

<div class="page">
  <div class="prose">
    <header class="page-head">
      <h1>Developers</h1>
      <p class="lead">Show cyex.tv emotes in your chat client. Reads are public; no key needed.</p>
    </header>

    <h2>Read a channel's emotes</h2>
    <p>Ask for the channel by its platform user ID, then fetch its active set. Both answer with an <code>ETag</code>; send it back as <code>If-None-Match</code> and you get a cheap <code>304</code> while nothing changed.</p>
    <pre><code>GET {api}/v1/channels/twitch/&lt;user id&gt;
GET {api}/v1/sets/&lt;active_set_id&gt;
GET {api}/v1/sets/global</code></pre>
    <p>An emote's pictures are at <code>&lt;host&gt;/&lt;1-4&gt;x.webp</code> (and <code>.avif</code> for still ones); animated ones also have <code>&lt;n&gt;x_static.webp</code>.</p>

    <h2>Personal emotes</h2>
    <p>Look up the chatters you see, up to 100 at once. Only users with a personal set come back; use them on that user's messages only, in channels where <code>allow_personal_emotes</code> is on.</p>
    <pre><code>POST {api}/v1/personal-sets/lookup
&#123; "platform": "twitch", "user_ids": ["123", "456"] &#125;</code></pre>

    <h2>Live updates</h2>
    <p>One WebSocket carries many subscriptions: <code>wss://api.cyex.tv/v1/events</code>. Subscribe to <code>channel:twitch:&lt;id&gt;</code>, <code>set:&lt;id&gt;</code> or <code>user:twitch:&lt;id&gt;</code>, keep the heartbeat, and resume with your session after a reconnect.</p>

    <h2>Everything else</h2>
    <p>GraphQL at <code>{api}/v1/gql</code> (open it in a browser to explore). The REST description is <a class="link" href="https://github.com/cyex-tv/platform/blob/main/openapi/openapi.json">openapi.json</a>, the GraphQL schema is <a class="link" href="https://github.com/cyex-tv/platform/blob/main/schema/schema.graphql">schema.graphql</a>.</p>
  </div>
</div>

<style>
  .prose { max-width: var(--measure); }
  h2 { font-size: var(--t-h3); line-height: 1.3; letter-spacing: -.015em; margin-top: 40px; }
  p { color: var(--muted); }
  code { font-size: .92em; }
  pre { margin: 16px 0; padding: 16px 18px; border: 1px solid var(--border); border-radius: var(--radius-m);
    background: var(--surface); overflow-x: auto; font-size: 14px; line-height: 1.6; }
</style>
