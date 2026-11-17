<script lang="ts">
  // A creator's uploads: Top from the load, New and every page after from the browser.
  // The page keys this on the creator, so a new creator starts from scratch (no reset effect).
  import { ApiError, gql } from '$lib/api';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import EmptyState from '$lib/components/profile/EmptyState.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import type { CardEmote } from '$lib/components/EmoteCard.svelte';
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
    <h2 class="block-title" id="up-h">Uploads</h2>
    <Segmented
      label="Sort uploads"
      options={[{ value: 'top', label: 'Top' }, { value: 'new', label: 'New' }]}
      value={sort}
      onchange={reorder}
    />
  </div>
  {#if items.length === 0}
    <EmptyState rule title="No public emotes yet."><bdi>{name}</bdi> hasn't published an emote that is ready to show.</EmptyState>
  {:else}
    <EmoteGrid {items} creator={false} eager={12} {loading} skeletons={6} label="Emotes by {name}" />
    <div class="foot"><LoadMore hasMore={more} busy={loading} error={failed} {total} onmore={loadMore} /></div>
  {/if}
</section>

<style>
  .bar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 24px; margin-bottom: 20px; }
  .foot { padding: 24px 0 0; }
</style>
