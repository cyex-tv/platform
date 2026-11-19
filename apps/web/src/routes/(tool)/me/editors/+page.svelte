<script lang="ts">
  // Account editors (ADR 0014): the people who work on your sets and channels, with what each one
  // may do and on which sets, and the accounts you edit yourself. Find someone by name, pick them,
  // and choose their permissions in the dialog; changing an editor opens the same dialog.
  import { ApiError, gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, iso } from '$lib/format';
  import type { EditorPermission, EditorsQuery, FindPeopleQuery } from '$lib/gql/graphql';
  import { ADD_ACCOUNT_EDITOR, EDITORS, REMOVE_ACCOUNT_EDITOR, SEARCH_ACCOUNTS, UPDATE_ACCOUNT_EDITOR } from '$lib/queries';
  import Confirm from '$lib/tool/Confirm.svelte';
  import EditorDialog from '$lib/tool/EditorDialog.svelte';
  import { ALL_SETS, MAX_EDITORS, effective, emotePermissionsIdle, grantOn, permissionLabels, scopeInput, scopeLabel, scopeOf, type Scope } from '$lib/tool/editors';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { session } from '$lib/tool/session.svelte';

  type Editor = EditorsQuery['accountEditors'][number];
  type Person = FindPeopleQuery['searchAccounts']['items'][number];

  const L = loader(({ fetch }) => gql(EDITORS, {}, fetch));
  const held = $derived(grantOn([], session.me?.id, session.me?.id));
  const ownSets = $derived((L.data?.mySets ?? []).filter((s) => s.owner?.id === session.me?.id));

  // Finding someone: nothing until Enter (or the button).
  let query = $state('');
  let searched = $state('');
  let results = $state<Person[] | null>(null);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let token = 0;

  let dialog = $state<EditorDialog>();
  let busyId = $state<string | null>(null);

  async function search(e: SubmitEvent) {
    e.preventDefault();
    const text = query.trim().replace(/^@/, '');
    if ([...text].length < 2) {
      searchError = 'Type at least two letters of their name.';
      results = null;
      return;
    }
    const mine = ++token;
    searching = true;
    searchError = null;
    try {
      const r = await gql(SEARCH_ACCOUNTS, { query: text, page: 1, perPage: 20 });
      if (mine === token) {
        results = r.searchAccounts.items;
        searched = text;
      }
    } catch (err) {
      if (mine === token) {
        results = null;
        searchError = err instanceof ApiError && err.code === 'rate_limited' ? err.message : 'Searching didn’t work. Try again in a moment.';
      }
    } finally {
      if (mine === token) searching = false;
    }
  }

  const editorById = $derived(new Map((L.data?.accountEditors ?? []).map((e) => [e.account.id, e])));

  function pick(p: Person) {
    const existing = editorById.get(p.account.id);
    if (existing) return change(existing);
    dialog?.open(p.account, { permissions: ['EMOTES'], scope: ALL_SETS }, 'add');
  }

  function change(ed: Editor) {
    dialog?.open(ed.account, { permissions: ed.permissions, scope: scopeOf(ed) }, 'edit');
  }

  function describe(err: unknown): string {
    const code = err instanceof ApiError ? err.code : null;
    if (code === 'already_owner') return 'That’s you. You can do all of this already.';
    if (code === 'already_editor') return 'They’re an editor already. Close this and change them in the list.';
    if (code === 'too_many_editors') return `You have ${MAX_EDITORS} editors, the most an account can have. Remove one to add someone else.`;
    if (code === 'not_found') return 'That account isn’t available any more.';
    if (err instanceof ApiError) return err.message;
    return 'Saving didn’t work. Check your connection and try again.';
  }

  async function save(who: { id: string; displayName: string }, permissions: EditorPermission[], scope: Scope) {
    if (!L.data) return;
    const existing = editorById.get(who.id);
    if (existing) {
      const r = await gql(UPDATE_ACCOUNT_EDITOR, { accountId: who.id, permissions, setIds: scopeInput(scope) });
      const next = r.updateAccountEditor;
      L.set({ ...L.data, accountEditors: L.data.accountEditors.map((x) => (x.account.id === next.account.id ? next : x)) });
      notify(`Saved. ${iso(next.account.displayName)} can now: ${permissionLabels(next.permissions)}.`);
    } else {
      const r = await gql(ADD_ACCOUNT_EDITOR, { input: { accountId: who.id, permissions, setIds: scopeInput(scope) } });
      const added = r.addAccountEditor;
      L.set({ ...L.data, accountEditors: [...L.data.accountEditors, added] });
      notify(`${iso(added.account.displayName)} can now work on your account.`);
    }
  }

  async function remove(ed: Editor) {
    if (!L.data) return;
    busyId = ed.account.id;
    try {
      await gql(REMOVE_ACCOUNT_EDITOR, { accountId: ed.account.id });
      L.set({ ...L.data, accountEditors: L.data.accountEditors.filter((x) => x.account.id !== ed.account.id) });
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
      L.set({ ...L.data, editorOf: L.data.editorOf.filter((x) => x.account.id !== ownerId) });
      notify(`You no longer edit ${iso(name)}’s account.`);
    } catch (err) {
      fail(err);
    } finally {
      busyId = null;
    }
  }

  const logins = (p: Person) => p.identities.map((i) => `${i.platform === 'TWITCH' ? 'Twitch' : 'Kick'} ${i.login}`).join(' · ');
  const scopeText = (g: { allSets: boolean; setIds: readonly string[] }, sets: readonly { id: string; name: string }[]) =>
    g.allSets ? 'all sets' : scopeLabel(scopeOf(g), sets);
