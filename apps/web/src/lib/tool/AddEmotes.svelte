<script lang="ts">
  // Find emotes to add: one plain search field, searched when you press Enter (never while you
  // type). Results are your favourites whose name holds the text first (marked with a heart), then
  // your own matching uploads (pending ones too, except for a personal set, which takes approved
  // emotes only), then everyone's approved emotes without the ones you hid for yourself, in a list of its own
  // height with an Add button on each.
  import { tick } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { ADD_SET_EMOTE, MY_EMOTES, MY_FAVOURITES, SEARCH } from '$lib/queries';
  import { favouritesFirst, personal as lists } from '$lib/personal/lists.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean, count, iso } from '$lib/format';
  import State from '$lib/components/State.svelte';
  import Thumb from './Thumb.svelte';
  import { session } from './session.svelte';
  import { STATE_LABEL } from './state';
  import { fail, notify } from './notices.svelte';
  import { nameProblem } from '$lib/rules';
  import { RESULT_PAGE, appendResults, freeName, mergeResults, showingLine } from './add-emotes';

  let {
    setId,
    personal,
    taken,
    names,
    full,
    onadded,
  }: { setId: string; personal: boolean; taken: Set<string>; names: Set<string>; full: boolean; onadded: (added: { id: string; name: string }) => unknown } = $props();
  let listEl = $state<HTMLElement>();
  let findEl = $state<HTMLInputElement>();

  let query = $state('');
  let searched = $state('');
  let results = $state<EmoteFieldsFragment[] | null>(null);
  let searching = $state(false);
  /** How many public results the search found in all, and how many of them are in the list. */
  let found = $state(0);
  let fetched = $state(0);
  let nextPage = $state(2);
  let moreBusy = $state(false);
  let moreProblem = $state('');
  let problem = $state<string | null>(null);
  let busy = $state<string | null>(null);
  /** The emote whose name is taken here, being added under another name. */
  let renaming = $state<string | null>(null);
  let alias = $state('');
  const aliasProblem = $derived(nameProblem(alias) ?? (names.has(alias.trim()) ? 'Another emote here has that name.' : null));

  function askName(e: EmoteFieldsFragment) {
    if (renaming === e.id) {
      renaming = null;
      return;
    }
    renaming = e.id;
    alias = freeName(e.name, names);
  }
  let token = 0;

  async function search(e?: SubmitEvent) {
    e?.preventDefault();
    const text = query.trim();
    if (!text) {
      // An empty field ends the search in flight too: its answer must not fill the list again.
      token++;
      moreBusy = false;
      searching = false;
      results = null;
      problem = null;
      return;
    }
    const mine = ++token;
    moreBusy = false; // a page of the old search no longer holds the button
    searching = true;
    problem = null;
    try {
      lists.ensure();
      const [everyone, own, favs] = await Promise.all([
        gql(SEARCH, { query: text, page: 1, perPage: RESULT_PAGE }),
        session.me ? gql(MY_EMOTES, { page: 1, perPage: 100 }) : Promise.resolve(null),
        // Favourites are a bonus: a failure there never fails the search.
        session.me ? gql(MY_FAVOURITES, { query: text, perPage: 20 }).catch(() => null) : Promise.resolve(null),
      ]);
      if (mine !== token) return;
      const fav = (favs?.myFavourites?.items ?? []).filter((e) => e.state === 'APPROVED' || (e.state === 'PENDING' && !personal));
      const rest = mergeResults(text, own?.myEmotes.items ?? [], lists.visible(everyone.searchEmotes.items), personal);
      results = favouritesFirst(fav, rest, Infinity);
      searched = text;
      found = everyone.searchEmotes.total;
      fetched = everyone.searchEmotes.items.length;
      nextPage = 2;
      moreProblem = '';
    } catch (err) {
      if (mine !== token) return;
      results = null;
      problem = err instanceof ApiError && err.code === 'rate_limited' ? err.message : 'Searching didn’t work. Check your connection and try again.';
    } finally {
      if (mine === token) searching = false;
    }
  }

  /** The next page of public results, after the ones on screen. */
  async function loadMore() {
    if (moreBusy || !results) return;
    const mine = token;
    moreBusy = true;
    moreProblem = '';
    try {
      const r = await gql(SEARCH, { query: searched, page: nextPage, perPage: RESULT_PAGE });
      if (mine !== token || !results) return;
      results = appendResults(results, lists.visible(r.searchEmotes.items));
      fetched += r.searchEmotes.items.length;
      found = r.searchEmotes.total;
      nextPage += 1;
    } catch (err) {
      if (mine !== token) return;
      moreProblem = err instanceof ApiError && err.code === 'rate_limited' ? err.message : 'That page didn’t load. Try again.';
    } finally {
      if (mine === token) moreBusy = false;
    }
  }

  async function add(e: EmoteFieldsFragment, as?: string) {
    if (busy) return;
    busy = e.id;
    // The pressed button turns into "Added" (disabled): focus moves on to the next Add.
    const at = (results ?? []).findIndex((r) => r.id === e.id);
    try {
      await gql(ADD_SET_EMOTE, { setId, emoteId: e.id, input: as ? { name: as } : null });
      notify(`${iso(as ?? e.name)} added.`);
      renaming = null;
      await onadded({ id: e.id, name: as ?? e.name });
      busy = null;
      await tick();
      const buttons = [...(listEl?.querySelectorAll<HTMLButtonElement>('button.add') ?? [])];
      const next = buttons.slice(at + 1).find((b) => !b.disabled) ?? buttons.slice(0, at).reverse().find((b) => !b.disabled);
      (next ?? findEl)?.focus();
    } catch (err) {
      fail(err);
    } finally {
      busy = null;
    }
  }
