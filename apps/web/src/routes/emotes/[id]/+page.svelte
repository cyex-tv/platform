<script lang="ts">
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import EmoteView from '$lib/components/emote/EmoteView.svelte';
  import { channelsUsing, globalPool } from '$lib/components/emote/data';
  import { backTarget, channelCount, type ChatPool, type EmoteChannel, type PageEmote } from '$lib/components/emote/emote-page';
  import { src, ready } from '$lib/emote';
  import { clean } from '$lib/format';
  import { EMOTE } from '$lib/queries';

  let { data } = $props();

  // The server can't see pending, rejected or taken-down emotes (no session cookie), so when it
  // found nothing the browser asks once more: their owner and moderators get the page.
  let late = $state<{ id: string; emote: PageEmote | null; pool: ChatPool; channels: EmoteChannel[] | null } | null>(null);
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
      const [pool, channels] = emote ? await Promise.all([globalPool(), channelsUsing(emote)]) : [[], null];
      if (!gone) late = { id, emote, pool, channels };
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

  const name = $derived(e ? clean(e.name) : '');
  const description = $derived(
    e ? `${name}, ${e.animated ? 'an animated' : 'an'} emote for Twitch and Kick chat on cyex.tv${e.owner ? `, by ${clean(e.owner.displayName)}` : ''}.` : '',
  );
</script>

<svelte:head>
  {#if e}
    <title>{name} - cyex.tv</title>
    <meta name="description" content={description} />
    <meta property="og:type" content="website" />
    <meta property="og:site_name" content="cyex.tv" />
    <meta property="og:title" content="{name} - cyex.tv" />
    <meta property="og:description" content={description} />
    <meta property="og:url" content={page.url.origin + page.url.pathname} />
    {#if ready(e)}
      <meta property="og:image" content={src(e, 4, true)} />
      <meta property="og:image:alt" content={name} />
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
    <EmoteView emote={e} {pool} more={data.more} channels={data.emote ? data.channels : (asked?.channels ?? null)} channelCount={channelCount(e)} {back} />
  {/key}
{:else if asked === undefined}
  <!-- Asking from the browser: hold the space quietly, then show the page or say it's gone. -->
  <ColumnLayout top>
    <div class="missing">
      <span class="back ghost" aria-hidden="true">All emotes</span>
      <State kind="loading" title="Looking for this emote…" />
    </div>
  </ColumnLayout>
{:else}
  <ColumnLayout top>
    <div class="missing">
      <a class="back" href="/search"><Icon name="arrow-left" />All emotes</a>
      <State level={1} role="status" title="No emote here.">
        It may have been taken down, its creator may have deleted it, or the link is missing a character. Pending emotes are
        only visible to their creator.
        {#snippet actions()}
          <a class="btn btn-s btn-primary" href="/search">Browse emotes</a>
          <a class="btn btn-s" href="/">Front page</a>
        {/snippet}
      </State>
    </div>
  </ColumnLayout>
{/if}

<style>
  .missing { display: grid; gap: 16px; min-height: 50svh; align-content: start; padding-bottom: 40px; }
  .back { justify-self: start; display: inline-flex; align-items: center; gap: 8px; min-height: 40px; color: var(--muted); font-size: var(--t-small); }
  .ghost { visibility: hidden; }
  @media (hover: hover) and (pointer: fine) { .back:hover { color: var(--text); } }
</style>
