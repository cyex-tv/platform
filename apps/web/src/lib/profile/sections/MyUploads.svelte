<script lang="ts">
  // Your uploads on your own profile: every state (pending, rejected and taken-down ones carry a
  // note in the tile's corner), each tile opening its edit page. Others see the public Uploads.
  // An editor with Manage on the account (ADR 0014) gets the same list of the owner's emotes
  // (`accountId`, through accountEmotes), and Upload links to "Upload as" them when they may.
  import { appendUnique } from '$lib/tool/paging';
  import { onMount } from 'svelte';
  import { gql } from '$lib/api';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import type { CardEmote } from '$lib/components/EmoteTile.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count, iso } from '$lib/format';
  import type { MyEmotesQuery } from '$lib/gql/graphql';
  import { ACCOUNT_EMOTES, MY_EMOTES } from '$lib/queries';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { STATE_LABEL } from '$lib/tool/state';

  type Item = MyEmotesQuery['myEmotes']['items'][number];
  const PER_PAGE = 60;

  type Props = {
    /** Someone else's account you manage emotes on; yours when left out. */
    accountId?: string;
    name?: string;
    /** You may upload for that account. */
    canUpload?: boolean;
  };
  let { accountId, name = '', canUpload = true }: Props = $props();
  const uploadHref = $derived(accountId ? `/me/upload?for=${encodeURIComponent(accountId)}` : '/me/upload');

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
      const r = accountId
        ? (await gql(ACCOUNT_EMOTES, { accountId, page: next, perPage: PER_PAGE })).accountEmotes
        : (await gql(MY_EMOTES, { page: next, perPage: PER_PAGE })).myEmotes;
      items = appendUnique(items, r.items);
      total = r.total;
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

<PageHead title="Uploaded emotes" line={first ? ' ' : `${count(total)} ${total === 1 ? 'emote' : 'emotes'}. ${accountId ? `You manage ${iso(clean(name))}’s emotes; pending ones show to them, their emote managers and moderators.` : 'Pending ones only show to you until a moderator looks at them.'}`}>
  {#snippet actions()}{#if canUpload}<a class="btn btn-s" href={uploadHref}><Icon name="upload" size={15} />Upload</a>{/if}{/snippet}
</PageHead>

{#if first}
  <EmoteGrid items={[]} loading skeletons={12} creator={false} />
{:else if problem && !items.length}
  <Problem cause="server" onretry={more} />
{:else if !items.length}
  <State title="No emotes yet">
    {accountId ? 'Emotes uploaded for this account show up here.' : 'Upload one and it shows up here, ready to add to a set.'}
    {#snippet actions()}{#if canUpload}<a class="btn btn-s btn-primary" href={uploadHref}><Icon name="upload" size={15} />Upload an emote</a>{/if}{/snippet}
  </State>
{:else}
  <EmoteGrid {items} creator={false} eager={12} label={accountId ? `${clean(name)}’s emotes` : 'Your emotes'} href={(e) => `/me/emotes/${e.id}`} {note} loading={busy} skeletons={6} />
  <div class="foot"><LoadMore hasMore={items.length < total} {busy} error={problem} {total} onmore={more} /></div>
{/if}

<style>
  .foot { padding: 24px 0 0; }
</style>