</script>

<section class="ui-panel">
  <header><h2><label for="add-find">Add emotes</label></h2></header>
  <form class="find" role="search" onsubmit={search}>
    <div class="field">
      <input id="add-find" class="ui-input" type="search" bind:this={findEl} bind:value={query} placeholder="Search emotes by name or tag"
        autocomplete="off" spellcheck="false" enterkeyhint="search" aria-describedby="add-hint" />
      <button class="ui-btn secondary" type="submit" disabled={searching}><Icon name="search" size={15} />{searching ? 'Searching…' : 'Search'}</button>
    </div>
    <p class="ui-hint" id="add-hint">Press Enter to search. Your own uploads come first{personal ? '' : ', pending ones too'}.</p>
    {#if full}<p class="ui-error">This set is full. Remove something first.</p>{/if}
  </form>
  {#if problem}
    <State bare kind="error" title="Couldn’t search">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={() => search()}>Try again</button>{/snippet}
    </State>
  {:else if results && results.length === 0}
    <State bare title="Nothing found">No emotes match {searched}. Try another name or tag.</State>
  {:else if results}
    <ul class="ui-list results" bind:this={listEl} aria-label="Emotes found for {searched}">
      {#each results as e (e.id)}
        {@const inSet = taken.has(e.id)}
        {@const clash = !inSet && names.has(e.name)}
        {@const mine = e.owner?.id === session.me?.id}
        <li>
          <Thumb emote={e} name="" box={40} />
          <a class="who" href={mine ? `/me/emotes/${e.id}` : `/emotes/${e.id}`}>
            <span class="name"><bdi>{e.name}</bdi>{#if lists.favourites.has(e.id)}<span class="fav" title="Favourite"><Icon name="heart" size={12} /><span class="sr-only">, favourite</span></span>{/if}</span>
            <span class="ui-hint">
              {#if mine}Yours{:else if e.owner}by <bdi>{clean(e.owner.displayName)}</bdi>{/if}{#if e.state !== 'APPROVED'} · {STATE_LABEL[e.state].toLowerCase()}{/if}
            </span>
          </a>
          {#if clash}
            <button class="ui-btn small add secondary" type="button" disabled={full || busy === e.id} aria-expanded={renaming === e.id}
              aria-label="Name taken: add {iso(e.name)} under another name" onclick={() => askName(e)}>Add as…</button>
          {:else}
            <button class="ui-btn small add {inSet ? 'ghost' : 'secondary'}" type="button" disabled={inSet || full || busy === e.id}
              aria-label="{inSet ? 'Added' : 'Add'} {iso(e.name)}" onclick={() => add(e)}>
              {inSet ? 'Added' : busy === e.id ? 'Adding…' : 'Add'}
            </button>
          {/if}
          {#if clash && renaming === e.id}
            <form class="alias" onsubmit={(ev) => { ev.preventDefault(); if (!aliasProblem) add(e, alias.trim()); }}>
              <p class="ui-hint">Another emote in this set is called <bdi>{iso(e.name)}</bdi>. Add this one as:</p>
              <div class="alias-row">
                <input class="ui-input" bind:value={alias} maxlength="100" spellcheck="false" aria-label="Name in this set"
                  aria-invalid={aliasProblem ? 'true' : undefined} />
                <button class="ui-btn small primary" type="submit" disabled={!!aliasProblem || busy === e.id}>{busy === e.id ? 'Adding…' : 'Add'}</button>
              </div>
              {#if aliasProblem}<span class="ui-error" role="alert">{aliasProblem}</span>{/if}
            </form>
          {/if}
        </li>
      {/each}
    </ul>
    {#if found > fetched || moreProblem}
      <div class="more">
        <p class="ui-hint" role="status">{moreProblem || showingLine(fetched, found, count)}</p>
        <button class="ui-btn secondary small" type="button" disabled={moreBusy} aria-busy={moreBusy} onclick={loadMore}>
          {moreBusy ? 'Loading…' : moreProblem ? 'Try again' : 'Load more'}
        </button>
      </div>
    {/if}
  {/if}
</section>

<style>
  .find { display: grid; gap: 8px; padding: 12px 16px; }
  .field { display: flex; gap: 8px; }
  .field input { flex: 1; }
  .field .ui-btn { flex: none; }
  .results { max-height: 360px; overflow-y: auto; overscroll-behavior: contain; border-top: 1px solid var(--border); }
  .results > li { padding: 6px 16px; flex-wrap: wrap; }
  .alias { flex-basis: 100%; display: grid; gap: 6px; padding: 4px 0 8px 52px; }
  .alias .ui-hint { margin: 0; }
  .alias-row { display: flex; gap: 8px; }
  .alias-row input { flex: 1; min-width: 0; }
  .who { display: grid; flex: 1; min-width: 0; line-height: 1.3; color: inherit; text-decoration: none; }
  .fav { display: inline-flex; margin-left: 6px; vertical-align: -1px; color: var(--accent-text); }
  .fav :global(svg) { fill: currentColor; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  @media (hover: hover) and (pointer: fine) { .who:hover .name { text-decoration: underline; } }
  .add { flex: none; }
  .more { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 16px; border-top: 1px solid var(--border); }
  .more p { margin: 0; }
</style>
