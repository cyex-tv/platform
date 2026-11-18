<script lang="ts">
  // The Users tab of /search: the same page as the emotes (title, count and tools in one toolbar,
  // the search field in the rail, or above the results on phones), with people as profile rows.
  // The address holds tab=users, q and n (pages loaded); the page's load asks the API, this only
  // adds further pages. Nothing else applies to people, so there are no filters.
  import { goto, invalidateAll, replaceState } from '$app/navigation';
  import { navigating, page } from '$app/state';
  import { untrack } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { looks } from '$lib/cosmetics/store.svelte';
  import { clean, count, iso } from '$lib/format';
  import { SEARCH_ACCOUNTS } from '$lib/queries';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { change, MAX_PAGES, toSearch, USERS_PER_PAGE, type Browse } from './params';
  import SearchField from './SearchField.svelte';
  import SearchTabs from './SearchTabs.svelte';
  import UserRows from './UserRows.svelte';
  import type { FindPeopleQuery } from '$lib/gql/graphql';

  type People = FindPeopleQuery['searchAccounts']['items'];
  type Props = { data: { view: Browse; total: number; people: People; pages: number; error: string | null } };
  let { data }: Props = $props();

  const view = $derived(data.view);
  const q = $derived(clean(view.q));

  // ---- Results: the pages the server rendered, plus the ones loaded here.
  let loaded = $state.raw<People>(untrack(() => data.people));
  let pageNo = $state(untrack(() => data.pages));
  let busy = $state(false);
  let moreError = $state('');
  $effect.pre(() => {
    loaded = data.people;
    pageNo = data.pages;
    moreError = '';
  });
  const hasMore = $derived(pageNo < Math.ceil(data.total / USERS_PER_PAGE) && pageNo < MAX_PAGES);
  const switching = $derived(!!navigating.to);

  // ---- Search field: the address follows the typing (replaced, not pushed, after a pause).
  let text = $state(untrack(() => data.view.q));
  let input = $state<HTMLInputElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const v = view.q;
    if (document.activeElement !== input) text = v;
  });
  function go(next: string) {
    return goto(`/search${toSearch(change(view, { q: next }))}`, { replaceState: true, keepFocus: true, noScroll: true });
  }
  function typed() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      if (text.trim() !== view.q) go(text.trim());
    }, 280);
  }
  function submit(e: SubmitEvent) {
    e.preventDefault();
    clearTimeout(timer);
    if (text.trim() !== view.q) go(text.trim());
  }
  function clearText() {
    text = '';
    clearTimeout(timer);
    go('');
    input?.focus();
  }

  // ---- Phones: the field sits above the results, not in a rail.
  let narrow = $state(false);
  $effect(() => {
    const mq = matchMedia('(max-width: 899.98px)');
    const on = () => (narrow = mq.matches);
    on();
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });

  // ---- Load more: the button, and automatically when the end comes near.
  async function loadMore(asked = true) {
    if (busy || !hasMore) return;
    const from = data;
    busy = true;
    moreError = '';
    try {
      const r = await gql(SEARCH_ACCOUNTS, { query: view.q, page: pageNo + 1, perPage: USERS_PER_PAGE }, fetch);
      if (from !== data) return; // the view changed while this loaded
      const seen = new Set(loaded.map((h) => h.account.id));
      loaded = [...loaded, ...r.searchAccounts.items.filter((h) => !seen.has(h.account.id))];
      pageNo += 1;
      if (asked || scrollY > 0) replaceState(`/search${toSearch({ ...view, pages: pageNo })}`, page.state);
    } catch (e) {
      moreError = e instanceof ApiError ? e.message : 'Couldn’t load more people.';
    } finally {
      busy = false;
    }
  }
  let end = $state<HTMLElement>();
  $effect(() => {
    if (!end || !hasMore || moreError) return;
    void pageNo;
    const io = new IntersectionObserver(
      (hits) => {
        if (hits.some((h) => h.isIntersecting)) void loadMore(false);
      },
      { rootMargin: '0px 0px 800px 0px' },
    );
    io.observe(end);
    return () => io.disconnect();
  });
  const moreHref = $derived(`/search${toSearch({ ...view, pages: Math.min(pageNo + 1, MAX_PAGES) })}`);
  const countText = $derived(`${count(data.total)} ${data.total === 1 ? 'person' : 'people'}`);
</script>

<svelte:head>
  <title>{q ? `${[...q].length > 40 ? [...q].slice(0, 40).join('') + '…' : q} - ` : ''}Users - cyex.tv</title>
  <meta name="description" content="Find people on cyex.tv by the name they sign in with on Twitch or Kick." />
</svelte:head>

{#snippet field()}
  <SearchField bind:value={text} bind:input label="Name or login" placeholder="Name or login" oninput={typed} onsubmit={submit} onclear={clearText} />
{/snippet}

<RailLayout>
  {#snippet rail()}
    {#if !narrow}
      <SideRail label="Search people">
        <p class="rail-title">Search</p>
        {@render field()}
      </SideRail>
    {/if}
  {/snippet}

  <div class="browse">
    <div class="toolbar">
      <div class="title">
        <h1>Users</h1>
        {#if !data.error && view.q}<p class="total" aria-live="polite">{countText}</p>{/if}
      </div>
      <div class="tools"><SearchTabs {view} /></div>
    </div>

    {#if narrow}<div class="field-top">{@render field()}</div>{/if}

    {#if data.error}
      <div class="state">
        <Problem cause="server" onretry={() => invalidateAll()} />
      </div>
    {:else if !view.q}
      <div class="state">
        <State title="Find someone.">Type a name, or the login they use on Twitch or Kick.</State>
      </div>
    {:else if switching && loaded.length === 0}
      <div class="state"><State kind="loading" title="Searching…" /></div>
    {:else if loaded.length === 0}
      <div class="state">
        <State title={`No one matches “${iso(q)}”.`}>
          Check the spelling, or try the login. Only people who have signed in to cyex.tv are listed.
          {#snippet actions()}
            <button class="ui-btn secondary" type="button" onclick={clearText}>Clear search</button>
          {/snippet}
        </State>
      </div>
    {:else}
      <div class="results" class:dim={switching}>
        <UserRows items={loaded} {looks} />
      </div>
      <div class="more" bind:this={end}>
        <LoadMore {hasMore} {busy} error={moreError} total={data.total} onmore={loadMore} href={moreHref} />
      </div>
    {/if}
  </div>
</RailLayout>

<style>
  .rail-title { margin: 0 0 10px; padding-left: 8px; font-size: var(--t-ui); font-weight: 600; letter-spacing: -.01em; }
  .toolbar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 16px; min-height: 40px; }
  @media (max-width: 899.98px) { .toolbar { padding-top: 4px; } }
  .title { display: flex; align-items: baseline; gap: 10px; min-width: 0; }
  h1 { margin: 0; font-size: var(--t-page); line-height: 1.25; letter-spacing: -.02em; }
  .total { margin: 0; font-size: var(--t-nav); color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .tools { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; min-width: 0; max-width: 100%; }
  .field-top { margin-top: 12px; }
  .results { margin-top: 16px; transition: opacity var(--dur-color) ease-out; }
  .results.dim { opacity: .55; transition-delay: .15s; }
  .more { padding: 32px 0 8px; min-height: 96px; }
  .state { margin-top: 16px; }
  @media (max-width: 480px) { .title { flex-direction: column; align-items: flex-start; gap: 0; } }
  @media (prefers-reduced-motion: reduce) { .results { transition: none; } }
</style>
