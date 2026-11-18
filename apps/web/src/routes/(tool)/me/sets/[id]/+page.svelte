<script lang="ts">
  import { MediaQuery } from 'svelte/reactivity';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import { problemTitle } from '$lib/components/problem';
  import { clean, count, iso } from '$lib/format';
  import { sectionHref } from '$lib/profile/rows';
  import Name from '$lib/components/Name.svelte';
  import type { EditSetQuery, EditorPermission } from '$lib/gql/graphql';
  import { EDIT_SET, DELETE_SET, REMOVE_SET_EDITOR, UPDATE_SET } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import EditorsPanel from '$lib/tool/EditorsPanel.svelte';
  import SetEmotes from '$lib/tool/SetEmotes.svelte';
  import AddEmotes from '$lib/tool/AddEmotes.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { grantOn, permissionLabels } from '$lib/tool/editors';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  type EditableSet = NonNullable<EditSetQuery['set']>;
  let set = $state<EditableSet | null>(null);
  let editors = $state<EditSetQuery['setEditors']>([]);
  let grants = $state<EditSetQuery['editorOf']>([]);
  let name = $state('');
  let saving = $state(false);
  let deleting = $state(false);

  const narrow = new MediaQuery('max-width: 1000px');
  const id = $derived(page.params.id ?? '');
  const owner = $derived(set?.owner?.id === session.me?.id);
  // What you may do here: as the owner, a per-set editor, or the owner's account editor (ADR 0014).
  const held = $derived(owner ? new Set<EditorPermission>() : grantOn(grants, set?.owner?.id, session.me?.id));
  const setEditor = $derived(editors.some((e) => e.id === session.me?.id));
  const canRename = $derived(owner || setEditor || held.has('SETS'));
  const canDelete = $derived(owner || held.has('SETS'));
  const canManageEditors = $derived(owner || (held.has('EDITORS') && held.has('EMOTES')));

  const L = loader(async ({ fetch }) => {
    const want = page.params.id ?? '';
    let r: EditSetQuery;
    try {
      r = await gql(EDIT_SET, { id: want }, fetch);
    } catch (e) {
      if (e instanceof ApiError && e.code === 'forbidden') throw new ApiError('Only the set’s owner and editors can change it.', 'forbidden');
      throw e;
    }
    if (!r.set) throw new ApiError('This set doesn’t exist.', 'not_found');
    if (want === page.params.id) {
      set = r.set;
      editors = r.setEditors;
      grants = r.editorOf;
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
      notify(`${iso(set.name)} deleted.`);
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
      notify(`You no longer edit ${iso(set.name)}.`);
      await goto('/me/sets');
    } catch (err) {
      fail(err);
    }
  }

  const taken = $derived(new Set(set?.emotes.map((m) => m.emote.id) ?? []));
  const names = $derived(new Set(set?.emotes.map((m) => m.name) ?? []));
  const full = $derived(set ? set.emotes.length >= set.capacity : false);
</script>

<svelte:head><title>{L.cause ? problemTitle(L.cause, true, 'No set here') : `${set?.name ?? 'Set'} - cyex.tv`}</title></svelte:head>

{#snippet editorsBlock()}
  {#if set}<EditorsPanel setId={set.id} owner={canManageEditors} bind:editors />{/if}
{/snippet}

{#snippet dangerBlock()}
  {#if set}
    <section class="ui-card">
      {#if canDelete}
        <h2>Delete set</h2>
        <p class="ui-hint">The emotes stay; only this set goes. A channel’s active set can’t be deleted: pick another one first.</p>
        <div class="ui-actions gap"><Confirm label="Delete set" confirm="Click again to delete" busy={deleting} onconfirm={remove} /></div>
      {:else if setEditor}
        <h2>Stop editing</h2>
        <p class="ui-hint">You lose access to this set until the owner adds you again.</p>
        <div class="ui-actions gap"><Confirm label="Leave" confirm="Click again to leave" onconfirm={leave} /></div>
      {/if}
    </section>
  {/if}
{/snippet}

<LoadGate
  source={L}
  label="Loading the set…"
  level={1}
  home={{ href: sectionHref(session.me?.id ?? '', 'sets'), label: 'Your sets' }}
  missing={{ title: 'No set here.', body: 'It may have been deleted, or the address is off.' }}
>
{#snippet children()}
{#if set}
  <PageHead
    name
    title={clean(set.name)}
    back={set.kind === 'GLOBAL' ? { href: '/admin', label: 'Admin' } : { href: '/me/sets', label: 'Sets' }}
  >
    {#snippet line()}
      {set!.kind === 'GLOBAL' ? 'The global set' : set!.kind === 'PERSONAL' ? 'Personal set' : 'Channel set'} ·
      <span>{count(set!.emotes.length)} / {count(set!.capacity)}</span> emotes
      {#if !owner && set!.owner}· by <Name value={set!.owner.displayName} />{/if}
      {#if held.size}· you edit this account: {permissionLabels([...held])}{/if}
    {/snippet}
  </PageHead>

  <div class="layout">
    <div class="ui-stack main">
      <SetEmotes {set} onchange={load} />
      <AddEmotes setId={set.id} personal={set.kind === 'PERSONAL'} {taken} {names} {full} onadded={load} />
    </div>
    <aside class="ui-stack">
      {#if canRename}
      <form class="ui-card ui-form" onsubmit={rename}>
        <h2><label for="set-name">Name</label></h2>
        <div class="row">
          <input id="set-name" class="ui-input" bind:value={name} maxlength="100" />
          <button class="ui-btn secondary" type="submit" disabled={saving || !name.trim() || name.trim() === set.name}>Rename</button>
        </div>
      </form>
      {/if}

      {#if set.kind === 'CHANNEL'}
        {#if narrow.current}
          <details class="fold"><summary>Editors</summary>{@render editorsBlock()}</details>
        {:else}
          {@render editorsBlock()}
        {/if}
      {/if}

      {#if set.kind !== 'GLOBAL' && (canDelete || setEditor)}
        {#if narrow.current}
          <details class="fold"><summary>{canDelete ? 'Delete set' : 'Stop editing'}</summary>{@render dangerBlock()}</details>
        {:else}
          {@render dangerBlock()}
        {/if}
      {/if}
    </aside>
  </div>
{/if}
{/snippet}
</LoadGate>

<style>
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 16px; align-items: start; }
  .row { display: flex; gap: 8px; }
  .row input { min-width: 0; }
  .gap { margin-top: 12px; }
  .fold > summary { cursor: pointer; padding: 10px 14px; font-weight: 600; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .fold[open] > summary { margin-bottom: 8px; }
  @media (max-width: 1000px) { .layout { grid-template-columns: minmax(0, 1fr); } aside { order: -1; } }
</style>
