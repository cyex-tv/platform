<script lang="ts">
  // A creator's uploads: Top from the load, New and every page after from the browser.
  // The page keys this on the creator, so a new creator starts from scratch (no reset effect).
  import { appendUnique } from '$lib/tool/paging';
  import { ApiError, gql } from '$lib/api';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import State from '$lib/components/State.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import type { CardEmote } from '$lib/components/EmoteTile.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { count } from '$lib/format';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { SEARCH_BY_OWNER } from '$lib/queries';

  type Sort = 'top' | 'new';
  let { id, name, first, total, firstNext = null }: { id: string; name: string; first: CardEmote[]; total: number; firstNext?: string | null } = $props();
  const PER_PAGE = 60;

  let sort = $state<Sort>('top');
  let fresh = $state<CardEmote[] | null>(null);
  let extra = $state<CardEmote[]>([]);
  // Where the next page starts (cursor paging, ADR 0030): from the load for Top, from the
  // first New page after reordering. Null when there is no next page.
  // svelte-ignore state_referenced_locally
  let after = $state<string | null>(firstNext);
  let pages = $state(1);
  let loading = $state(false);
  let failed = $state('');

  const items = $derived([...(sort === 'new' ? (fresh ?? []) : first), ...extra]);
  const fetched = $derived(PER_PAGE * pages);
  const more = $derived(total > fetched && after !== null);

  // The in-page filter looks through what is loaded so far.
  let filter = $state('');
  const needle = $derived(filter.trim().toLowerCase());
  const shown = $derived(needle ? items.filter((e) => e.name.toLowerCase().includes(needle)) : items);

  const ask = (s: Sort, from: string | null) =>
    gql(SEARCH_BY_OWNER, { owner: id, perPage: PER_PAGE, sort: s === 'new' ? 'NEW' : 'TOP', after: from }).then(
      (r) => r.searchEmotes,
    );
  const message = (e: unknown) => (e instanceof ApiError ? e.message : 'That didn’t load. Try again.');

  async function loadMore() {
    if (loading || !after) return;
    const s = sort;
    loading = true;
    failed = '';
    try {
      const got = await ask(s, after);
      if (s !== sort) return; // the order changed while this loaded
      extra = appendUnique(extra, got.items);
      after = got.next ?? null;
      pages += 1;
    } catch (e) {
      failed = message(e);
    } finally {
      loading = false;
    }
  }

  // Changing the order asks again from the first page.
  async function reorder(s: Sort) {
    sort = s;
    extra = [];
    pages = 1;
    failed = '';
    // Top's first page came with the load, and so did where its next page starts.
    after = s === 'top' ? firstNext : null;
    if (s === 'top') return;
    loading = true;
    try {
      const got = await ask(s, null);
      if (s === sort) {
        fresh = got.items;
        after = got.next ?? null;
      }
    } catch (e) {
      failed = message(e);
      if (s === sort) {
        sort = 'top';
        after = firstNext;
      }
    } finally {
      loading = false;
    }
  }
</script>

<section aria-label="Uploaded emotes">
  <PageHead title="Uploaded emotes" line="{count(total)} {total === 1 ? 'emote' : 'emotes'}." />
  <div class="bar">
    <label class="find">
      <span class="sr-only">Filter uploads</span>
      <Icon name="search" size={16} />
      <input class="input" type="search" placeholder="Filter uploads" bind:value={filter} autocomplete="off" />
    </label>
    <Segmented
      label="Sort uploads"
      options={[{ value: 'top', label: 'Top' }, { value: 'new', label: 'New' }]}
      value={sort}
      onchange={reorder}
    />
  </div>
  {#if items.length === 0}
    <State title="No public emotes yet."><bdi>{name}</bdi> hasn't published an emote that is ready to show.</State>
  {:else if shown.length === 0}
    <State title="Nothing matches “{filter.trim()}”.">
      Nothing loaded so far matches{#if more}; load more to look further{/if}.
      {#snippet actions()}
        <button class="btn btn-s" type="button" onclick={() => (filter = '')}>Clear filter</button>
      {/snippet}
    </State>
    {#if more}<div class="foot"><LoadMore hasMore={more} busy={loading} error={failed} {total} onmore={loadMore} /></div>{/if}
  {:else}
    <EmoteGrid sheet items={shown} creator={false} eager={12} {loading} skeletons={6} label="Emotes by {name}" />
    <div class="foot"><LoadMore hasMore={more} busy={loading} error={failed} {total} onmore={loadMore} /></div>
  {/if}
</section>

<style>
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; margin-bottom: 20px; }
  .bar :global([role='radiogroup']) { margin-left: auto; }
  .find { position: relative; display: block; flex: 0 1 280px; min-width: 0; }
  .find :global(svg) { position: absolute; left: 12px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 40px; padding-left: 36px; }
  @media (max-width: 639.98px) { .find { flex-basis: 100%; order: 3; } }
  .foot { padding: 24px 0 0; }
</style>
