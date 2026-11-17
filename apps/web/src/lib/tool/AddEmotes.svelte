<script lang="ts">
  // Find emotes to add: public search, or your own uploads (pending ones too, except for a
  // personal set, which takes approved emotes only).
  import { gql } from '$lib/api';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { ADD_SET_EMOTE, MY_EMOTES, SEARCH } from '$lib/queries';
  import Thumb from './Thumb.svelte';
  import Name from '$lib/components/Name.svelte';
  import { iso } from '$lib/format';
  import { STATE_LABEL } from './state';
  import { fail, notify } from './notices.svelte';

  let {
    setId,
    personal,
    taken,
    names,
    full,
    onadded,
  }: { setId: string; personal: boolean; taken: Set<string>; names: Set<string>; full: boolean; onadded: () => void } = $props();

  let source = $state<'search' | 'mine'>('search');
  let query = $state('');
  let results = $state<EmoteFieldsFragment[] | null>(null);
  let busy = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let token = 0;

  async function run() {
    const mine = token + 1;
    token = mine;
    try {
      const items =
        source === 'search'
          ? (await gql(SEARCH, { query: query.trim(), page: 1, perPage: 30 })).searchEmotes.items
          : (await gql(MY_EMOTES, { page: 1, perPage: 100 })).myEmotes.items.filter((e) => {
              const q = query.trim().toLowerCase();
              return (e.state === 'APPROVED' || (e.state === 'PENDING' && !personal)) && (!q || e.name.toLowerCase().includes(q));
            });
      if (mine === token) results = items;
    } catch (err) {
      if (mine === token) fail(err);
    }
  }

  $effect(() => {
    void source;
    void query;
    clearTimeout(timer);
    timer = setTimeout(run, 250);
    return () => clearTimeout(timer);
  });

  async function add(e: EmoteFieldsFragment) {
    busy = e.id;
    try {
      await gql(ADD_SET_EMOTE, { setId, emoteId: e.id });
      notify(`${iso(e.name)} added.`);
      onadded();
    } catch (err) {
      fail(err);
    } finally {
      busy = null;
    }
  }
</script>

<section class="ui-panel">
  <header>
    <h2>Add emotes</h2>
    <div class="ui-seg" role="group" aria-label="Where from">
      <button type="button" aria-pressed={source === 'search'} onclick={() => (source = 'search')}>Search</button>
      <button type="button" aria-pressed={source === 'mine'} onclick={() => (source = 'mine')}>Your uploads</button>
    </div>
  </header>
  <div class="find">
    <input class="ui-input" type="search" bind:value={query} placeholder={source === 'search' ? 'Search by name or tag…' : 'Filter your uploads…'} aria-label="Find emotes" />
    {#if full}<p class="ui-error">This set is full. Remove something first.</p>{/if}
  </div>
  {#if results === null}
    <div class="ui-empty" aria-busy="true"><span class="ui-spinner" aria-hidden="true"></span></div>
  {:else if results.length === 0}
    <div class="ui-empty"><p>{source === 'search' ? 'Nothing found.' : 'No uploads to add.'}</p></div>
  {:else}
    <ul class="grid">
      {#each results as e (e.id)}
        {@const inSet = taken.has(e.id)}
        {@const clash = !inSet && names.has(e.name)}
        <li>
          <Thumb emote={e} name={e.name} box={40} />
          <span class="name" title={e.name}><Name value={e.name} /></span>
          {#if e.state !== 'APPROVED'}<span class="ui-chip {e.state.toLowerCase()}">{STATE_LABEL[e.state]}</span>{/if}
          <button class="ui-btn small {inSet ? 'ghost' : 'secondary'}" type="button" disabled={inSet || clash || full || busy === e.id}
            title={clash ? 'Another emote here has this name. Rename that one first.' : undefined} onclick={() => add(e)}>
            {inSet ? 'Added' : clash ? 'Name taken' : 'Add'}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .find { display: grid; gap: 8px; padding: 10px; border-bottom: 1px solid var(--border); }
  .find .ui-input { background: var(--bg); }
  .grid { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(136px, 1fr)); gap: 1px; background: var(--border); }
  .grid li { display: grid; justify-items: center; align-content: start; gap: 6px; padding: 12px 8px; background: var(--surface); }
  .name { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; font-size: .82rem; }
</style>
