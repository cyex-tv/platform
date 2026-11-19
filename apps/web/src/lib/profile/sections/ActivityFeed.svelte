<script lang="ts">
  // What happened on an account (7TV's activity feed): one row per change with an icon (added,
  // removed, renamed, paint or badge changed, an editor added or removed, …), a sentence with the
  // emote's picture and the names, who did it, and the date and time. Public. Pages back 30 at a
  // time. A filter bar narrows it: what (Everything, Emotes, Sets, Cosmetics, Editors), by whom
  // (anyone, the owner, or one of the account's editors) and when (the cyex logs timeframe and
  // date picker). The filters live in the address (?k=, ?by=, ?d= or ?m=), so a view can be shared.
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { appendUnique } from '$lib/tool/paging';
  import { loadActivity, type ActivityItem, type ActivityQuery } from '../activity';
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
  onMount(() => {
    profileOf(accountId)
      .then((p) => (editors = p?.editors ?? []))
      .catch(() => {});
    loadCatalog().then((c) => (catalog = c)).catch(() => {});
  });
  const people = $derived([{ id: accountId, displayName: name }, ...editors.filter((e) => e.id !== accountId)]);

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
  <div class="groups"><Segmented label="Show" size="s" options={GROUPS} value={group} onchange={(v) => set({ k: v === 'all' ? null : v })} /></div>
  <div class="by">
    <label class="lbl" for="activity-by">By</label>
    <select id="activity-by" class="ui-input small" value={actor} onchange={(e) => set({ by: e.currentTarget.value || null })}>
      <option value="">Anyone</option>
      {#each people as p (p.id)}<option value={p.id}>{clean(p.displayName)}{p.id === accountId ? '' : ' (editor)'}</option>{/each}
      {#if actor && !people.some((p) => p.id === actor)}<option value={actor}>Someone else</option>{/if}
    </select>
  </div>
  <TimeframePicker value={frame} shown={items.map((i) => i.at)} onchange={setFrame} />
</div>

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
  .by { display: flex; align-items: center; gap: 8px; }
  .lbl { font-size: var(--t-fine); color: var(--muted); }
  .by select { min-height: 34px; height: 34px; padding-block: 0; font-size: var(--t-fine); width: auto; max-width: 220px; }
  .foot { padding: 20px 0 0; }
  @media (max-width: 640px) {
    .bar { gap: 10px; }
    /* Five groups don't fit a phone: they scroll sideways, edge to edge, without a bar. */
    .groups { width: calc(100% + 2 * var(--page-pad)); margin-inline: calc(-1 * var(--page-pad)); padding-inline: var(--page-pad);
      overflow-x: auto; scrollbar-width: none; overscroll-behavior-x: contain; }
    .groups::-webkit-scrollbar { display: none; }
    .groups :global([role='radiogroup']) { width: max-content; }
  }
</style>
