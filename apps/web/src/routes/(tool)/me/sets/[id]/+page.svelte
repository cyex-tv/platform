<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import { clean, count } from '$lib/format';
  import type { EditSetQuery } from '$lib/gql/graphql';
  import { EDIT_SET, DELETE_SET, REMOVE_SET_EDITOR, UPDATE_SET } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import EditorsPanel from '$lib/tool/EditorsPanel.svelte';
  import SetEmotes from '$lib/tool/SetEmotes.svelte';
  import AddEmotes from '$lib/tool/AddEmotes.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type EditableSet = NonNullable<EditSetQuery['set']>;
  let set = $state<EditableSet | null>(null);
  let editors = $state<EditSetQuery['setEditors']>([]);
  let name = $state('');
  let saving = $state(false);
  let deleting = $state(false);

  const id = $derived(page.params.id ?? '');
  const owner = $derived(set?.owner?.id === session.me?.id);

  const L = loader(async ({ fetch }) => {
    const want = page.params.id ?? '';
    let r: EditSetQuery;
    try {
      r = await gql(EDIT_SET, { id: want }, fetch);
    } catch (e) {
      if (e instanceof ApiError && e.code === 'forbidden') throw new Error('Only the set’s owner and editors can change it.');
      throw e;
    }
    if (!r.set) throw new Error('This set doesn’t exist.');
    if (want === page.params.id) {
      set = r.set;
      editors = r.setEditors;
      name = r.set.name;
    }
    return r.set;
  });
  const load = () => L.reload();

  async function rename(e: SubmitEvent) {
    e.preventDefault();
    if (saving || !set || !name.trim() || name.trim() === set.name) return;
    saving = true;
    try {
      const r = await gql(UPDATE_SET, { id: set.id, input: { name: name.trim() } });
      set.name = r.updateSet.name;
      notify('Renamed.');
    } catch (err) {
      fail(err);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!set) return;
    deleting = true;
    try {
      await gql(DELETE_SET, { id: set.id });
      notify(`${set.name} deleted.`);
      await goto('/me/sets');
    } catch (err) {
      fail(err);
      deleting = false;
    }
  }

  async function leave() {
    if (!set || !session.me) return;
    try {
      await gql(REMOVE_SET_EDITOR, { setId: set.id, accountId: session.me.id });
      notify(`You no longer edit ${set.name}.`);
      await goto('/me/sets');
    } catch (err) {
      fail(err);
    }
  }

  const taken = $derived(new Set(set?.emotes.map((m) => m.emote.id) ?? []));
  const names = $derived(new Set(set?.emotes.map((m) => m.name) ?? []));
  const full = $derived(set ? set.emotes.length >= set.capacity : false);
</script>

<svelte:head><title>{set?.name ?? 'Set'} - cyex.tv</title></svelte:head>

<LoadGate source={L} label="Loading the set…">
{#snippet children()}
{#if set}
  <p class="crumbs"><a href={set.kind === 'GLOBAL' ? '/admin' : '/me/sets'}>{set.kind === 'GLOBAL' ? 'Admin' : 'Sets'}</a> <span aria-hidden="true">/</span> {clean(set.name)}</p>
  <div class="ui-head">
    <div>
      <h1>{clean(set.name)}</h1>
      <p>
        {set.kind === 'GLOBAL' ? 'The global set' : set.kind === 'PERSONAL' ? 'Personal set' : 'Channel set'} ·
        <span class="ui-mono">{count(set.emotes.length)} / {count(set.capacity)}</span> emotes
        {#if !owner && set.owner}· by {set.owner.displayName}{/if}
      </p>
    </div>
  </div>

  <div class="layout">
    <div class="ui-stack">
      <SetEmotes {set} onchange={load} />
      <AddEmotes setId={set.id} personal={set.kind === 'PERSONAL'} {taken} {names} {full} onadded={load} />
    </div>
    <aside class="ui-stack">
      <form class="ui-card ui-form" onsubmit={rename}>
        <h2>Name</h2>
        <div class="row">
          <input class="ui-input" bind:value={name} maxlength="100" aria-label="Set name" />
          <button class="ui-btn secondary" type="submit" disabled={saving || !name.trim() || name.trim() === set.name}>Rename</button>
        </div>
      </form>

      {#if set.kind === 'CHANNEL'}
        <EditorsPanel setId={set.id} {owner} bind:editors />
      {/if}

      {#if set.kind !== 'GLOBAL'}
      <section class="ui-card">
        {#if owner}
          <h2>Delete set</h2>
          <p class="ui-hint">The emotes stay; only this set goes. A channel’s active set can’t be deleted: pick another one first.</p>
          <div class="ui-actions gap"><Confirm label="Delete set" confirm="Click again to delete" busy={deleting} onconfirm={remove} /></div>
        {:else}
          <h2>Stop editing</h2>
          <p class="ui-hint">You lose access to this set until the owner adds you again.</p>
          <div class="ui-actions gap"><Confirm label="Leave" confirm="Click again to leave" onconfirm={leave} /></div>
        {/if}
      </section>
      {/if}
    </aside>
  </div>
{/if}
{/snippet}
</LoadGate>

<style>
  .crumbs { margin-bottom: 16px; min-width: 0; overflow-wrap: anywhere; color: var(--dim); font-size: var(--text-sub); }
  .crumbs a { color: var(--muted); text-decoration: none; }
  @media (hover: hover) and (pointer: fine) { .crumbs a:hover { color: var(--text); } }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 16px; align-items: start; }
  .row { display: flex; gap: 8px; }
  .row input { min-width: 0; }
  .gap { margin-top: 12px; }
  @media (max-width: 1000px) { .layout { grid-template-columns: minmax(0, 1fr); } }
</style>
