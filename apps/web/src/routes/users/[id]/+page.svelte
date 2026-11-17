<script lang="ts">
  import { ApiError, gql } from '$lib/api';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import ProfileHead from '$lib/components/profile/ProfileHead.svelte';
  import ProfileState from '$lib/components/profile/ProfileState.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import { count } from '$lib/format';
  import { SEARCH_BY_OWNER } from '$lib/queries';

  let { data } = $props();
  const PER_PAGE = 60;

  type Sort = 'top' | 'new';
  let sort = $state<Sort>('top');
  // The first page of Top comes from the load; New, and every page after, from the browser.
  let fresh = $state<typeof data.items | null>(null);
  let extra = $state<typeof data.items>([]);
  let nextPage = $state(2);
  let loading = $state(false);
  let failed = $state<string | null>(null);

  // A new creator (client-side navigation) starts over.
  $effect(() => {
    void data.id;
    fresh = null;
    extra = [];
    nextPage = 2;
    failed = null;
    sort = 'top';
  });

  const items = $derived([...(sort === 'new' ? (fresh ?? []) : data.items), ...extra]);
  const fetched = $derived(PER_PAGE * (nextPage - 1));
  const more = $derived(data.total > fetched);
  const name = $derived(data.creator?.displayName ?? '');

  const ask = (page: number, s: Sort) =>
    gql(SEARCH_BY_OWNER, { owner: data.id, page, perPage: PER_PAGE, sort: s === 'new' ? 'NEW' : 'TOP' }).then(
      (r) => r.searchEmotes.items,
    );

  async function loadMore() {
    if (loading) return;
    const s = sort;
    loading = true;
    failed = null;
    try {
      const got = await ask(nextPage, s);
      if (s !== sort) return; // the order changed while this loaded
      extra = [...extra, ...got];
      nextPage += 1;
    } catch (e) {
      failed = e instanceof ApiError ? e.message : 'That didn’t load. Try again.';
    } finally {
      loading = false;
    }
  }

  // Changing the order asks again from the first page.
  async function reorder(s: Sort) {
    sort = s;
    extra = [];
    nextPage = 2;
    failed = null;
    if (s === 'top') return;
    loading = true;
    try {
      const got = await ask(1, s);
      if (s === sort) fresh = got;
    } catch (e) {
      failed = e instanceof ApiError ? e.message : 'That didn’t load. Try again.';
      if (s === sort) sort = 'top';
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>{name ? `${name} - cyex.tv` : 'Creator - cyex.tv'}</title>
  {#if name}<meta name="description" content="{name}'s emotes on cyex.tv." />{/if}
</svelte:head>

<div class="page">
  {#if data.error}
    <div class="pad">
      <ProfileState title="This page can’t load right now." role="alert">
        {data.error}
        {#snippet actions()}
          <button class="btn btn-primary" type="button" onclick={() => location.reload()}>Try again</button>
        {/snippet}
      </ProfileState>
    </div>
  {:else if !data.creator}
    <div class="pad">
      <ProfileState title="No public emotes here." role="status">
        This account hasn't published an emote, or there is no account with this address. Check the link, or
        look through what everyone has made.
        {#snippet actions()}
          <a class="btn btn-primary" href="/search">Browse emotes</a>
        {/snippet}
      </ProfileState>
    </div>
  {:else}
    <ProfileHead {name} avatar={data.creator.avatar} kind="Creator">
      {#snippet meta()}
        {count(data.total)} {data.total === 1 ? 'emote' : 'emotes'} uploaded
      {/snippet}
      {#snippet actions()}
        {#if data.channels.length}
          <div class="chans">
            {#each data.channels as ch (ch.platform + ch.login)}
              <a class="btn btn-s" href="/channels/{ch.platform}/{ch.login}">{ch.platform === 'twitch' ? 'Twitch' : 'Kick'}: {ch.login}</a>
            {/each}
          </div>
        {/if}
      {/snippet}
    </ProfileHead>

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
        <ProfileState title="No public emotes yet.">
          {name} hasn't published an emote that is ready to show.
        </ProfileState>
      {:else}
        <EmoteGrid {items} creator={false} eager={12} {loading} skeletons={6} label="Emotes by {name}" />
        <div class="more">
          {#if failed}<p class="fail" role="alert">{failed}</p>{/if}
          {#if more}
            <button class="btn" type="button" onclick={loadMore} disabled={loading} aria-busy={loading}>
              {loading ? 'Loading…' : failed ? 'Try again' : `Load more (${count(data.total - fetched)} left)`}
            </button>
          {:else}
            <p class="fine">That is all {count(data.total)}.</p>
          {/if}
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .pad { padding-top: clamp(48px, 8vw, 120px); }
  .bar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 24px; margin-bottom: 20px; }
  .chans { display: flex; flex-wrap: wrap; gap: 8px; }
  .more { display: grid; justify-items: center; gap: 12px; padding: 24px 0 0; }
  .more p { margin: 0; }
  .fail { color: var(--muted); font-size: var(--t-small); }
</style>
