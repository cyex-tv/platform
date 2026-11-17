<script lang="ts">
  // Find emotes to add: public search, or your own uploads (pending ones too, except for a
  // personal set, which takes approved emotes only).
  import { gql } from '$lib/api';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { ADD_SET_EMOTE, MY_EMOTES, SEARCH } from '$lib/queries';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import { iso } from '$lib/format';
  import State from './State.svelte';
  import { session } from './session.svelte';
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
  const byId = $derived(new Map((results ?? []).map((r) => [r.id, r])));
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
    <State bare kind="loading" title="Loading…" />
  {:else if results.length === 0}
    <State bare title={source === 'search' ? 'Nothing found' : 'No uploads to add'}>
      {source === 'search' ? 'Try another name or tag.' : 'Pending and approved uploads you haven’t added yet show up here.'}
    </State>
  {:else}
    <div class="grid">
      <EmoteGrid
        items={results}
        density="compact"
        label="Results"
        href={(e) => (byId.get(e.id)?.owner?.id === session.me?.id ? `/me/emotes/${e.id}` : `/emotes/${e.id}`)}
        note={(e) => {
          const st = byId.get(e.id)?.state;
          return st && st !== 'APPROVED' ? STATE_LABEL[st].toLowerCase() : undefined;
        }}
      >
        {#snippet below(e)}
          {@const inSet = taken.has(e.id)}
          {@const clash = !inSet && names.has(e.name)}
          <button class="ui-btn small {inSet ? 'ghost' : 'secondary'}" type="button" disabled={inSet || clash || full || busy === e.id}
            title={clash ? 'Another emote here has this name. Rename that one first.' : undefined} aria-label="{inSet ? 'Added' : clash ? 'Name taken' : 'Add'} {iso(e.name)}" onclick={() => byId.get(e.id) && add(byId.get(e.id)!)}>
            {inSet ? 'Added' : clash ? 'Name taken' : 'Add'}
          </button>
        {/snippet}
      </EmoteGrid>
    </div>
  {/if}
</section>

<style>
  .find { display: grid; gap: 8px; padding: 12px 16px; border-bottom: 1px solid var(--border); }
  .grid { padding: 16px; }
</style>
