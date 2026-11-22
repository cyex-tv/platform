<script lang="ts">
  // What happened on an account (7TV's activity feed): one row per change with an icon (added,
  // removed, renamed, paint or badge changed, an editor added or removed, …), a sentence with the
  // emote's picture and the names, who did it, and the date and time. Public. Pages back 30 at a
  // time. A filter bar narrows it: what (Everything, Emotes, Sets, Cosmetics, Editors), by whom
  // (UserFilter: focused empty, the people with the most lines here, the owner first; typing, the
  // people search) and when (the cyex logs timeframe and date picker). The filters live in the
  // address (?k=, ?by=, ?d= or ?m=), so a view can be shared.
  import RetryNote from '$lib/components/RetryNote.svelte';
  import { quiet } from '$lib/quiet';
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { fetchWith, gql } from '$lib/api';
  import UserFilter from '$lib/components/UserFilter.svelte';
  import type { Person } from '$lib/components/user-filter';
  import { ACTIVITY_TOP_ACTORS } from '$lib/queries';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { appendUnique } from '$lib/tool/paging';
  import { filterOf, loadActivity, type ActivityItem, type ActivityQuery } from '../activity';
  import ActivityRows from '../ActivityRows.svelte';
  import { profileOf } from '../data.svelte';
  import TimeframePicker from '../TimeframePicker.svelte';
  import { fromQuery, rangeOf, toQuery, type Timeframe } from '../timeframe';

  let { accountId, name = '' }: { accountId: string; name?: string } = $props();

  type Group = NonNullable<ActivityQuery['group']>;
  const GROUPS: { value: Group; label: string }[] = [
    { value: 'all', label: 'Everything' },
    { value: 'emotes', label: 'Emotes' },
    { value: 'sets', label: 'Sets' },
    { value: 'cosmetics', label: 'Cosmetics' },
    { value: 'editors', label: 'Editors' },
  ];
  const isGroup = (v: string | null): v is Group => GROUPS.some((g) => g.value === v);

  // The filters, read from the address.
  const q = $derived(page.url.searchParams);
  const group = $derived<Group>(isGroup(q.get('k')) ? (q.get('k') as Group) : 'all');
  const actor = $derived(q.get('by') ?? '');
  const frame = $derived<Timeframe>(fromQuery(q));
  const query = $derived<ActivityQuery>({ group, actor: actor || null, ...rangeOf(frame) });
  const filtered = $derived(group !== 'all' || !!actor || frame.frame !== 'all');

  // Who can be picked under "By": the owner and the account's editors (public, names only).
  let editors = $state<{ id: string; displayName: string }[]>([]);
  let editorsFailed = $state(false);
  function loadEditors() {
    editorsFailed = false;
    void quiet(profileOf(accountId).then((p) => (editors = p?.editors ?? [])), 'activity editors', () => (editorsFailed = true));
  }
  onMount(() => {
    loadEditors();
    void quiet(loadCatalog().then((c) => (catalog = c)), 'cosmetics catalog');
  });
  // The "By" filter: the owner first, then whoever has the most lines under the other filters;
  // the editors when nobody has any.
  const asPerson = (a: { id: string; displayName: string; avatar?: string | null }): Person => ({ account: a, identities: [] });
  const owner = $derived<Person[]>(name ? [asPerson({ id: accountId, displayName: name })] : []);
  const editorPeople = $derived(editors.filter((e) => e.id !== accountId).map(asPerson));
  const editorIds = $derived(new Set(editors.map((e) => e.id)));
  const topKey = $derived(`${accountId}|${group}|${query.since ?? ''}|${query.until ?? ''}`);
  async function topActors(signal: AbortSignal): Promise<Person[]> {
    const { groups, since, until } = filterOf({ ...query, actor: null });
    const r = await gql(ACTIVITY_TOP_ACTORS, { accountId, filter: { groups, since, until }, limit: 5 }, fetchWith(signal));
    return r.activityTopActors;
  }
  const flagOf = (p: Person) => (p.account.id === accountId ? 'Owner' : editorIds.has(p.account.id) ? 'Editor' : null);

  function set(next: Record<string, string | null>) {
    const url = new URL(page.url);
    for (const [k, v] of Object.entries(next)) {
      if (v) url.searchParams.set(k, v);
      else url.searchParams.delete(k);
    }
    goto(url, { replaceState: true, keepFocus: true, noScroll: true });
  }
  const setFrame = (t: Timeframe) => set({ d: null, m: null, ...toQuery(t) });
  const reset = () => set({ k: null, by: null, d: null, m: null });

  // On a phone the five groups scroll sideways; an edge fades where there is more to reach.
  let strip = $state<HTMLDivElement>();
  let fadeStart = $state(false);
  let fadeEnd = $state(false);
  // The Segmented control is what scrolls (it caps itself at the width it has).
  const scroller = () => strip?.firstElementChild as HTMLElement | null | undefined;
  function edges() {
    const el = scroller();
    if (!el) return;
    fadeStart = el.scrollLeft > 1;
    fadeEnd = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
  }
  $effect(() => {
    const el = strip && scroller();
    if (!el) return;
    const ro = new ResizeObserver(edges);
    ro.observe(el);
    el.addEventListener('scroll', edges, { passive: true });
    edges();
    return () => {
      ro.disconnect();
      el.removeEventListener('scroll', edges);
    };
  });

  let items = $state<ActivityItem[]>([]);
  let first = $state(true);
  let busy = $state(false);
  /** Where the next page starts; null once the feed has ended. */
  let cursor = $state<string | null>(null);
  const more = $derived(cursor !== null);
  let problem = $state('');
  let catalog = $state<Catalog | null>(null);
  // Answers for an older filter are dropped.
  let round = 0;

  async function next(fresh = false) {
    if (busy && !fresh) return;
    const mine = fresh ? ++round : round;
    busy = true;
    problem = '';
    try {
      const got = await loadActivity(accountId, fresh ? null : cursor, fetch, query);
      if (mine !== round) return;
      items = fresh ? got.items : appendUnique(items, got.items);
      cursor = got.next;
    } catch (e) {
      if (mine !== round) return;
      problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      if (mine === round) {
        busy = false;
        first = false;
      }
    }
  }

  // A new filter starts the feed over.
  $effect(() => {
    void query.group; void query.actor; void query.since; void query.until;
    untrack(() => {
      first = true;
      next(true);
    });
  });
