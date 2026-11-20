<script lang="ts">
  // The emotes of a set, a page at a time (a set can hold a thousand): the first 120, Load more
  // for the next, and Filter asks the API, so it looks through the whole set and not just what
  // is drawn. Rename and remove are on every tile.
  import { onDestroy, tick, untrack } from 'svelte';
  import Capacity from '$lib/components/Capacity.svelte';
  import { ApiError, gql } from '$lib/api';
  import type { SetEmotesQuery } from '$lib/gql/graphql';
  import { REMOVE_SET_EMOTE, SET_EMOTES, UPDATE_SET_EMOTE } from '$lib/queries';
  import { nameProblem } from '$lib/rules';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import type { CardEmote } from '$lib/components/EmoteTile.svelte';
  import { clean, iso } from '$lib/format';
  import State from '$lib/components/State.svelte';
  import { session } from './session.svelte';
  import { STATE_LABEL } from './state';
  import { fail, notify } from './notices.svelte';
  import { SET_PAGE, appendPage, pagesAfter, pagesFor, type MemberChange } from './set-paging';

  type Member = SetEmotesQuery['setEmotes']['items'][number];
  type Props = {
    setId: string;
    capacity: number;
    used: number;
    /** Bumped when emotes were added from outside: the pages are read again. */
    rev?: number;
    /** The set changed here; `change` says how, so the page can follow without reading it all again. */
    onchange: (change?: MemberChange) => unknown;
  };
  let { setId, capacity, used, rev = 0, onchange }: Props = $props();
  let gridEl = $state<HTMLElement>();
  let heading = $state<HTMLElement>();
  /** Emotes with a remove on its way: one request per row, however often it is pressed. */
  let removing = $state(new Set<string>());

  let items = $state<Member[]>([]);
  /** The last page read, and how many emotes were removed since (they shift the pages after them up). */
  let pageNo = 1;
  let removedSince = 0;
  let total = $state(0);
  /** The first page is on its way (or the filter changed). */
  let loading = $state(true);
  let more = $state(false);
  let problem = $state('');
  let moreProblem = $state('');
  let filter = $state('');
  /** What the list was read with: the filter, a moment after typing stops. */
  let asked = $state('');
  let editing = $state<string | null>(null);
  let alias = $state('');
  let zeroWidth = $state(false);
  /** The rename is on its way: a second press sends nothing. */
  let saving = $state(false);
  /** Why the rename failed, in the dialog (a toast would sit behind it). */
  let failure = $state('');
  let dialog: HTMLDialogElement;
  const aliasError = $derived(nameProblem(alias));

  let token = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(timer));

  /** Reads the list from the first page: one page, or as many as are on screen when `keep`. */
  async function read(keep: boolean) {
    const mine = ++token;
    const pages = keep ? pagesFor(items.length) : 1;
    problem = '';
    if (!keep) loading = true;
    try {
      const got = await Promise.all(
        Array.from({ length: pages }, (_, i) => gql(SET_EMOTES, { id: setId, query: asked || null, page: i + 1, perPage: SET_PAGE })),
      );
      if (mine !== token) return;
      items = got.flatMap((r) => r.setEmotes.items);
      pageNo = pages;
      removedSince = 0;
      total = got[0].setEmotes.total;
    } catch (err) {
      if (mine !== token) return;
      problem = err instanceof ApiError && err.code === 'rate_limited' ? err.message : 'The emotes didn’t load. Check your connection and try again.';
    } finally {
      if (mine === token) loading = false;
    }
  }

  async function loadMore() {
    if (more) return;
    const mine = token;
    more = true;
    moreProblem = '';
    try {
      const asking = pagesAfter(pageNo, removedSince);
      const got = await Promise.all(asking.map((n) => gql(SET_EMOTES, { id: setId, query: asked || null, page: n, perPage: SET_PAGE })));
      if (mine !== token) return;
      for (const r of got) items = appendPage(items, r.setEmotes.items);
      total = got[got.length - 1].setEmotes.total;
      pageNo += 1;
      removedSince = 0;
    } catch {
      moreProblem = 'That page didn’t load.';
    } finally {
      more = false;
    }
  }

  // The filter asks the API a moment after typing stops.
  $effect(() => {
    const q = filter.trim();
    untrack(() => {
      clearTimeout(timer);
      if (q === asked) return;
      timer = setTimeout(() => {
        asked = q;
        void read(false);
      }, 250);
    });
  });
  // The first read, and a read again when emotes were added elsewhere (the pages on screen stay).
  let seen = -1;
  $effect(() => {
    void setId;
    const r = rev;
    untrack(() => {
      if (seen === r) return;
      const again = seen !== -1;
      seen = r;
      void read(again);
    });
  });

  const byId = $derived(new Map(items.map((m) => [m.emote.id, m])));
  const aliasFor = $derived(clean(byId.get(editing ?? '')?.emote.name ?? ''));
  /** An emote as this set shows it: its set name when renamed, zero-width as set here. */
  const tiles = $derived<CardEmote[]>(
    items.map((m) => ({ ...m.emote, shownName: m.name !== m.emote.name ? m.name : undefined, flags: { ...m.emote.flags, zeroWidth: m.zeroWidth } })),
  );
  const slots = $derived(Math.min(100, (used / Math.max(1, capacity)) * 100));

  function edit(m: Member) {
    editing = m.emote.id;
    alias = m.name;
    zeroWidth = m.zeroWidth;
    failure = '';
    dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (!editing || aliasError || saving) return;
    saving = true;
    failure = '';
    try {
      await gql(UPDATE_SET_EMOTE, { setId, emoteId: editing, input: { name: alias.trim(), zeroWidth } });
      const id = editing;
      const from = items.find((m) => m.emote.id === id)?.name ?? '';
      items = items.map((m) => (m.emote.id === id ? { ...m, name: alias.trim(), zeroWidth } : m));
      dialog.close();
      notify('Saved.');
      void onchange({ kind: 'rename', id, from, to: alias.trim() });
    } catch (err) {
      failure = err instanceof ApiError && err.message ? err.message : 'That didn’t work. Try again.';
    } finally {
      saving = false;
    }
  }

  async function remove(m: Member) {
    const id = m.emote.id;
    if (removing.has(id)) return;
    removing = new Set([...removing, id]);
    // Focus goes to the row that takes this one's place (the next, or the last), not the page.
    const at = tiles.findIndex((t) => t.id === id);
    try {
      await gql(REMOVE_SET_EMOTE, { setId, emoteId: id });
      notify(`${iso(m.name)} removed.`);
      items = items.filter((x) => x.emote.id !== id);
      total = Math.max(0, total - 1);
      removedSince += 1;
      await onchange({ kind: 'remove', id, name: m.name });
      await tick();
      const rows = gridEl?.querySelectorAll<HTMLElement>('.below .danger');
      const next = rows?.length ? rows[Math.min(at, rows.length - 1)] : null;
      (next ?? heading)?.focus();
    } catch (err) {
      fail(err);
    } finally {
      const left = new Set(removing);
      left.delete(id);
      removing = left;
    }
  }
