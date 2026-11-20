<script lang="ts">
  // Your favourite emotes, on your own profile only (private: nobody else, editors neither, sees
  // this tab). The latest hearted first; the field narrows them by name. An emote you take off
  // your favourites (the heart on its page) leaves the grid at once.
  import { onMount } from 'svelte';
  import { gql } from '$lib/api';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count } from '$lib/format';
  import type { MyFavouritesQuery } from '$lib/gql/graphql';
  import { personal } from '$lib/personal/lists.svelte';
  import { MY_FAVOURITES } from '$lib/queries';
  import { appendUnique, totalAfterPage } from '$lib/tool/paging';
  import PageHead from '$lib/tool/PageHead.svelte';

  type Item = MyFavouritesQuery['myFavourites']['items'][number];
  const PER_PAGE = 60;

  let items = $state<Item[]>([]);
  let total = $state(0);
  let next = $state(1);
  let first = $state(true);
  let busy = $state(false);
  let problem = $state('');
  let query = $state('');
  let asked = $state('');
  let token = 0;

  async function load(reset = false) {
    if (busy && !reset) return;
    const mine = ++token;
    busy = true;
    problem = '';
    const page = reset ? 1 : next;
    try {
      const r = (await gql(MY_FAVOURITES, { query: asked || null, page, perPage: PER_PAGE })).myFavourites;
      if (mine !== token) return;
      items = reset ? r.items : appendUnique(items, r.items);
      total = totalAfterPage(items.length, r.items.length, r.total, reset);
      next = page + 1;
    } catch (e) {
      if (mine !== token) return;
      problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      if (mine === token) {
        busy = false;
        first = false;
      }
    }
  }
  onMount(() => {
    personal.ensure();
    void load(true);
  });

  function find(e: SubmitEvent) {
    e.preventDefault();
    const q = query.trim();
    if (q === asked) return;
    asked = q;
    void load(true);
  }

  // Taken off your favourites somewhere else on the page: gone from the grid too.
  const shown = $derived(personal.loaded ? items.filter((e) => personal.favourites.has(e.id)) : items);
  const dropped = $derived(items.length - shown.length);
  const left = $derived(Math.max(0, total - dropped));
</script>

<PageHead
  title="Favourites"
  line={first ? ' ' : `${count(left)} ${left === 1 ? 'emote' : 'emotes'}${asked ? ` matching “${clean(asked)}”` : ''}. Only you see this list.`}
/>

{#if total > 0 || asked}
  <form class="find" role="search" onsubmit={find}>
    <label class="sr-only" for="fav-find">Find in your favourites</label>
    <span class="ui-input-icon"><Icon name="search" size={15} /></span>
    <input id="fav-find" class="ui-input" type="search" placeholder="Find in your favourites" autocomplete="off" bind:value={query}
      oninput={() => { if (!query.trim() && asked) { asked = ''; void load(true); } }} />
  </form>
{/if}

{#if first}
  <EmoteGrid items={[]} loading skeletons={12} />
{:else if problem && !items.length}
  <Problem cause="server" onretry={() => load(true)} />
{:else if !shown.length}
  {#if asked}
    <State title="No favourites match">
      None of your favourites has “{clean(asked)}” in its name.
      {#snippet actions()}<button class="btn btn-s" type="button" onclick={() => { query = ''; asked = ''; void load(true); }}>Show all favourites</button>{/snippet}
    </State>
  {:else}
    <State title="No favourites yet">
      Press the heart on an emote, or pick Favourite in a tile’s menu. Favourites come first when you search and when you add emotes to a set.
      {#snippet actions()}<a class="btn btn-s btn-primary" href="/search">Browse emotes</a>{/snippet}
    </State>
  {/if}
{:else}
  <EmoteGrid items={shown} eager={12} label="Your favourites" loading={busy} skeletons={6} />
  <div class="foot"><LoadMore hasMore={items.length < total} {busy} error={problem} total={left} onmore={() => load()} /></div>
{/if}

<style>
  .find { position: relative; display: flex; align-items: center; max-width: 360px; margin: 0 0 16px; }
  .find input { width: 100%; padding-left: 34px; }
  .ui-input-icon { position: absolute; left: 11px; display: grid; color: var(--muted); pointer-events: none; }
  .foot { padding: 24px 0 0; }
</style>
