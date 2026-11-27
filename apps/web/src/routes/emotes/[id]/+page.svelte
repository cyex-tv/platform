<script lang="ts">
  import { afterNavigate, goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import EmoteView from '$lib/components/emote/EmoteView.svelte';
  import { channelsUsing } from '$lib/components/emote/data';
  import { backTarget, channelCount, type EmoteChannel, type PageEmote } from '$lib/components/emote/emote-page';
  import { mergedPath } from '$lib/components/emote/merge';
  import { clean } from '$lib/format';
  import { legacyRedirect } from '$lib/ids';
  import { EMOTE } from '$lib/queries';

  let { data } = $props();

  // The server can't see pending, rejected or taken-down emotes (no session cookie), so when it
  // found nothing the browser asks once more: their owner and moderators get the page.
  let late = $state<{ id: string; emote: PageEmote | null; channels: EmoteChannel[] | null } | null>(null);
  $effect(() => {
    if (data.emote) return;
    const id = data.id;
    let gone = false;
    (async () => {
      let emote: (PageEmote & { mergedInto?: string | null }) | null = null;
      try {
        emote = (await gql(EMOTE, { id })).emote ?? null;
      } catch {
        emote = null;
      }
      // A merged emote (ADR 0040), seen only now: its page is the target's.
      if (emote?.mergedInto) {
        if (!gone) void goto(mergedPath(emote.mergedInto, page.url.search), { replaceState: true });
        return;
      }
      const channels = emote ? await channelsUsing(emote) : null;
      if (gone) return;
      late = { id, emote, channels };
      // Found only now (the server could not see it): an old ULID link still ends on the snowflake address.
      const moved = legacyRedirect('emotes', id, emote?.id, page.url.search);
      if (moved) void goto(moved, { replaceState: true });
    })();
    return () => (gone = true);
  });

  const asked = $derived(data.emote ? null : late?.id === data.id ? late : undefined);
  const e = $derived(data.emote ?? asked?.emote ?? null);

  let from = $state<URL | null>(null);
  // Switching tabs (`?tab=`) is a navigation on this same page: it keeps where the reader came
  // from, but the back link then goes there by its address, since one step back is the last tab.
  let tabbed = $state(false);
  afterNavigate((nav) => {
    if (nav.type === 'enter') return;
    if (nav.from && nav.to && nav.from.url.pathname === nav.to.url.pathname) {
      tabbed = true;
      return;
    }
    from = nav.from?.url ?? null;
    tabbed = false;
  });
  const back = $derived.by(() => {
    const b = backTarget(from, page.url.pathname);
    return tabbed ? { ...b, back: false } : b;
  });

  const name = $derived(e ? clean(e.name) : '');
</script>

<svelte:head>
  {#if e}
    <title>{name} - cyex.tv</title>
  {:else}
    <title>No emote here - cyex.tv</title>
  {/if}
</svelte:head>

{#if e}
  {#key e.id}
    <EmoteView emote={e} more={data.more} channels={data.emote ? data.channels : (asked?.channels ?? null)} channelCount={channelCount(e)} trending={data.emote ? data.trending : null} {back} />
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