</script>

<section class="ui-panel">
  <header>
    <h2 bind:this={heading} tabindex="-1">In this set</h2>
    <input class="ui-input filter" type="search" bind:value={filter} placeholder="Filter…" aria-label="Filter the set’s emotes" />
  </header>
  <div class="meter">
    <span class="bar"><span style:width="{slots}%"></span></span>
    <span class="ui-hint ui-num"><Capacity {used} {capacity} /></span>
  </div>
  {#if loading && items.length === 0}
    <State bare kind="loading" title="Loading the emotes…" />
  {:else if problem}
    <State bare kind="error" title="Couldn’t load the emotes">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={() => read(false)}>Try again</button>{/snippet}
    </State>
  {:else if items.length === 0 && !asked}
    <State bare title="Nothing in this set yet">Add emotes from the panel below.</State>
  {:else if items.length === 0}
    <State bare title="No match">No emote here matches “{asked}”.</State>
  {:else}
    <div class="grid" bind:this={gridEl}>
      <EmoteGrid
        items={tiles}
        label="Emotes in this set"
        eager={12}
        href={(e) => (byId.get(e.id)?.emote.owner?.id === session.me?.id ? `/me/emotes/${e.id}` : `/emotes/${e.id}`)}
        note={(e) => {
          const st = byId.get(e.id)?.emote.state;
          return st && st !== 'APPROVED' ? STATE_LABEL[st].toLowerCase() : undefined;
        }}
      >
        {#snippet below(e)}
          {@const m = byId.get(e.id)}
          {#if m}
            <button class="ui-icon" type="button" aria-label="Rename {iso(m.name)}" title="Rename" onclick={() => edit(m)}>
              <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4ZM14 6l4 4" /></svg>
            </button>
            <button class="ui-icon danger" type="button" aria-label="Remove {iso(m.name)}" title="Remove"
              aria-disabled={removing.has(m.emote.id) ? 'true' : undefined} onclick={() => remove(m)}>
              <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M5 7h14M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg>
            </button>
          {/if}
        {/snippet}
      </EmoteGrid>
      <div class="foot"><LoadMore hasMore={items.length < total} busy={more} error={moreProblem} {total} onmore={loadMore} /></div>
    </div>
  {/if}
</section>

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="alias-title">
  <form onsubmit={save}>
    <div class="body">
      <h2 id="alias-title">Rename {#if aliasFor}<bdi>{aliasFor}</bdi>{:else}the emote{/if} in this set</h2>
      <label class="ui-field">
        <span class="ui-label">Name here</span>
        <input class="ui-input" bind:value={alias} maxlength="100" spellcheck="false" aria-invalid={aliasError ? 'true' : undefined} />
        {#if aliasError}<span class="ui-error">{aliasError}</span>{:else if failure}<span class="ui-error" role="alert">{failure}</span>{:else}<span class="ui-hint">Only this set; the emote keeps its own name.</span>{/if}
      </label>
      <label class="ui-row">
        <div><strong>Zero-width</strong><span class="ui-hint">Drawn over the emote before it.</span></div>
        <input class="ui-switch" type="checkbox" bind:checked={zeroWidth} />
      </label>
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={!!aliasError || saving}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  .filter { width: min(240px, 50%); }
  .meter { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-bottom: 1px solid var(--border); }
  .bar { flex: 1; height: 4px; border-radius: 2px; background: var(--border-strong); overflow: hidden; }
  .bar > span { display: block; height: 100%; background: var(--accent); border-radius: 2px; }
  .grid { padding: 16px; }
  .foot { margin-top: 16px; }
  @media (hover: hover) and (pointer: fine) { .danger:hover { color: var(--red); } }
  .danger[aria-disabled='true'] { opacity: .5; cursor: progress; }
  .ui-row .ui-hint { display: block; }
</style>
