<script lang="ts">
  // A creator's uploads: Top from the load, New and every page after from the browser.
  // The page keys this on the creator, so a new creator starts from scratch (no reset effect).
  import { ApiError, gql } from '$lib/api';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import State from '$lib/components/State.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import type { CardEmote } from '$lib/components/EmoteCard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { count } from '$lib/format';
  import { SEARCH_BY_OWNER } from '$lib/queries';

  type Sort = 'top' | 'new';
  let { id, name, first, total }: { id: string; name: string; first: CardEmote[]; total: number } = $props();
  const PER_PAGE = 60;

  let sort = $state<Sort>('top');
  let fresh = $state<CardEmote[] | null>(null);
  let extra = $state<CardEmote[]>([]);
  let nextPage = $state(2);
  let loading = $state(false);
  let failed = $state('');

  const items = $derived([...(sort === 'new' ? (fresh ?? []) : first), ...extra]);
  const fetched = $derived(PER_PAGE * (nextPage - 1));
  const more = $derived(total > fetched);

  // The in-page filter looks through what is loaded so far.
  let filter = $state('');
  const needle = $derived(filter.trim().toLowerCase());
  const shown = $derived(needle ? items.filter((e) => e.name.toLowerCase().includes(needle)) : items);

  const ask = (page: number, s: Sort) =>
    gql(SEARCH_BY_OWNER, { owner: id, page, perPage: PER_PAGE, sort: s === 'new' ? 'NEW' : 'TOP' }).then(
      (r) => r.searchEmotes.items,
    );
  const message = (e: unknown) => (e instanceof ApiError ? e.message : 'That didn’t load. Try again.');

  async function loadMore() {
    if (loading) return;
    const s = sort;
    loading = true;
    failed = '';
    try {
      const got = await ask(nextPage, s);
      if (s !== sort) return; // the order changed while this loaded
      extra = [...extra, ...got];
      nextPage += 1;
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
    nextPage = 2;
    failed = '';
    if (s === 'top') return;
    loading = true;
    try {
      const got = await ask(1, s);
      if (s === sort) fresh = got;
    } catch (e) {
      failed = message(e);
      if (s === sort) sort = 'top';
    } finally {
      loading = false;
    }
  }
</script>

<section aria-labelledby="up-h">
  <div class="bar">
    <h1 class="page-title" id="up-h">Uploaded emotes <span class="n">{count(total)}</span></h1>
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
        <button class="ui-btn secondary" type="button" onclick={() => (filter = '')}>Clear filter</button>
      {/snippet}
    </State>
    {#if more}<div class="foot"><LoadMore hasMore={more} busy={loading} error={failed} {total} onmore={loadMore} /></div>{/if}
  {:else}
    <EmoteGrid items={shown} creator={false} eager={12} {loading} skeletons={6} label="Emotes by {name}" />
    <div class="foot"><LoadMore hasMore={more} busy={loading} error={failed} {total} onmore={loadMore} /></div>
  {/if}
</section>

<style>
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; margin-bottom: 20px; }
  .bar :global([role='radiogroup']) { margin-left: auto; }
  .n { margin-left: 6px; font-size: var(--t-small); font-weight: 500; color: var(--dim); font-variant-numeric: tabular-nums; }
  .find { position: relative; display: block; flex: 0 1 280px; min-width: 0; }
  .find :global(svg) { position: absolute; left: 12px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 40px; padding-left: 36px; }
  @media (max-width: 520px) { .find { flex-basis: 100%; order: 3; } }
  .foot { padding: 24px 0 0; }
</style>
