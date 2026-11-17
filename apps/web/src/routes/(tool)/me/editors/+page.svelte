<script lang="ts">
  // Account editors (ADR 0014): the people who work on your sets and channels, with what each one
  // may do, and the accounts you edit yourself. Adding is by Twitch or Kick login (they need to
  // have signed in to cyex.tv once).
  import { ApiError, gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, iso } from '$lib/format';
  import type { EditorPermission, EditorsQuery, Platform } from '$lib/gql/graphql';
  import { ADD_ACCOUNT_EDITOR, EDITORS, REMOVE_ACCOUNT_EDITOR, UPDATE_ACCOUNT_EDITOR } from '$lib/queries';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { PERMISSIONS, ordered, permissionLabels } from '$lib/tool/editors';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { session } from '$lib/tool/session.svelte';

  type Editor = EditorsQuery['accountEditors'][number];
  const MAX = 20;

  const L = loader(({ fetch }) => gql(EDITORS, {}, fetch));

  // Adding
  let platform = $state<Platform>('TWITCH');
  let login = $state('');
  let picked = $state<EditorPermission[]>(['EMOTES']);
  let adding = $state(false);
  let problem = $state<string | null>(null);

  // Changing one editor's permissions
  let dialog: HTMLDialogElement;
  let editing = $state<Editor | null>(null);
  let editPicked = $state<EditorPermission[]>([]);
  let saving = $state(false);
  let busyId = $state<string | null>(null);

  const toggle = (list: EditorPermission[], key: EditorPermission, on: boolean) =>
    on ? ordered([...list, key]) : list.filter((k) => k !== key);

  function replace(next: EditorsQuery) {
    L.set(next);
  }

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const name = login.trim().replace(/^@/, '');
    if (adding || !name || !picked.length || !L.data) return;
    adding = true;
    problem = null;
    try {
      const r = await gql(ADD_ACCOUNT_EDITOR, { input: { platform, login: name, permissions: picked } });
      const added = r.addAccountEditor;
      replace({ ...L.data, accountEditors: [...L.data.accountEditors, added] });
      notify(`${iso(added.account.displayName)} can now work on your account.`);
      login = '';
    } catch (err) {
      const code = err instanceof ApiError ? err.code : null;
      if (code === 'not_found') problem = `No one called ${name} has signed in to cyex.tv with ${platform === 'TWITCH' ? 'Twitch' : 'Kick'}.`;
      else if (code === 'already_owner') problem = 'That’s you. You can do all of this already.';
      else if (code === 'already_editor') problem = `${name} is an editor already. Change their permissions in the list.`;
      else fail(err);
    } finally {
      adding = false;
    }
  }

  function edit(ed: Editor) {
    editing = ed;
    editPicked = ordered(ed.permissions);
    dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (!editing || saving || !editPicked.length || !L.data) return;
    saving = true;
    try {
      const r = await gql(UPDATE_ACCOUNT_EDITOR, { accountId: editing.account.id, permissions: editPicked });
      const next = r.updateAccountEditor;
      replace({ ...L.data, accountEditors: L.data.accountEditors.map((x) => (x.account.id === next.account.id ? next : x)) });
      dialog.close();
      notify(`Saved. ${iso(next.account.displayName)} can now: ${permissionLabels(next.permissions)}.`);
    } catch (err) {
      fail(err);
    } finally {
      saving = false;
    }
  }

  async function remove(ed: Editor) {
    if (!L.data) return;
    busyId = ed.account.id;
    try {
      await gql(REMOVE_ACCOUNT_EDITOR, { accountId: ed.account.id });
      replace({ ...L.data, accountEditors: L.data.accountEditors.filter((x) => x.account.id !== ed.account.id) });
      notify(`${iso(ed.account.displayName)} no longer works on your account.`);
    } catch (err) {
      fail(err);
    } finally {
      busyId = null;
    }
  }

  async function leave(ownerId: string, name: string) {
    if (!L.data || !session.me) return;
    busyId = ownerId;
    try {
      await gql(REMOVE_ACCOUNT_EDITOR, { ownerId, accountId: session.me.id });
      replace({ ...L.data, editorOf: L.data.editorOf.filter((x) => x.account.id !== ownerId) });
      notify(`You no longer edit ${iso(name)}’s account.`);
    } catch (err) {
      fail(err);
    } finally {
      busyId = null;
    }
  }
</script>

<svelte:head><title>Editors - cyex.tv</title></svelte:head>

