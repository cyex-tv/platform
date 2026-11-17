<script lang="ts">
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Mark from '$lib/components/Mark.svelte';

  let { data } = $props();
  const global = $derived(data.global.map((m) => ({ ...m.emote, shownName: m.name })));
</script>

<svelte:head>
  <title>cyex.tv · Emotes for Twitch and Kick chat</title>
  <meta name="description" content="Upload emotes, put them in your channel's set, and see them in chat on Twitch and Kick." />
</svelte:head>

<section class="hero">
  <div class="page">
    <h1>Emotes for Twitch and Kick chat</h1>
    <p class="lead center">Upload an emote, put it in your channel's set, and your chat sees it in cyex deck and other chat apps.</p>
    <div class="actions">
      <a class="btn btn-primary" href="/search">
        Find emotes
        <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
      </a>
      <a class="btn" href="/signin">Sign in</a>
    </div>
  </div>
</section>

{#if global.length > 0}
  <section class="page">
    <EmoteGrid items={global} />
    <p class="fine caption">The global set: shown in every channel that uses cyex.tv.</p>
  </section>
{/if}

<section class="section page">
  <header class="head">
    <p class="kicker"><Mark size={18} lit="top" /> For streamers</p>
    <h2>One set per channel, kept by your team.</h2>
    <p class="lead">Sign in with Twitch or Kick, link the other, and pick the set your channel shows. Editors can change it with you.</p>
  </header>
  <div class="steps">
    <div>
      <h3>Upload</h3>
      <p>PNG, GIF, WebP and animated ones, up to 5 MB. You can use it in your own sets right away.</p>
    </div>
    <div>
      <h3>Reviewed</h3>
      <p>Everyone else sees an emote once a moderator approved it. Rejected ones say why.</p>
    </div>
    <div>
      <h3>Live</h3>
      <p>Add or rename one and chats showing your set get it within a second.</p>
    </div>
  </div>
</section>

<section class="section page">
  <header class="head">
    <p class="kicker"><Mark size={18} lit="middle" /> For chat</p>
    <h2>Personal emotes that follow you.</h2>
    <p class="lead">A small set of your own, shown on your messages in channels that allow them.</p>
  </header>
</section>

<section class="section page">
  <div class="band">
    <div>
      <p class="kicker"><Mark size={18} lit="bottom" /> For developers</p>
      <h2>A public API, no key needed.</h2>
    </div>
    <a class="btn" href="/developers">
      Read the API
      <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
    </a>
  </div>
</section>

<style>
  .hero { padding: calc(var(--nav-h) + clamp(40px, 6vw, 96px)) 0 clamp(48px, 6vw, 72px); text-align: center;
    background: radial-gradient(60% 45% at 50% 0%, color-mix(in srgb, var(--accent-glow) 60%, transparent), transparent 70%); }
  .hero h1 { font-size: var(--t-h1); line-height: .98; letter-spacing: -.045em; max-width: 14ch; margin-inline: auto; }
  .center { margin-inline: auto; }
  .hero .actions { justify-content: center; margin-top: 36px; }
  .caption { margin: 14px 0 0; text-align: center; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
</style>