</script>

<svelte:head><title>Editors - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Editors" line="People who work on your sets and channels for you. You choose what each one can do and on which sets, and can take it back at any time." />

  <LoadGate source={L} label="Loading your editors…">
  {#snippet children(data)}
  {@const editors = data.accountEditors}
  {@const full = editors.length >= MAX_EDITORS}
  <div class="ui-stack">
    <section class="ui-card add">
      <h2><label for="find-editor">Add an editor</label></h2>
      {#if full}
        <p class="ui-hint">You have {MAX_EDITORS} editors, the most an account can have. Remove one to add someone else.</p>
      {:else}
        <form class="find" role="search" onsubmit={search}>
          <input id="find-editor" class="ui-input" type="search" bind:value={query} placeholder="Search by name" autocomplete="off" spellcheck="false"
            maxlength="64" enterkeyhint="search" aria-describedby="find-hint" aria-invalid={searchError ? 'true' : undefined} />
          <button class="ui-btn secondary" type="submit" disabled={searching}><Icon name="search" size={15} />{searching ? 'Searching…' : 'Search'}</button>
        </form>
        <p class="ui-hint" id="find-hint">Their Twitch or Kick name. They need to have signed in to cyex.tv once. Press Enter to search.</p>
        {#if searchError}
          <State bare kind="error" title="Couldn’t search">{searchError}</State>
        {:else if results && results.length === 0}
          <State bare title="No one found">No one called {searched} has signed in to cyex.tv yet.</State>
        {:else if results}
          <ul class="ui-list results" aria-label="People found">
            {#each results as p (p.account.id)}
              {@const self = p.account.id === session.me?.id}
              {@const already = editorById.has(p.account.id)}
              <li>
                <Avatar src={p.account.avatar} name={p.account.displayName} size={32} tone="tool" />
                <span class="who">
                  <strong><bdi>{clean(p.account.displayName)}</bdi></strong>
                  <span class="ui-hint">{logins(p)}</span>
                </span>
                <button class="ui-btn small {already ? 'ghost' : 'secondary'}" type="button" disabled={self} onclick={() => pick(p)}
                  aria-label="{self ? 'That’s you' : already ? 'Change' : 'Add'} {iso(p.account.displayName)}">
                  {self ? 'You' : already ? 'Change' : 'Add'}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </section>

    <section class="ui-panel">
      <header><h2>Your editors</h2><span class="ui-hint ui-num">{editors.length} / {MAX_EDITORS}</span></header>
      {#if editors.length}
        <ul class="ui-list">
          {#each editors as ed (ed.account.id)}
            <li>
              <Avatar src={ed.account.avatar} name={ed.account.displayName} size={32} tone="tool" />
              <span class="who">
                <strong><bdi>{clean(ed.account.displayName)}</bdi></strong>
                <span class="ui-hint">
                  {permissionLabels(ed.permissions)} · {scopeText(ed, ownSets)}{#if ed.grantedBy && ed.grantedBy.id !== session.me?.id} · added by <bdi>{clean(ed.grantedBy.displayName)}</bdi>{/if}
                </span>
              </span>
              <span class="ui-actions">
                <button class="ui-btn secondary small" type="button" onclick={() => change(ed)}
                  aria-label="Change what {iso(ed.account.displayName)} can do">Change</button>
                <Confirm small label="Remove" confirm="Click again to remove" busy={busyId === ed.account.id} onconfirm={() => remove(ed)} />
              </span>
            </li>
          {/each}
        </ul>
      {:else}
        <State bare title="No editors yet">Find a moderator or manager above, and pick what they can do.</State>
      {/if}
    </section>

    <section class="ui-panel">
      <header><h2>You edit</h2><span class="ui-hint ui-num">{data.editorOf.length}</span></header>
      {#if data.editorOf.length}
        <ul class="ui-list">
          {#each data.editorOf as acc (acc.account.id)}
            {@const can = effective(acc)}
            <li>
              <Avatar src={acc.account.avatar} name={acc.account.displayName} size={32} tone="tool" />
              <span class="who">
                <strong><bdi>{clean(acc.account.displayName)}</bdi></strong>
                <span class="ui-hint">You can: {permissionLabels(acc.permissions)}{acc.allSets ? '' : ` · ${acc.setIds.length === 1 ? '1 set' : `${acc.setIds.length} sets`}`}{emotePermissionsIdle(acc) ? '. Uploads, emote managing and transfers need all sets, so they’re off' : ''}</span>
              </span>
              <span class="ui-actions">
                {#if acc.permissions.includes('EMOTES') || acc.permissions.includes('SETS') || acc.permissions.includes('EDITORS')}
                  <a class="ui-btn ghost small" href="/me/sets#of-{acc.account.id}">Sets</a>
                {/if}
                {#if acc.permissions.includes('CHANNELS') || acc.permissions.includes('SETS')}
                  <a class="ui-btn ghost small" href="/me/channels#of-{acc.account.id}">Channels</a>
                {/if}
                {#if can.includes('EMOTE_MANAGE')}
                  <a class="ui-btn ghost small" href="/users/{encodeURIComponent(acc.account.id)}">Emotes</a>
                {/if}
                {#if can.includes('EMOTE_UPLOAD')}
                  <a class="ui-btn ghost small" href="/me/upload?for={encodeURIComponent(acc.account.id)}">Upload</a>
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

<EditorDialog bind:this={dialog} {held} sets={ownSets} onsave={save} {describe} />

<style>
  .who { display: grid; flex: 1; min-width: 0; line-height: 1.35; }
  .who strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .who .ui-hint { overflow-wrap: anywhere; }
  .ui-list .ui-actions { flex: none; }
  .add { display: grid; gap: 12px; }
  .add > h2 + * { margin-top: 0; }
  .find { display: flex; gap: 8px; }
  .find input { flex: 1; }
  .find .ui-btn { flex: none; }
  .results { max-height: 320px; overflow-y: auto; overscroll-behavior: contain; border: 1px solid var(--border); border-radius: var(--radius-card); }
  .results > li { padding: 8px 12px; }
  @media (max-width: 560px) {
    .ui-list:not(.results) > li { flex-wrap: wrap; }
    .ui-list:not(.results) .ui-actions { width: 100%; padding-left: 44px; }
  }
</style>