{#snippet checks(list: EditorPermission[], set: (next: EditorPermission[]) => void, prefix: string)}
  <fieldset class="perms">
    <legend class="ui-label">Can</legend>
    {#each PERMISSIONS as p (p.key)}
      <label class="perm">
        <input type="checkbox" checked={list.includes(p.key)} onchange={(e) => set(toggle(list, p.key, e.currentTarget.checked))}
          aria-describedby="{prefix}-{p.key}" />
        <span class="text">
          <strong>{p.label}</strong>
          <span class="ui-hint" id="{prefix}-{p.key}">{p.line}</span>
        </span>
      </label>
    {/each}
  </fieldset>
{/snippet}

<div class="narrow">
  <PageHead title="Editors" line="People who work on your sets and channels for you. You choose what each one can do, and can take it back at any time." />

  <LoadGate source={L} label="Loading your editors…">
  {#snippet children(data)}
  {@const editors = data.accountEditors}
  <div class="ui-stack">
    <section class="ui-panel">
      <header><h2>Your editors</h2><span class="ui-hint ui-num">{editors.length} / {MAX}</span></header>
      {#if editors.length}
        <ul class="ui-list">
          {#each editors as ed (ed.account.id)}
            <li>
              <Avatar src={ed.account.avatar} name={ed.account.displayName} size={32} tone="tool" />
              <span class="who">
                <strong><bdi>{clean(ed.account.displayName)}</bdi></strong>
                <span class="ui-hint">
                  {permissionLabels(ed.permissions)}{#if ed.grantedBy && ed.grantedBy.id !== session.me?.id} · added by <bdi>{clean(ed.grantedBy.displayName)}</bdi>{/if}
                </span>
              </span>
              <span class="ui-actions">
                <button class="ui-btn secondary small" type="button" onclick={() => edit(ed)}
                  aria-label="Change what {iso(ed.account.displayName)} can do">Change</button>
                <Confirm small label="Remove" confirm="Click again to remove" busy={busyId === ed.account.id} onconfirm={() => remove(ed)} />
              </span>
            </li>
          {/each}
        </ul>
      {:else}
        <State bare title="No editors yet">Add a moderator or manager below, and pick what they can do.</State>
      {/if}
    </section>

    <form class="ui-card add" onsubmit={add} novalidate>
      <h2>Add an editor</h2>
      {#if editors.length >= MAX}
        <p class="ui-hint">You have {MAX} editors, the most an account can have. Remove one to add someone else.</p>
      {:else}
        <div class="ui-field">
          <span class="ui-label" id="platform-label">Their login on</span>
          <div class="ui-seg" role="group" aria-labelledby="platform-label">
            <button type="button" aria-pressed={platform === 'TWITCH'} onclick={() => (platform = 'TWITCH')}>Twitch</button>
            <button type="button" aria-pressed={platform === 'KICK'} onclick={() => (platform = 'KICK')}>Kick</button>
          </div>
          <input class="ui-input" bind:value={login} placeholder="Login" aria-label="Their login" autocomplete="off" spellcheck="false"
            maxlength="64" aria-invalid={problem ? 'true' : undefined} aria-describedby={problem ? 'add-err' : undefined} />
          {#if problem}<span class="ui-error" id="add-err">{problem}</span>{/if}
        </div>
        {@render checks(picked, (next) => (picked = next), 'add')}
        {#if !picked.length}<p class="ui-hint">Pick at least one.</p>{/if}
        <div class="ui-actions">
          <button class="ui-btn primary" type="submit" disabled={adding || !login.trim() || !picked.length}>
            <Icon name="plus" size={15} />{adding ? 'Adding…' : 'Add editor'}
          </button>
        </div>
      {/if}
    </form>

    <section class="ui-panel">
      <header><h2>You edit</h2><span class="ui-hint ui-num">{data.editorOf.length}</span></header>
      {#if data.editorOf.length}
        <ul class="ui-list">
          {#each data.editorOf as acc (acc.account.id)}
            <li>
              <Avatar src={acc.account.avatar} name={acc.account.displayName} size={32} tone="tool" />
              <span class="who">
                <strong><bdi>{clean(acc.account.displayName)}</bdi></strong>
                <span class="ui-hint">You can: {permissionLabels(acc.permissions)}</span>
              </span>
              <span class="ui-actions">
                {#if acc.permissions.includes('EMOTES') || acc.permissions.includes('SETS') || acc.permissions.includes('EDITORS')}
                  <a class="ui-btn ghost small" href="/me/sets#of-{acc.account.id}">Sets</a>
                {/if}
                {#if acc.permissions.includes('SETS')}
                  <a class="ui-btn ghost small" href="/me/channels#of-{acc.account.id}">Channels</a>
                {/if}
                <Confirm small label="Leave" confirm="Click again to leave" busy={busyId === acc.account.id}
                  onconfirm={() => leave(acc.account.id, acc.account.displayName)} />
              </span>
            </li>
          {/each}
        </ul>
      {:else}
        <State bare title="You don’t edit anyone’s account">When someone adds you as their editor, their account shows up here.</State>
      {/if}
    </section>
  </div>
  {/snippet}
  </LoadGate>
</div>

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="edit-title" onclose={() => (editing = null)}>
  <form onsubmit={save} novalidate>
    <div class="body">
      <h2 id="edit-title">{#if editing}What <bdi>{clean(editing.account.displayName)}</bdi> can do{/if}</h2>
      {@render checks(editPicked, (next) => (editPicked = next), 'edit')}
      {#if !editPicked.length}<p class="ui-hint">Pick at least one, or remove them instead.</p>{/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={saving || !editPicked.length}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  .who { display: grid; flex: 1; min-width: 0; line-height: 1.35; }
  .who strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .who .ui-hint { overflow-wrap: anywhere; }
  .ui-list .ui-actions { flex: none; }
  .add { display: grid; gap: 16px; }
  .add > h2 + * { margin-top: 0; }
  .ui-seg { justify-self: start; }
  .perms { display: grid; gap: 2px; margin: 0; padding: 0; border: 0; min-width: 0; }
  .perms legend { padding: 0; margin-bottom: 6px; }
  .perm { display: flex; align-items: flex-start; gap: 10px; min-height: 44px; padding: 6px 0; cursor: pointer; }
  .perm input { flex: none; width: 16px; height: 16px; margin: 3px 0 0; accent-color: var(--accent); }
  .perm .text { display: grid; gap: 2px; min-width: 0; }
  .perm strong { font-weight: 600; }
  .ui-dialog .body { min-width: min(420px, calc(100vw - 48px)); }
  @media (max-width: 560px) {
    .ui-list > li { flex-wrap: wrap; }
    .ui-list .ui-actions { width: 100%; padding-left: 44px; }
  }
</style>