</script>

<PageHead title="Activity" line="Changes to the sets, emotes, cosmetics and editors, newest first." />

<div class="bar" role="group" aria-label="Filter activity">
  <div class="groups" class:fade-start={fadeStart} class:fade-end={fadeEnd} bind:this={strip}><Segmented label="Show" size="s" options={GROUPS} value={group} onchange={(v) => set({ k: v === 'all' ? null : v })} /></div>
  <UserFilter id="activity-by" label="By" value={actor} onchange={(id) => set({ by: id || null })} top={topActors} {topKey}
    pinned={owner} fallback={editorPeople} flag={flagOf} name={actor === accountId ? name : null} />
  <TimeframePicker value={frame} onchange={setFrame} />
</div>
{#if editorsFailed}
  <RetryNote onretry={loadEditors}>Couldn’t load the editors to pick under “By”.</RetryNote>
{/if}

{#if first}
  <State kind="loading" title="Loading activity…" />
{:else if problem && !items.length}
  <Problem cause="server" onretry={() => next(true)} />
{:else if !items.length}
  {#if filtered}
    <State title="Nothing matches.">
      Nothing like that happened then. Try another timeframe, or show everything.
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={reset}>Show everything</button>{/snippet}
    </State>
  {:else}
    <State title="Nothing yet.">Changes to the sets, emotes, cosmetics and editors show up here.</State>
  {/if}
{:else}
  <ActivityRows {items} {catalog} subject={accountId} />
  <div class="foot"><LoadMore hasMore={more} {busy} error={problem} total={items.length} onmore={() => next()} /></div>
{/if}

<style>
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; margin: 0 0 16px; }
  .foot { padding: 20px 0 0; }
  @media (max-width: 640px) {
    .bar { gap: 10px; }
    /* Five groups don't fit a phone: they scroll sideways, edge to edge, without a bar. */
    .groups { width: calc(100% + 2 * var(--page-pad)); margin-inline: calc(-1 * var(--page-pad)); padding-inline: var(--page-pad);
      overflow-x: auto; scrollbar-width: none; overscroll-behavior-x: contain; }
    .groups::-webkit-scrollbar { display: none; }
    .groups { -webkit-mask-image: linear-gradient(90deg, transparent, #000 var(--fs, 0px), #000 calc(100% - var(--fe, 0px)), transparent);
      mask-image: linear-gradient(90deg, transparent, #000 var(--fs, 0px), #000 calc(100% - var(--fe, 0px)), transparent); }
    .groups.fade-start { --fs: 28px; }
    .groups.fade-end { --fe: 28px; }
    .groups :global([role='radiogroup']) { width: max-content; }
  }
</style>
