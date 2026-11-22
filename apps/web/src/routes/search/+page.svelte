<script lang="ts">
  // The browse page, in the Workspace layout (redesign, 2026/11/21). Signed in, three columns: the
  // set rail on the left (SetRail: every set you can add to, the target picked there), the grid in
  // the middle (sort, search, filters, Select; a + on every tile adds to the target), and from
  // 1280px the tray on the right (TargetTray: what the target holds, live). Under 900px the rail
  // is a bottom sheet opened from the "Adding to" button. Signed out the grid has the full width
  // and a short line offers sign in; sorting, filters and search work the same. Every filter
  // lives in the address (params.ts).
  import { goto, invalidateAll, replaceState } from '$app/navigation';
  import { navigating, page } from '$app/state';
  import { tick, untrack } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import Chip from '$lib/components/Chip.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import BrowseFilters from '$lib/components/browse/BrowseFilters.svelte';
  import BrowseGrid from '$lib/components/browse/BrowseGrid.svelte';
  import { personal } from '$lib/personal/lists.svelte';
  import SelectBar from '$lib/components/browse/SelectBar.svelte';
  import SetRail from '$lib/components/browse/SetRail.svelte';
  import TargetTray from '$lib/components/browse/TargetTray.svelte';
  import BottomSheet from '$lib/components/BottomSheet.svelte';
  import { target as sets, setLabel } from '$lib/components/browse/target.svelte';
  import { all, CAP, EMPTY, toggle, type Selection, type Step } from '$lib/components/browse/selection';
  import { session } from '$lib/tool/session.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import {
    apiVars,
    CLEARED,
    MAX_PAGES,
    PER_PAGE,
    WINDOWS,
    active as activeOf,
    change,
    parse,
    toSearch,
    topTags,
    without,
    type Active,
    type Browse,
    type Sort,
    type Window,
  } from '$lib/components/browse/params';
  import { clean, count, iso } from '$lib/format';
  import { SEARCH } from '$lib/queries';
  import SearchField from '$lib/components/browse/SearchField.svelte';
  import { registerShortcuts } from '$lib/shell/shortcuts.svelte';
  import { BROWSE_KEYS, SELECT_KEYS } from '$lib/components/browse/keys';

  let { data } = $props();

  // The keys of the emote grid and of Select mode, in the shortcut sheet.
  $effect(() => {
    const a = registerShortcuts(BROWSE_KEYS);
    const b = registerShortcuts(SELECT_KEYS);
    return () => (a(), b());
  });

  const view = $derived(data.view);
  const q = $derived(clean(view.q));
  // The query as it goes in the tab title: no control characters, cut short, and nothing at all when nothing is left.
  const titleQ = $derived.by(() => {
    const t = [...q.replace(/[\p{Cc}\p{Cf}]/gu, '').trim()];
    return t.length > 40 ? `${t.slice(0, 40).join('')}…` : t.join('');
  });
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
  // What you hid for yourself ("Hide for me") drops out here; the API answers the same for everyone.
  $effect(() => personal.ensure());
  const shown = $derived(personal.visible(loaded));
  const tags = $derived(topTags(loaded, 12, view.tags));
  const switching = $derived(!!navigating.to);
  const trending = $derived(view.sort === 'trending');

  const sorts = $derived<{ value: Sort; label: string; icon: IconName }[]>([
    ...(view.q ? [{ value: 'relevance' as const, label: 'Relevance', icon: 'search' as const }] : []),
    { value: 'top', label: 'Top', icon: 'arrow-up' },
    { value: 'trending', label: 'Trending', icon: 'trending' },
    { value: 'new', label: 'New', icon: 'sparkles' },
  ]);
  const windows = WINDOWS.map((w) => ({ value: w.value, label: w.short }));
  const windowLabel = $derived(WINDOWS.find((w) => w.value === view.window)?.label.toLowerCase() ?? '');

  // ---- Search field: the URL follows the typing (replaced, not pushed, after a short pause).
  let text = $state(untrack(() => data.view.q));
  let input = $state<HTMLInputElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const q = view.q;
    if (document.activeElement !== input) text = q;
  });

  // Select mode is in the URL too, set without a reload.
  let selecting = $state(untrack(() => data.view.select));
  // Follows the address (which `sync` and every navigation keep in step with it).
  $effect.pre(() => {
    selecting = parse(page.url.searchParams).select;
  });
  const here = $derived<Browse>({ ...view, select: selecting });

  function go(over: Partial<Browse>, replace = false) {
    const next = change(here, over);
    return goto(`/search${toSearch(next)}`, { replaceState: replace, keepFocus: true, noScroll: true });
  }
  let clearAll = $state<HTMLButtonElement>();
  let chipsEl = $state<HTMLElement>();
  let gridTop = $state<HTMLElement>();
  // Removing a chip would drop focus to the page: hand it to the next chip's remove button,
  // else Clear all, else the toolbar.
  async function removeChip(key: Active['key'], i: number) {
    await go(without(here, key));
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

  /** Puts the local view state into the address without loading anything again. */
  function sync(pages = pageNo) {
    replaceState(`/search${toSearch({ ...here, pages: Math.min(pages, MAX_PAGES) })}`, page.state);
  }

  // ---- Multi-select (up to CAP): picked ids in the order picked, kept while filters change.
  let sel = $state.raw<Selection>(EMPTY);
  const order = $derived(shown.map((e) => e.id));
  let note = $state('');
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  function step(s: Step) {
    if (s.sel !== sel && !s.refused) note = ''; // the cap note goes once the picks change again
    sel = s.sel;
    if (s.refused) {
      note = `That’s ${CAP}, the most at once. Add these first, then pick more.`;
      clearTimeout(noteTimer);
      noteTimer = setTimeout(() => (note = ''), 4000);
    }
  }
  function setSelecting(on: boolean, id?: string) {
    selecting = on;
    if (!on) sel = EMPTY;
    else if (id && !sel.ids.includes(id)) step(toggle(sel, id));
    sync(on ? pageNo : 1); // leaving Select mode leaves a plain address: no ?select=, no ?n=
  }
  function unpick(done: string[]) {
    note = '';
    sel = { ...sel, ids: sel.ids.filter((id) => !done.includes(id)) };
    return sel.ids;
  }
  // Escape clears the picks (and leaves Select mode when nothing is picked); Ctrl/⌘+A picks what
  // is shown, up to the cap. Not while typing, or inside a menu, popover or sheet.
  function onkeydown(e: KeyboardEvent) {
    if (!selecting || e.defaultPrevented) return;
    const t = e.target as HTMLElement;
    if (t.closest('input, textarea, select, [contenteditable="true"], [role="menu"], [role="dialog"]')) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      note = '';
      if (sel.ids.length) sel = EMPTY;
      else setSelecting(false);
    } else if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'a') {
      e.preventDefault();
      step(all(sel, order));
    }
  }
  const signin = $derived(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`);

  // ---- The workspace: signed in (or, before the session is known, signed in last time).
  const signed = $derived(!!session.me && !session.ended);
  $effect(() => {
    if (signed) sets.ensure();
  });
  let railSheet = $state(false);
  // The tray folds to a strip; remembered on this device (app.html reads it before first paint).
  let trayOpen = $state(true);
  $effect(() => {
    try {
      trayOpen = localStorage.getItem('cyex.browse.tray') !== '0';
    } catch {
      trayOpen = true;
    }
  });
  function toggleTray() {
    trayOpen = !trayOpen;
    document.documentElement.dataset.tray = trayOpen ? '' : 'shut';
    if (trayOpen) delete document.documentElement.dataset.tray;
    try {
      localStorage.setItem('cyex.browse.tray', trayOpen ? '1' : '0');
    } catch {
      /* not remembered */
    }
  }

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
        apiVars(view, pageNo + 1),
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
        if (hits.some((h) => h.isIntersecting)) void loadMore(false);
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

  const countText = $derived(`${count(data.total)} ${data.total === 1 ? 'emote' : 'emotes'}`);
  const emptyTitle = $derived(
    q ? `No emotes match “${iso(q)}”.` : trending ? `Nothing is trending, ${windowLabel}.` : 'No emotes here.',
  );
  const moreHref = $derived(`/search${toSearch({ ...here, pages: Math.min(pageNo + 1, MAX_PAGES) })}`);
</script>

<svelte:head>
  <title>{titleQ ? `${titleQ} - ` : ''}Emotes - cyex.tv</title>
  <meta name="description" content="Search approved cyex.tv emotes by name and tag." />
</svelte:head>

{#snippet field()}
  <SearchField bind:value={text} bind:input label="Emote name or tag" placeholder="Name or tag" oninput={typed} onsubmit={submit} onclear={clearText} />
{/snippet}

<svelte:window {onkeydown} />

<div class="wrap ws" class:signed class:shut={!trayOpen}>
  <aside class="rail-col" aria-label="Your sets">
    {#if signed}<SetRail />{:else if !session.known}<div class="hold" aria-hidden="true"><span class="ui-skeleton"></span><span class="ui-skeleton"></span><span class="ui-skeleton"></span></div>{/if}
  </aside>
  <div class="browse">
    <div class="head">
      <div class="title">
        <h1>Emotes</h1>
        {#if !data.error}<p class="total" aria-live="polite">{countText}</p>{/if}
      </div>
      {#if session.known && !signed}
        <p class="invite"><a href={signin}>Sign in</a> to add emotes to your channel.</p>
      {/if}
    </div>

    <div class="toolbar" bind:this={gridTop}>
      <div class="sorts">
        <Segmented label="Sort" size="s" compact options={sorts} value={view.sort} onchange={(v) => go({ sort: v }, true)} />
        {#if trending}
          <Segmented label="Trending window" size="s" options={windows} value={view.window} onchange={(v: Window) => go({ window: v }, true)} />
        {/if}
      </div>
      <div class="find">{@render field()}</div>
      <div class="acts">
        <Popover label="Filters" sheet align="end" width="340px">
          {#snippet trigger(props)}
            <button class="btn btn-s filt" type="button" {...props}>
              <Icon name="sliders" size={15} />Filters{#if filters.length}<span class="n">{filters.length}</span>{/if}
            </button>
          {/snippet}
          <BrowseFilters {view} {tags} failed={!!data.error} onchange={(o) => go(o)} />
          {#snippet footer()}
            <button class="btn btn-s" type="button" disabled={!filters.length} onclick={() => go({ ...CLEARED })}>Reset</button>
          {/snippet}
        </Popover>
        <label class="switch" class:on={selecting}>
          <span>Select</span>
          <input type="checkbox" role="switch" checked={selecting} onchange={(e) => setSelecting(e.currentTarget.checked)} />
          <span class="track" aria-hidden="true"></span>
        </label>
        {#if signed}
          <button class="btn btn-s tgt" type="button" aria-haspopup="dialog" aria-expanded={railSheet} onclick={() => (railSheet = true)}>
            <Icon name="folder" size={15} /><span class="tw">Adding to</span>
            <bdi>{sets.current ? setLabel(sets.current) : sets.sets ? 'No set' : 'Your set'}</bdi>
            <Icon name="chevron-down" size={14} />
          </button>
        {:else if !session.known}
          <!-- Signed in last time (the first-paint hint): the button's place, so the toolbar doesn't wrap later. -->
          <span class="btn btn-s tgt tgt-hold" aria-hidden="true"><Icon name="folder" size={15} /><span class="tw">Adding to</span><bdi>Your set</bdi><Icon name="chevron-down" size={14} /></span>
        {/if}
      </div>
    </div>

    {#if trending}
      <p class="wlabel">Ranked by how many channels added them in the {windowLabel}.</p>
    {/if}

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
        <Problem cause="server" onretry={() => invalidateAll()} />
      </div>
    {:else if shown.length === 0 && !hasMore && !switching}
      <div class="state">
        <State title={emptyTitle}>
          {#if filters.length}
            {filters.map((f) => f.label).join(', ')} {filters.length === 1 ? 'is' : 'are'} on. Removing {filters.length === 1 ? 'it' : 'one'} may bring emotes back.
          {:else if view.q}
            Check the spelling, or try a shorter name or a tag.
          {:else if trending}
            Emotes show up here once channels add them. A longer window may have some.
          {:else}
            Approved emotes show up here.
          {/if}
          {#snippet actions()}
            {#if trending && !filters.length && !view.q && view.window !== 'month'}
              <button class="btn btn-s" type="button" onclick={() => go({ window: 'month' }, true)}>Show 30 days</button>
            {:else if filters.length}
              <button class="btn btn-s" type="button" onclick={() => go({ ...CLEARED })}>Clear filters</button>
            {:else if view.q}
              <button class="btn btn-s" type="button" onclick={clearText}>Clear search</button>
            {/if}
          {/snippet}
        </State>
      </div>
    {:else}
      <div class="results" class:dim={switching}>
        <BrowseGrid
          items={shown}
          eager={12}
          loading={busy || (switching && shown.length === 0)}
          skeletons={10}
          label={trending ? `Trending emotes, ${windowLabel}` : 'Emotes'}
          ranked={trending}
          {selecting}
          {sel}
          onstep={step}
          onstart={(id) => setSelecting(true, id)}
          onstop={() => setSelecting(false)}
        />
      </div>
      <div class="more" bind:this={end}>
        <LoadMore {hasMore} {busy} error={moreError} total={shown.length > 0 ? data.total : 0} onmore={loadMore} href={moreHref} />
      </div>
    {/if}
    {#if selecting && !data.error}
      <SelectBar
        ids={sel.ids}
        shown={shown.length}
        {signin}
        {note}
        onall={() => step(all(sel, order))}
        onclear={() => ((sel = EMPTY), (note = ''))}
        onexit={() => setSelecting(false)}
        onadded={unpick}
      />
    {/if}
  </div>
  <aside class="tray-col" aria-label={sets.current ? `What ${setLabel(sets.current)} holds` : 'Your set'}>
    {#if signed}<TargetTray open={trayOpen} ontoggle={toggleTray} />{/if}
  </aside>
</div>

{#if signed}
  <BottomSheet bind:open={railSheet} label="Add to which set">
    {#if railSheet}<SetRail sheet onpicked={() => (railSheet = false)} />{/if}
  </BottomSheet>
{/if}

<style>
  /* The workspace: the rail, the grid, the tray. Signed out (no hint either) only the grid. The
     columns are decided by CSS (and the first-paint hint), so nothing moves when the session lands. */
  .ws { display: grid; grid-template-columns: minmax(0, 1fr); column-gap: 24px; align-items: start; }
  .rail-col, .tray-col { display: none; }
  .browse { min-width: 0; padding-top: var(--rail-gap); padding-bottom: 32px; }
  @media (min-width: 900px) {
    :global(html[data-signed]) .ws, .ws.signed { grid-template-columns: 248px minmax(0, 1fr); }
    :global(html[data-signed]) .rail-col, .signed .rail-col { display: block; position: sticky; top: var(--nav-h); height: calc(100svh - var(--nav-h));
      overflow-y: auto; overscroll-behavior: contain; margin-left: calc(-1 * var(--page-pad)); padding: var(--rail-gap) 12px 24px var(--page-pad);
      border-right: 1px solid var(--border); scrollbar-width: thin; }
    .tgt { display: none; }
  }
  @media (min-width: 1280px) {
    :global(html[data-signed]) .ws, .ws.signed { grid-template-columns: 248px minmax(0, 1fr) 300px; }
    :global(html[data-signed][data-tray='shut']) .ws, .ws.signed.shut { grid-template-columns: 248px minmax(0, 1fr) 44px; }
    :global(html[data-signed]) .tray-col, .signed .tray-col { display: block; position: sticky; top: var(--nav-h); height: calc(100svh - var(--nav-h));
      overflow-y: auto; overscroll-behavior: contain; margin-right: calc(-1 * var(--page-pad)); border-left: 1px solid var(--border); background: var(--surface);
      scrollbar-width: thin; }
  }
  /* Signed out after all (the hint was stale): back to one column. */
  .ws:not(.signed) .rail-col:empty, .ws:not(.signed) .tray-col:empty { display: none; }
  .hold { display: grid; gap: 8px; }
  .hold span { display: block; height: 40px; border-radius: var(--radius-btn); }
  .invite { margin: 0; font-size: var(--t-small); color: var(--muted); }
  .invite a { color: var(--accent-text); font-weight: 600; }
  .tgt { height: 34px; max-width: 260px; min-width: 0; }
  .tgt bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 700; }
  .tgt .tw { color: var(--muted); font-weight: 500; }
  .tgt :global(svg) { flex: none; color: var(--dim); }
  .tgt-hold { visibility: hidden; pointer-events: none; }
  :global(html:not([data-signed])) .tgt-hold { display: none; }

  /* Title row: the one page title and the count. */
  .head { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 16px; min-height: 40px; }
  .title { display: flex; align-items: baseline; gap: 10px; min-width: 0; }
  h1 { margin: 0; font-size: var(--t-page); line-height: 1.25; letter-spacing: -.02em; }
  .total { margin: 0; font-size: var(--t-nav); color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }

  /* Toolbar: sort (and Trending's window), the search field, then Filters, Select and the target set. */
  .toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 12px; margin-top: 16px; }
  .sorts { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; min-width: 0; }
  .find { flex: 1 1 120px; min-width: 120px; max-width: 420px; }
  .acts { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; margin-left: auto; min-width: 0; }
  .filt { height: 34px; }
  .filt :global(svg) { color: var(--dim); }
  .n { display: inline-grid; place-items: center; min-width: 18px; height: 18px; padding: 0 5px; border-radius: 999px;
    background: var(--accent-dim); color: var(--accent-text); font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums; }

  /* Select: the cyex-app switch. Accent track when on, with a tick on it so on reads as on. */
  .switch { position: relative; display: inline-flex; align-items: center; gap: 8px; min-height: 40px; padding: 0 2px;
    font-size: var(--t-nav); font-weight: 500; color: var(--muted); cursor: pointer; user-select: none; }
  .switch.on { color: var(--text); font-weight: 700; }
  .switch input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .track { position: relative; width: 36px; height: 20px; border-radius: 999px; background: var(--border-strong);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--text) 40%, transparent); transition: background-color var(--fast) var(--ease); }
  .track::before { content: ''; position: absolute; top: 3px; left: 3px; width: 14px; height: 14px; border-radius: 50%; background: var(--text);
    transition: translate var(--fast) var(--ease), background-color var(--fast); }
  .track::after { content: ''; position: absolute; top: 6px; left: 9px; width: 2px; height: 8px; border-radius: 1px; background: var(--accent-ink); opacity: 0; }
  .on .track { background: var(--accent); box-shadow: none; }
  .on .track::before { translate: 16px 0; background: var(--accent-ink); }
  .on .track::after { opacity: 1; }
  .switch:has(input:focus-visible) .track { outline: 2px solid var(--focus-ring); outline-offset: 2px; }

  .wlabel { margin: 8px 0 0; font-size: var(--t-small); color: var(--muted); }
  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 12px; }
  .clearall { background: none; border: 0; padding: 0 8px; font: inherit; font-size: var(--t-small); cursor: pointer; min-height: 40px; margin-block: 0; }

  .results { margin-top: 16px; transition: opacity var(--dur-color) ease-out; }
  .results.dim { opacity: .55; transition-delay: .15s; }
  .more { padding: 32px 0 8px; min-height: 96px; }
  .state { margin-top: 16px; }

  /* Narrow: the field takes its own row first, the rest wraps under it. */
  @media (max-width: 760px) {
    .find { order: -1; flex-basis: 100%; max-width: none; }
    .acts { margin-left: 0; }
  }
  @media (max-width: 480px) { .title { flex-direction: column; align-items: flex-start; gap: 0; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .results, :global(html:not([data-motion='full'])) .track, :global(html:not([data-motion='full'])) .track::before { transition: none; } }
</style>
