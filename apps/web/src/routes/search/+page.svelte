<script lang="ts">
  import { goto, invalidateAll, replaceState } from '$app/navigation';
  import { navigating } from '$app/state';
  import { tick, untrack } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import Chip from '$lib/components/Chip.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import BrowseFilters from '$lib/components/browse/BrowseFilters.svelte';
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
    toSearch,
    topTags,
    type Browse,
    type Sort,
  } from '$lib/components/browse/params';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { clean, count } from '$lib/format';
  import { SEARCH } from '$lib/queries';

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

  function go(over: Partial<Browse>, replace = false) {
    const next = change(view, over);
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

  // ---- Density, remembered on this device.
  type Density = 'comfortable' | 'compact';
  let density = $state<Density>('comfortable');
  $effect(() => {
    try {
      const d = localStorage.getItem('cyex.browse.density');
      if (d === 'compact' || d === 'comfortable') density = d;
    } catch {
      /* private mode: keep the default */
    }
  });
  function setDensity(d: Density) {
    density = d;
    try {
      localStorage.setItem('cyex.browse.density', d);
    } catch {
      /* not remembered */
    }
  }

  // ---- Load more: button, and automatically when the end comes near.
  async function loadMore() {
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
      replaceState(`/search${toSearch({ ...view, pages: Math.min(pageNo, MAX_PAGES) })}`, {});
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
        if (hits.some((h) => h.isIntersecting) && (shown.length > 0 || pageNo < 8)) void loadMore();
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
  const moreHref = $derived(`/search${toSearch({ ...view, pages: Math.min(pageNo + 1, MAX_PAGES) })}`);
</script>

<svelte:head>
  <title>{q ? `${[...q].length > 40 ? [...q].slice(0, 40).join('') + '…' : q} - ` : ''}Emotes - cyex.tv</title>
  <meta name="description" content="Search approved cyex.tv emotes by name and tag." />
</svelte:head>

{#snippet field()}
  <form class="search" role="search" onsubmit={submit}>
    <Icon name="search" size={16} />
    <label class="sr-only" for="q">Emote name or tag</label>
    <input
      id="q"
      bind:this={input}
      bind:value={text}
      oninput={typed}
      type="search"
      name="q"
      placeholder="Name or tag"
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      enterkeyhint="search"
      data-page-search
    />
    {#if text}
      <span class="clear"><IconButton icon="x" label="Clear search" size="s" tip="none" onclick={clearText} type="button" /></span>
    {/if}
  </form>
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
        <Segmented label="Sort" size="s" height={28} options={sorts} value={view.sort} onchange={(v) => go({ sort: v }, true)} />
        <div class="dens" role="group" aria-label="Density">
          <IconButton icon="grid" label="Comfortable" size="s" variant="outline" pressed={density === 'comfortable'} onclick={() => setDensity('comfortable')} />
          <IconButton icon="grid-dense" label="Compact" size="s" variant="outline" pressed={density === 'compact'} onclick={() => setDensity('compact')} />
        </div>
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
      <div class="state" role="alert">
        <h2>Couldn’t load emotes.</h2>
        <p class="muted">{data.error}</p>
        <button class="btn btn-s" type="button" onclick={() => invalidateAll()}>Try again</button>
      </div>
    {:else if shown.length === 0 && !hasMore && !switching}
      <div class="state">
        <h2>No emotes{#if q}{' '}match “<bdi>{q}</bdi>”{:else}{' '}here{/if}.</h2>
        {#if filters.length}
          <p class="muted">
            {filters.map((f) => f.label).join(', ')} {filters.length === 1 ? 'is' : 'are'} on. Removing {filters.length === 1 ? 'it' : 'one'} may bring emotes back.
          </p>
          <button class="btn btn-s" type="button" onclick={() => go({ ...CLEARED })}>Clear filters</button>
        {:else if view.q}
          <p class="muted">Check the spelling, or try a shorter name or a tag.</p>
          <button class="btn btn-s" type="button" onclick={clearText}>Clear search</button>
        {:else}
          <p class="muted">Approved emotes show up here.</p>
        {/if}
      </div>
    {:else}
      <div class="results" class:dim={switching}>
        <EmoteGrid items={shown} {density} eager={12} loading={busy || (switching && shown.length === 0)} skeletons={density === 'compact' ? 16 : 10} label="Emotes" />
      </div>
      <div class="more" bind:this={end}>
        <LoadMore {hasMore} {busy} error={moreError} total={shown.length > 0 ? (clientFiltered ? shown.length : data.total) : 0} onmore={loadMore} href={moreHref} />
      </div>
    {/if}
  </div>
</RailLayout>

<style>
  /* Rail head: the rail's title, the number of filters on, and Reset. */
  .rail-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 28px; padding-left: 8px; }
  .rail-title { display: flex; align-items: center; gap: 8px; margin: 0; font-size: var(--t-ui); font-weight: 600; letter-spacing: -.01em; }
  .rail-on { margin: 0; font-size: var(--t-nav); color: var(--muted); }
  .n { display: inline-grid; place-items: center; min-width: 18px; height: 18px; padding: 0 5px; border-radius: var(--radius);
    background: var(--accent-dim); color: var(--accent-text); font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .reset { height: 28px; padding: 0 8px; border: 0; border-radius: var(--radius); background: none; color: var(--accent-text);
    font: inherit; font-size: var(--t-fine); font-weight: 500; cursor: pointer; }
  .reset:active { opacity: .6; }

  /* The search field: in the rail on wide screens, above the toolbar on narrow ones. */
  .search { position: relative; display: flex; align-items: center; min-width: 0; color: var(--muted); }
  .search > :global(svg) { position: absolute; left: 10px; pointer-events: none; }
  .search input { width: 100%; height: 36px; padding: 0 36px 0 32px; font: inherit; font-size: 16px; color: var(--text);
    background: var(--bg); border: 1px solid var(--border-strong); border-radius: var(--radius-btn);
    transition: border-color var(--fast); -webkit-appearance: none; appearance: none; }
  .search input::-webkit-search-cancel-button { display: none; }
  .search input::placeholder { color: var(--dim); }
  .search input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .clear { position: absolute; right: 2px; display: inline-flex; }
  @media (hover: hover) and (pointer: fine) { .search input:hover { border-color: var(--muted); } }
  /* Narrow: the field sits in the sticky Filters bar, beside the button, at its height. */
  :global(.bar-extra) .search input { height: 40px; background: var(--surface); }

  /* Toolbar: title and count left; sort and density right; one 36px row like the header's. */
  .toolbar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 16px; min-height: 36px; }
  @media (max-width: 899.98px) { .toolbar { padding-top: 4px; } }
  .title { display: flex; align-items: baseline; gap: 10px; min-width: 0; }
  h1 { margin: 0; font-size: 22px; line-height: 1.25; letter-spacing: -.02em; }
  .total { margin: 0; font-size: var(--t-nav); color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .tools { display: flex; align-items: center; gap: 8px; }
  .dens { display: inline-flex; gap: 4px; }
  .dens :global(.ib) { width: 36px; height: 36px; }

  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 12px; }
  .clearall { background: none; border: 0; padding: 4px 6px; font: inherit; font-size: var(--t-small); cursor: pointer; min-height: 32px; }

  .results { margin-top: 16px; transition: opacity var(--dur-color) ease-out; }
  .results.dim { opacity: .55; transition-delay: .15s; }
  .more { padding: 32px 0 8px; min-height: 96px; }

  .state { display: grid; justify-items: start; gap: 12px; padding: 40px 0; }
  .state h2 { margin: 0; font-size: var(--t-block); letter-spacing: -.02em; }
  .state h2, .state p { overflow-wrap: anywhere; }
  .state p { margin: 0; max-width: 52ch; }

  @media (max-width: 480px) { .title { flex-direction: column; align-items: flex-start; gap: 0; } }
  @media (pointer: coarse) { .dens :global(.ib) { width: 44px; height: 44px; } }
  @media (prefers-reduced-motion: reduce) { .results { transition: none; } }
</style>
