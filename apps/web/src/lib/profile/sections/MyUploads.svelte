<script lang="ts">
  // Your uploads on your own profile: every state (pending, rejected and taken-down ones carry a
  // note in the tile's corner), each tile opening its edit page. Others see the public Uploads.
  import { appendUnique } from '$lib/tool/paging';
  import { onMount } from 'svelte';
  import { gql } from '$lib/api';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import type { CardEmote } from '$lib/components/EmoteCard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { count } from '$lib/format';
  import type { MyEmotesQuery } from '$lib/gql/graphql';
  import { MY_EMOTES } from '$lib/queries';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { STATE_LABEL } from '$lib/tool/state';

  type Item = MyEmotesQuery['myEmotes']['items'][number];
  const PER_PAGE = 60;

  let items = $state<Item[]>([]);
  let total = $state(0);
  let next = $state(1);
  let first = $state(true);
  let busy = $state(false);
  let problem = $state('');

  async function more() {
    if (busy) return;
    busy = true;
    problem = '';
    try {
      const r = await gql(MY_EMOTES, { page: next, perPage: PER_PAGE });
      items = appendUnique(items, r.myEmotes.items);
      total = r.myEmotes.total;
      next += 1;
    } catch (e) {
      problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      busy = false;
      first = false;
    }
  }
  onMount(more);

  // Only what needs attention gets a note; approved emotes are the normal case.
  const note = (e: CardEmote) => {
    const s = (e as Item).state;
    return s && s !== 'APPROVED' ? STATE_LABEL[s].toLowerCase() : undefined;
  };
</script>

<PageHead title="Uploaded emotes" line={first ? ' ' : `${count(total)} ${total === 1 ? 'emote' : 'emotes'}. Pending ones only show to you until a moderator looks at them.`}>
  {#snippet actions()}<a class="btn btn-s" href="/me/upload"><Icon name="upload" size={15} />Upload</a>{/snippet}
</PageHead>

{#if first}
  <EmoteGrid items={[]} loading skeletons={12} creator={false} />
{:else if problem && !items.length}
  <Problem cause="server" onretry={more} />
{:else if !items.length}
  <State title="No emotes yet">
    Upload one and it shows up here, ready to add to a set.
    {#snippet actions()}<a class="btn btn-s btn-primary" href="/me/upload"><Icon name="upload" size={15} />Upload an emote</a>{/snippet}
  </State>
{:else}
  <EmoteGrid {items} creator={false} eager={12} label="Your emotes" href={(e) => `/me/emotes/${e.id}`} {note} loading={busy} skeletons={6} />
  <div class="foot"><LoadMore hasMore={items.length < total} {busy} error={problem} {total} onmore={more} /></div>
{/if}

<style>
  .foot { padding: 24px 0 0; }
</style>
