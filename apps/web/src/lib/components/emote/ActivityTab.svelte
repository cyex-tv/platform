<script lang="ts">
  // The emote page's Activity tab: who uploaded it and which sets added, renamed or removed it,
  // newest first, 30 to a page, each with its date and time. Only listed (public) emotes have
  // any, and the page doesn't mount this for the others. Pages are kept once loaded; the pager
  // sits in the tab bar, so this hands its state up (`pager`). A "By" filter (UserFilter: focused
  // empty, who has the most lines about this emote; typing, the people search) narrows it, kept in
  // the address as ?by=.
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { fetchWith, gql } from '$lib/api';
  import UserFilter from '$lib/components/UserFilter.svelte';
  import type { Person } from '$lib/components/user-filter';
  import { ACTIVITY_TOP_ACTORS } from '$lib/queries';
  import { SvelteMap } from 'svelte/reactivity';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { loadEmoteActivity, type ActivityItem } from '$lib/profile/activity';
  import EmoteActivityRows from './EmoteActivityRows.svelte';
  import { rangeLabel } from './emote-page';
  import { CursorTrail, type PagerState } from './pager';

  let { emoteId, name, owner = null, pager = $bindable(null) }: { emoteId: string; name: string; owner?: { id: string; displayName: string; avatar?: string | null } | null; pager?: PagerState | null } = $props();

  const by = $derived(page.url.searchParams.get('by') ?? '');
  function setBy(id: string) {
    const url = new URL(page.url);
    if (id) url.searchParams.set('by', id);
    else url.searchParams.delete('by');
    void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
  }
  async function topActors(signal: AbortSignal): Promise<Person[]> {
    return (await gql(ACTIVITY_TOP_ACTORS, { emoteId, limit: 5 }, fetchWith(signal))).activityTopActors;
  }
  const flagOf = (p: Person) => (owner && p.account.id === owner.id ? 'Uploader' : null);

  const pages = new SvelteMap<number, ActivityItem[]>();
  let trail = new CursorTrail();
  // Answers for an older "By" are dropped.
  let round = 0;
  let at = $state(0);
  /** Bumped when the trail learns a page, so `next` is read again. */
  let known = $state(0);
  let busy = $state(false);
  let failed = $state<number | null>(null);

  const rows = $derived(pages.get(at) ?? []);
  const offset = $derived([...pages.entries()].filter(([i]) => i < at).reduce((n, [, r]) => n + r.length, 0));

  async function go(to: number) {
    if (busy || to < 0) return;
    failed = null;
    if (!pages.has(to)) {
      const start = trail.start(to);
      if (start === undefined) return;
      busy = true;
      const mine = round;
      try {
        const got = await loadEmoteActivity(emoteId, start, fetch, by || null);
        if (mine !== round) return;
        pages.set(to, got.items);
        trail.after(to, got.next);
        known++;
      } catch {
        if (mine === round) failed = to;
        return;
      } finally {
        if (mine === round) busy = false;
      }
      if (mine !== round) return;
    }
    at = to;
  }
  // A new "By" starts the feed over.
  $effect(() => {
    void by;
    untrack(() => {
      round++;
      pages.clear();
      trail = new CursorTrail();
      at = 0;
      busy = false;
      failed = null;
      known++;
      void go(0);
    });
  });

  $effect(() => {
    void known;
    pager = {
      range: rows.length && (at > 0 || trail.hasNext(at)) ? rangeLabel(offset, rows.length) : '',
      prev: at > 0,
      next: trail.hasNext(at),
      busy,
      onprev: () => go(at - 1),
      onnext: () => go(at + 1),
    };
  });
</script>

<div class="bar" role="group" aria-label="Filter activity">
  <UserFilter id="emote-activity-by" label="By" value={by} onchange={setBy} top={topActors} topKey={emoteId} flag={flagOf}
    name={owner && by === owner.id ? owner.displayName : null} />
</div>

{#if failed !== null && !rows.length}
  <Problem cause="server" onretry={() => failed !== null && go(failed)} home={{ href: '/search', label: 'Browse emotes' }} />
{:else if !pages.has(at)}
  <State kind="loading" title="Loading activity…" />
{:else if !rows.length}
  {#if by}
    <State title="Nothing by them.">They haven’t added, renamed or removed this emote in a listed set.
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={() => setBy('')}>Show everyone</button>{/snippet}
    </State>
  {:else}
    <State title="No activity yet.">Sets adding, renaming or removing this emote show up here.</State>
  {/if}
{:else}
  <div class:busy><EmoteActivityRows items={rows} {name} /></div>
  {#if failed !== null}<p class="fine err" role="alert">That page didn’t load. <button class="link" type="button" onclick={() => failed !== null && go(failed)}>Try again</button></p>{/if}
{/if}

<style>
  p { margin: 0; }
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; margin: 0 0 12px; }
  div { transition: opacity var(--dur-color); }
  .busy { opacity: .6; }
  .err { padding-top: 12px; color: var(--muted); }
  .err .link { padding: 0; border: 0; background: none; font: inherit; color: var(--text); text-decoration: underline; cursor: pointer; }
</style>
