<script lang="ts">
  import { goto, invalidateAll, replaceState } from '$app/navigation';
  import { navigating } from '$app/state';
  import { untrack } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import Chip from '$lib/components/Chip.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import FilterSheet from '$lib/components/browse/FilterSheet.svelte';
  import { apiSort, CLEARED, MAX_PAGES, PER_PAGE, active as activeOf, change, matches, toSearch, type Browse, type Sort } from '$lib/components/browse/params';
  import { count } from '$lib/format';
  import { SEARCH } from '$lib/queries';

  let { data } = $props();

  const view = $derived(data.view);
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
  const clientFiltered = $derived(view.zeroWidth || view.wide);
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
          query: view.q,
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
    restore: (s: { y: number }) => requestAnimationFrame(() => scrollTo(0, s.y)),
  };

  const countText = $derived(
    `${count(clientFiltered ? shown.length : data.total)}${clientFiltered && hasMore ? '+' : ''} ${
      (clientFiltered ? shown.length : data.total) === 1 ? 'emote' : 'emotes'
    }`,
  );
  const moreHref = $derived(`/search${toSearch({ ...view, pages: Math.min(pageNo + 1, MAX_PAGES) })}`);
</script>

<svelte:head>
  <title>{view.q ? `${view.q} - ` : ''}Emotes - cyex.tv</title>
  <meta name="description" content="Search approved cyex.tv emotes by name and tag." />
</svelte:head>

<div class="page">
  <header class="head">
    <h1>Emotes</h1>
    {#if !data.error}
      <p class="total muted" aria-live="polite">{countText}</p>
    {/if}
  </header>

  <form class="search" role="search" onsubmit={submit}>
    <Icon name="search" size={20} />
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
    />
    {#if text}
      <span class="clear"><IconButton icon="x" label="Clear search" size="s" tip="none" onclick={clearText} type="button" /></span>
    {/if}
  </form>

  <div class="controls">
    <Segmented label="Sort" size="s" options={sorts} value={view.sort} onchange={(v) => go({ sort: v })} />
    <Popover label="Filters" sheet width="340px">
      {#snippet trigger(p)}
        <button class="btn btn-s filters-btn" type="button" {...p}>
          <Icon name="sliders" size={16} /> Filters{#if filters.length}<span class="n">{filters.length}</span>{/if}
        </button>
      {/snippet}
      <FilterSheet {view} onchange={(o) => go(o)} />
      {#snippet footer()}
        <span class="foot">{countText}</span>
        {#if filters.length}<button class="btn btn-s" type="button" onclick={() => go({ ...CLEARED })}>Reset</button>{/if}
      {/snippet}
    </Popover>
    <span class="grow"></span>
    <div class="dens" role="group" aria-label="Density">
      <IconButton icon="grid" label="Comfortable" size="s" variant="outline" pressed={density === 'comfortable'} onclick={() => setDensity('comfortable')} />
      <IconButton icon="grid-dense" label="Compact" size="s" variant="outline" pressed={density === 'compact'} onclick={() => setDensity('compact')} />
    </div>
  </div>

  {#if filters.length}
    <div class="chips" role="list" aria-label="Active filters">
      {#each filters as f (f.key)}
        <span role="listitem"><Chip label={f.label} onremove={() => go({ [f.key]: CLEARED[f.key] })} /></span>
      {/each}
      <button class="link clearall" type="button" onclick={() => go({ ...CLEARED })}>Clear all</button>
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
      <h2>No emotes{view.q ? ` match “${view.q}”` : ' here'}.</h2>
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
      {#if moreError}
        <p class="muted" role="alert">{moreError}</p>
        <button class="btn btn-s" type="button" onclick={loadMore}>Try again</button>
      {:else if hasMore}
        <a class="btn btn-s" href={moreHref} onclick={(e) => { e.preventDefault(); void loadMore(); }} aria-disabled={busy}>
          {busy ? 'Loading…' : 'Load more'}
        </a>
      {:else if shown.length > 0}
        <p class="muted fine">That’s everything{view.q ? ` for “${view.q}”` : ''}.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .head { display: flex; align-items: baseline; justify-content: space-between; gap: 16px; padding: clamp(28px, 4vw, 56px) 0 20px; }
  h1 { font-size: var(--t-h3-sub); line-height: 1.1; letter-spacing: -.03em; margin: 0; }
  .total { margin: 0; font-variant-numeric: tabular-nums; white-space: nowrap; }

  .search { position: relative; display: flex; align-items: center; color: var(--muted); }
  .search > :global(svg) { position: absolute; left: 16px; pointer-events: none; }
  .search input { width: 100%; height: 56px; padding: 0 56px 0 48px; font: inherit; font-size: 18px; color: var(--text);
    background: var(--surface); border: 1px solid var(--border-strong); border-radius: var(--radius-btn);
    transition: border-color var(--dur-color); -webkit-appearance: none; appearance: none; }
  .search input::-webkit-search-cancel-button { display: none; }
  .search input::placeholder { color: var(--muted); }
  .search input:focus-visible { outline: 2px solid var(--accent-text); outline-offset: 2px; }
  .clear { position: absolute; right: 8px; display: inline-flex; }
  @media (hover: hover) and (pointer: fine) { .search input:hover { border-color: var(--muted); } }

  .controls { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin: 16px 0 0; }
  .grow { flex: 1; }
  .dens { display: inline-flex; gap: 4px; }
  .filters-btn { gap: 8px; }
  .n { display: inline-grid; place-items: center; min-width: 20px; height: 20px; padding: 0 5px; border-radius: 3px;
    background: var(--text); color: var(--bg); font-size: 12px; font-weight: 600; }
  .foot { margin-right: auto; font-size: var(--t-small); color: var(--muted); font-variant-numeric: tabular-nums; }

  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 12px; }
  .clearall { background: none; border: 0; padding: 4px 6px; font: inherit; font-size: var(--t-small); cursor: pointer; min-height: 32px; }

  .results { margin-top: 20px; transition: opacity var(--dur-color) ease-out; }
  .results.dim { opacity: .55; }
  .more { display: grid; justify-items: center; gap: 12px; padding: 32px 0 8px; min-height: 96px; }
  .more p { margin: 0; }
  .more a[aria-disabled='true'] { pointer-events: none; opacity: .7; }

  .state { display: grid; justify-items: start; gap: 12px; padding: 48px 0; }
  .state h2 { margin: 0; font-size: var(--t-h4, 22px); letter-spacing: -.02em; }
  .state p { margin: 0; max-width: 52ch; }

  @media (max-width: 720px) {
    .controls { flex-wrap: nowrap; overflow-x: auto; scrollbar-width: none; margin-inline: -16px; padding-inline: 16px; padding-block: 2px; }
    .controls::-webkit-scrollbar { display: none; }
    .controls > :global(*) { flex: none; }
    .grow { display: none; }
    .dens { margin-left: auto; }
  }
  @media (prefers-reduced-motion: reduce) { .results { transition: none; } }
</style>
