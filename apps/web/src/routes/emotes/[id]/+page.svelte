<script lang="ts">
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import EmoteView from '$lib/components/emote/EmoteView.svelte';
  import { globalPool } from '$lib/components/emote/data';
  import { backTarget, type ChatPool, type PageEmote } from '$lib/components/emote/emote-page';
  import { src, ready } from '$lib/emote';
  import { EMOTE } from '$lib/queries';

  let { data } = $props();

  // The server can't see pending, rejected or taken-down emotes (no session cookie), so when it
  // found nothing the browser asks once more: their owner and moderators get the page.
  let late = $state<{ id: string; emote: PageEmote | null; pool: ChatPool } | null>(null);
  $effect(() => {
    if (data.emote) return;
    const id = data.id;
    let gone = false;
    (async () => {
      let emote: PageEmote | null = null;
      try {
        emote = (await gql(EMOTE, { id })).emote ?? null;
      } catch {
        emote = null;
      }
      const pool = emote ? await globalPool() : [];
      if (!gone) late = { id, emote, pool };
    })();
    return () => (gone = true);
  });

  const asked = $derived(data.emote ? null : late?.id === data.id ? late : undefined);
  const e = $derived(data.emote ?? asked?.emote ?? null);
  const pool = $derived(data.emote ? data.pool : (asked?.pool ?? []));

  let from = $state<URL | null>(null);
  afterNavigate((nav) => {
    if (nav.type !== 'enter') from = nav.from?.url ?? null;
  });
  const back = $derived(backTarget(from, page.url.pathname));

  const description = $derived(
    e ? `${e.name}, ${e.animated ? 'an animated' : 'an'} emote for Twitch and Kick chat on cyex.tv${e.owner ? `, by ${e.owner.displayName}` : ''}.` : '',
  );
</script>

<svelte:head>
  {#if e}
    <title>{e.name} - cyex.tv</title>
    <meta name="description" content={description} />
    <meta property="og:type" content="website" />
    <meta property="og:site_name" content="cyex.tv" />
    <meta property="og:title" content="{e.name} - cyex.tv" />
    <meta property="og:description" content={description} />
    <meta property="og:url" content={page.url.origin + page.url.pathname} />
    {#if ready(e)}
      <meta property="og:image" content={src(e, 4, true)} />
      <meta property="og:image:alt" content={e.name} />
      <meta name="twitter:card" content="summary" />
    {/if}
    {#if e.state !== 'APPROVED'}<meta name="robots" content="noindex" />{/if}
  {:else}
    <title>No emote here - cyex.tv</title>
    <meta name="robots" content="noindex" />
  {/if}
</svelte:head>

{#if e}
  {#key e.id}
    <EmoteView emote={e} {pool} more={data.more} channels={data.channels} channelCount={data.channelCount} {back} />
  {/key}
{:else if asked === undefined}
  <!-- Asking from the browser: hold the space quietly, then show the page or say it's gone. -->
  <div class="page missing" aria-busy="true"><p class="sr-only" role="status">Looking for this emote…</p></div>
{:else}
  <div class="page missing">
    <a class="back" href="/search"><Icon name="arrow-left" />All emotes</a>
    <div class="nothing">
      <h1>No emote here.</h1>
      <p class="lead">
        It may have been taken down, its creator may have deleted it, or the link is missing a character. Pending emotes are
        only visible to their creator.
      </p>
      <div class="actions">
        <a class="btn btn-primary" href="/search">
          Browse emotes
          <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
        </a>
        <a class="btn" href="/">Front page</a>
      </div>
    </div>
  </div>
{/if}

<style>
  .missing { padding-top: clamp(16px, 3vw, 32px); padding-bottom: clamp(64px, 8vw, 120px); min-height: 60svh; }
  .back { display: inline-flex; align-items: center; gap: 8px; min-height: 40px; color: var(--muted); font-size: var(--t-small); }
  .nothing { padding-top: clamp(48px, 8vw, 112px); max-width: 640px; }
  h1 { font-size: clamp(44px, 7vw, 96px); line-height: 1; letter-spacing: -.045em; }
  .lead { margin-top: 20px; }
  .actions { margin-top: 32px; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  @media (hover: hover) and (pointer: fine) { .back:hover { color: var(--text); } }
</style>
