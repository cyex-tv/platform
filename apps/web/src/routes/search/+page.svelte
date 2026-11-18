<script lang="ts">
  import { goto, invalidateAll, replaceState } from '$app/navigation';
  import { navigating, page } from '$app/state';
  import { tick, untrack } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import Chip from '$lib/components/Chip.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import BrowseFilters from '$lib/components/browse/BrowseFilters.svelte';
  import BulkAdd from '$lib/components/browse/BulkAdd.svelte';
  import EmoteTable from '$lib/components/browse/EmoteTable.svelte';
  import { looks, loadOwned } from '$lib/cosmetics/store.svelte';
  import { session } from '$lib/tool/session.svelte';
  import State from '$lib/components/State.svelte';
  import {
    apiQuery,
    apiSort,
    CLEARED,
    clientSide,
    MAX_PAGES,
    PER_PAGE,
    active as activeOf,
    change,
    matches,
    parse,
    toSearch,
    topTags,
    type Browse,
    type Layout,
    type Sort,
  } from '$lib/components/browse/params';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { clean, count, iso } from '$lib/format';
  import { SEARCH } from '$lib/queries';
  import SearchField from '$lib/components/browse/SearchField.svelte';
  import SearchTabs from '$lib/components/browse/SearchTabs.svelte';
  import UsersSearch from '$lib/components/browse/UsersSearch.svelte';

  let { data } = $props();

  const view = $derived(data.view);
  const q = $derived(clean(view.q));
  const filters = $derived(activeOf(view));

  // ---- Results: the pages the server rendered, plus the ones loaded here.
  let loaded = $state.raw(untrack(() => data.items));
  let pageNo = $state(untrack(() => data.pages));
  let busy = $state(false);
  let moreError = $state('');
  $effect.pre(() => {
    loaded = data.items;
    pageNo = data.pages;
    moreError = '';
  });

  const totalPages = $derived(Math.ceil(data.total / PER_PAGE));
  const hasMore = $derived(pageNo < totalPages);
  const shown = $derived(loaded.filter((e) => matches(view, e)));
  const clientFiltered = $derived(clientSide(view));
  const tags = $derived(topTags(loaded));
  const switching = $derived(!!navigating.to);

  const sorts = $derived<{ value: Sort; label: string }[]>([
    ...(view.q ? [{ value: 'relevance' as const, label: 'Relevance' }] : []),
    { value: 'top', label: 'Top' },
    { value: 'new', label: 'New' },
  ]);

  // ---- Search field: the URL follows the typing (replaced, not pushed, after a short pause).
  let text = $state(untrack(() => data.view.q));
  let input = $state<HTMLInputElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const q = view.q;
    if (document.activeElement !== input) text = q;
  });

  // The view's local parts (layout, select mode) are in the URL too, set without a reload.
  let layoutPick = $state<Layout>(untrack(() => data.view.layout));
  let selecting = $state(untrack(() => data.view.select));
  // Follows the address (which `sync` and every navigation keep in step with these two).
  $effect.pre(() => {
    const now = parse(page.url.searchParams);
    layoutPick = now.layout;
    selecting = now.select;
  });
  const here = $derived<Browse>({ ...view, layout: layoutPick, select: selecting });

  function go(over: Partial<Browse>, replace = false) {
    const next = change(here, over);
    return goto(`/search${toSearch(next)}`, { replaceState: replace, keepFocus: true, noScroll: true });
  }
  let clearAll = $state<HTMLButtonElement>();
  let chipsEl = $state<HTMLElement>();
  let gridTop = $state<HTMLElement>();
  // Removing a chip would drop focus to the page: hand it to the next chip's remove button,
  // else Clear all, else the toolbar.
  async function removeChip(key: keyof typeof CLEARED, i: number) {
    await go({ [key]: CLEARED[key] } as Partial<Browse>);
    await tick();
    const rm = chipsEl?.querySelectorAll<HTMLElement>('.chips [role=listitem] button');
    const target = rm?.[Math.min(i, rm.length - 1)] ?? clearAll ?? gridTop?.querySelector<HTMLElement>('[aria-checked="true"]');
    target?.focus();
  }
  function typed() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      if (text.trim() !== view.q) go({ q: text.trim() }, true);
    }, 280);
  }
  function submit(e: SubmitEvent) {
    e.preventDefault();
    clearTimeout(timer);
    if (text.trim() !== view.q) go({ q: text.trim() }, true);
  }
  function clearText() {
    text = '';
    clearTimeout(timer);
    go({ q: '' }, true);
    input?.focus();
  }

  // ---- Phones and narrow windows (SideRail's breakpoint): the rail becomes a Filters sheet and
  // the search field moves into its sticky bar, so it is never hidden behind a button.
  let narrow = $state(false);
  $effect(() => {
    const mq = matchMedia('(max-width: 899.98px)');
    const on = () => (narrow = mq.matches);
    on();
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });

  // ---- Density and grid or table, remembered on this device (the URL wins when it says view=).
  type Density = 'comfortable' | 'compact';
  let density = $state<Density>('comfortable');
  let savedLayout = $state<'grid' | 'table'>('grid');
  const layout = $derived<'grid' | 'table'>(layoutPick || savedLayout);
  $effect(() => {
    try {
      const d = localStorage.getItem('cyex.browse.density');
      if (d === 'compact' || d === 'comfortable') density = d;
      const l = localStorage.getItem('cyex.browse.layout');
      if (l === 'grid' || l === 'table') savedLayout = l;
    } catch {
      /* private mode: keep the default */
    }
  });
  function remember(key: string, v: string) {
    try {
      localStorage.setItem(key, v);
    } catch {
      /* not remembered */
    }
  }
  function setDensity(d: Density) {
    density = d;
    remember('cyex.browse.density', d);
    setLayout('grid');
  }
  function setLayout(l: 'grid' | 'table') {
    layoutPick = l;
    savedLayout = l;
    remember('cyex.browse.layout', l);
    sync();
  }
  /** Puts the local view state into the address without loading anything again. */
  function sync() {
    replaceState(`/search${toSearch({ ...here, pages: Math.min(pageNo, MAX_PAGES) })}`, page.state);
  }

  // ---- Multi-select: picked ids (in the order picked), kept while filters change.
  let picked = $state.raw<string[]>([]);
  const pickedSet = $derived(new Set(picked));
  function toggle(e: { id: string }) {
    picked = pickedSet.has(e.id) ? picked.filter((id) => id !== e.id) : [...picked, e.id];
  }
  function setSelecting(on: boolean) {
    selecting = on;
    if (!on) picked = [];
    sync();
  }
  // The painted name of your own emotes comes from the shared cosmetics cache (the API shows
  // other creators' paints by platform id only, so theirs stay plain).
  $effect(() => {
    if (layout === 'table' && session.me) loadOwned(session.me.id, true).catch(() => {});
  });
  const signin = $derived(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`);

  // ---- Load more: button, and automatically when the end comes near. The page count goes into
  // the URL (n=) only for loads the reader asked for: the button, or scrolling down to the end.
  // Loads that only fill a tall window leave the address as it was.
  async function loadMore(asked = true) {
    if (busy || !hasMore) return;
    const from = data;
    busy = true;
    moreError = '';
    try {
      const r = await gql(
        SEARCH,
        {
          query: apiQuery(view),
          animated: view.kind === 'animated' ? true : view.kind === 'still' ? false : null,
          nsfw: view.nsfw ? true : null,
          sort: apiSort(view.sort),
          page: pageNo + 1,
          perPage: PER_PAGE,
        },
        fetch,
      );
      if (from !== data) return; // the view changed while this loaded
      const seen = new Set(loaded.map((e) => e.id));
      loaded = [...loaded, ...r.searchEmotes.items.filter((e) => !seen.has(e.id))];
      pageNo += 1;
      if (asked || scrollY > 0) replaceState(`/search${toSearch({ ...here, pages: Math.min(pageNo, MAX_PAGES) })}`, page.state);
    } catch (e) {
      moreError = e instanceof ApiError ? e.message : 'Couldn’t load more emotes.';
    } finally {
      busy = false;
    }
  }

  let end = $state<HTMLElement>();
  $effect(() => {
    if (!end || !hasMore || moreError) return;
    // A new observer per page: it reports at once, so a tall screen keeps filling.
    void pageNo;
    const io = new IntersectionObserver(
      (hits) => {
        // With a client-side filter that hides everything so far, keep going a little, not forever.
        if (hits.some((h) => h.isIntersecting) && (shown.length > 0 || pageNo < 8)) void loadMore(false);
      },
      { rootMargin: '0px 0px 800px 0px' },
    );
    io.observe(end);
    return () => io.disconnect();
  });

  // ---- Back navigation brings the scroll position back (the URL already holds the page count).
  export const snapshot = {
    capture: () => ({ y: scrollY }),
    restore: (s: { y: number }) => requestAnimationFrame(() => scrollTo({ top: s.y, behavior: 'instant' })),
  };

  const countText = $derived(
    `${count(clientFiltered ? shown.length : data.total)}${clientFiltered && hasMore ? '+' : ''} ${
      (clientFiltered ? shown.length : data.total) === 1 ? 'emote' : 'emotes'
    }`,
  );
  const moreHref = $derived(`/search${toSearch({ ...here, pages: Math.min(pageNo + 1, MAX_PAGES) })}`);
</script>

<svelte:head>
  <title>{q ? `${[...q].length > 40 ? [...q].slice(0, 40).join('') + '…' : q} - ` : ''}Emotes - cyex.tv</title>
  <meta name="description" content="Search approved cyex.tv emotes by name and tag." />
</svelte:head>

{#snippet field()}
  <SearchField bind:value={text} bind:input label="Emote name or tag" placeholder="Name or tag" oninput={typed} onsubmit={submit} onclear={clearText} />
{/snippet}

{#snippet railHead()}
  <div class="rail-head">
    {#if narrow}
      <p class="rail-on">{filters.length} on</p>
    {:else}
      <p class="rail-title">Filters{#if filters.length}<span class="n">{filters.length}</span>{/if}</p>
    {/if}
    {#if filters.length}<button class="reset" type="button" onclick={() => go({ ...CLEARED })}>Reset</button>{/if}
  </div>
{/snippet}

{#if view.tab === 'users'}
  <UsersSearch {data} />
{:else}
<RailLayout>
  {#snippet rail()}
    <!-- In the sheet its title already says Filters: the head is only there to Reset. -->
    <SideRail
      label="Emote filters"
      collapse="sheet"
      sheetLabel="Filters"
      bar={narrow ? field : undefined}
      sheetCount={filters.length}
      head={narrow && !filters.length ? undefined : railHead}
    >
      <BrowseFilters {view} {tags} onchange={(o) => go(o)} search={narrow ? undefined : field} />
    </SideRail>
  {/snippet}

  <div class="browse">
    <div class="toolbar" bind:this={gridTop}>
      <div class="title">
        <h1>Emotes</h1>
        {#if !data.error}<p class="total" aria-live="polite">{countText}</p>{/if}
      </div>
      <div class="tools">
        <SearchTabs {view} />
        <Segmented label="Sort" size="s" options={sorts} value={view.sort} onchange={(v) => go({ sort: v }, true)} />
        <div class="dens" role="group" aria-label="View">
          <IconButton icon="grid" label="Comfortable grid" size="s" variant="outline" pressed={layout === 'grid' && density === 'comfortable'} onclick={() => setDensity('comfortable')} />
          <IconButton icon="grid-dense" label="Compact grid" size="s" variant="outline" pressed={layout === 'grid' && density === 'compact'} onclick={() => setDensity('compact')} />
          <IconButton icon="list" label="Table" size="s" variant="outline" pressed={layout === 'table'} onclick={() => setLayout('table')} />
        </div>
        <button class="ui-btn secondary small selbtn" type="button" aria-pressed={selecting} onclick={() => setSelecting(!selecting)}>Select</button>
      </div>
    </div>

    {#if filters.length}
      <div class="chips" bind:this={chipsEl} role="list" aria-label="Active filters">
        {#each filters as f, i (f.key)}
          <span role="listitem"><Chip label={f.label} onremove={() => removeChip(f.key, i)} /></span>
        {/each}
        <button class="link clearall" type="button" bind:this={clearAll} onclick={() => go({ ...CLEARED })}>Clear all</button>
      </div>
    {/if}

    {#if data.error}
      <div class="state">
        <State kind="error" title="Couldn’t load emotes.">
          {data.error}
          {#snippet actions()}
            <button class="ui-btn secondary" type="button" onclick={() => invalidateAll()}>Try again</button>
          {/snippet}
        </State>
      </div>
    {:else if shown.length === 0 && !hasMore && !switching}
      <div class="state">
        <State title={q ? `No emotes match “${iso(q)}”.` : 'No emotes here.'}>
          {#if filters.length}
            {filters.map((f) => f.label).join(', ')} {filters.length === 1 ? 'is' : 'are'} on. Removing {filters.length === 1 ? 'it' : 'one'} may bring emotes back.
          {:else if view.q}
            Check the spelling, or try a shorter name or a tag.
          {:else}
            Approved emotes show up here.
          {/if}
          {#snippet actions()}
            {#if filters.length}
              <button class="ui-btn secondary" type="button" onclick={() => go({ ...CLEARED })}>Clear filters</button>
            {:else if view.q}
              <button class="ui-btn secondary" type="button" onclick={clearText}>Clear search</button>
            {/if}
          {/snippet}
        </State>
      </div>
    {:else}
      <div class="results" class:dim={switching}>
        {#if layout === 'table'}
          <EmoteTable
            items={shown}
            sort={view.sort}
            onsort={(s) => go({ sort: s }, true)}
            {looks}
            picked={selecting ? pickedSet : undefined}
            onpick={selecting ? toggle : undefined}
            eager={12}
          />
        {:else}
          <EmoteGrid
            items={shown}
            {density}
            eager={12}
            loading={busy || (switching && shown.length === 0)}
            skeletons={density === 'compact' ? 16 : 10}
            label="Emotes"
            picked={selecting ? pickedSet : undefined}
            onpick={selecting ? toggle : undefined}
          />
        {/if}
      </div>
      <div class="more" bind:this={end}>
        <LoadMore {hasMore} {busy} error={moreError} total={shown.length > 0 ? (clientFiltered ? shown.length : data.total) : 0} onmore={loadMore} href={moreHref} />
      </div>
      {#if selecting}
        <BulkAdd
          ids={picked}
          shown={shown.length}
          {signin}
          onall={() => (picked = [...new Set([...picked, ...shown.map((e) => e.id)])])}
          onclear={() => (picked = [])}
          onexit={() => setSelecting(false)}
          onadded={(ids) => (picked = picked.filter((id) => !ids.includes(id)))}
        />
      {/if}
    {/if}
  </div>
</RailLayout>
{/if}

<style>
  /* Rail head: the rail's title, the number of filters on, and Reset. */
  .rail-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; padding-left: 8px; }
  .rail-title { display: flex; align-items: center; gap: 8px; margin: 0; font-size: var(--t-ui); font-weight: 600; letter-spacing: -.01em; }
  .rail-on { margin: 0; font-size: var(--t-nav); color: var(--muted); }
  .n { display: inline-grid; place-items: center; min-width: 18px; height: 18px; padding: 0 5px; border-radius: var(--radius);
    background: var(--accent-dim); color: var(--accent-text); font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .reset { height: 40px; padding: 0 10px; border: 0; border-radius: var(--radius); background: none; color: var(--accent-text);
    font: inherit; font-size: var(--t-fine); font-weight: 500; cursor: pointer; }
  .reset:active { opacity: .6; }

  /* Toolbar: title and count left; sort and density right; one 40px row like the header's. */
  .toolbar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 16px; min-height: 40px; }
  @media (max-width: 899.98px) { .toolbar { padding-top: 4px; } }
  .title { display: flex; align-items: baseline; gap: 10px; min-width: 0; }
  h1 { margin: 0; font-size: var(--t-page); line-height: 1.25; letter-spacing: -.02em; }
  .total { margin: 0; font-size: var(--t-nav); color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .tools { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; min-width: 0; max-width: 100%; }
  .dens { display: inline-flex; gap: 4px; }
  .dens :global(.ib) { width: 40px; height: 40px; }
  .selbtn { height: 34px; }
  .selbtn[aria-pressed='true'] { font-weight: 700; color: var(--accent-text); border-color: var(--accent-border); }

  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 12px; }
  .clearall { background: none; border: 0; padding: 0 8px; font: inherit; font-size: var(--t-small); cursor: pointer; min-height: 40px; margin-block: 0; }

  .results { margin-top: 16px; transition: opacity var(--dur-color) ease-out; }
  .results.dim { opacity: .55; transition-delay: .15s; }
  .more { padding: 32px 0 8px; min-height: 96px; }

  .state { margin-top: 16px; }

  @media (max-width: 480px) { .title { flex-direction: column; align-items: flex-start; gap: 0; } }
  @media (pointer: coarse) { .dens :global(.ib) { width: 44px; height: 44px; } }
  @media (prefers-reduced-motion: reduce) { .results { transition: none; } }
</style>
