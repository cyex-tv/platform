<script lang="ts">
  // Your profile → Editors (?s=editors, owner only; it was Settings → Editors, /me/editors, until
  // 2026/11/22). Account editors (ADR 0014): the people who work on your sets and channels, with
  // what each one may do and on which sets, and the accounts you edit yourself (Editing for). Find
  // someone by name, pick them, and choose their permissions in the dialog; changing an editor
  // opens the same dialog. Two flat columns on wide screens.
  import { ApiError, gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, iso } from '$lib/format';
  import type { EditorPermission, EditorsQuery, FindPeopleQuery } from '$lib/gql/graphql';
  import { ADD_ACCOUNT_EDITOR, EDITORS, REMOVE_ACCOUNT_EDITOR, SEARCH_ACCOUNTS, UPDATE_ACCOUNT_EDITOR } from '$lib/queries';
  import Confirm from '$lib/tool/Confirm.svelte';
  import EditorDialog from '$lib/tool/EditorDialog.svelte';
  import { ALL_SETS, MAX_EDITORS, effective, effectiveLabels, effectivePhrase, emotePermissionsIdle, grantOn, ownerSetsHref, scopeInput, scopeLabel, scopeOf, type Scope } from '$lib/tool/editors';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { useToolButtons } from '$lib/components/btn';
  import { session } from '$lib/tool/session.svelte';

  useToolButtons(); // the editor controls are tool buttons, as in Settings

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
  /** One letter typed: said under the field, not as an error card. */
  let shortHint = $state(false);
  let token = 0;

  let dialog = $state<EditorDialog>();
  let busyId = $state<string | null>(null);

  async function search(e: SubmitEvent) {
    e.preventDefault();
    const text = query.trim().replace(/^@/, '');
    if ([...text].length < 2) {
      shortHint = true;
      results = null;
      return;
    }
    const mine = ++token;
    searching = true;
    searchError = null;
    shortHint = false;
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
      notify(`Saved. ${iso(next.account.displayName)} can now ${effectivePhrase(next)}.`);
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

<PageHead title="Editors" line="People who work on your sets and channels for you. You choose what each one can do and on which sets, and can take it back at any time." />

  <LoadGate source={L} label="Loading your editors…">
  {#snippet children(data)}
  {@const editors = data.accountEditors}
  {@const full = editors.length >= MAX_EDITORS}
  <div class="cq"><div class="cols">
  <div class="col">
  <Section title="Your editors" count="{editors.length} / {MAX_EDITORS}">
    {#each editors as ed (ed.account.id)}
      <Row label={clean(ed.account.displayName)}>
        {#snippet media()}<Avatar src={ed.account.avatar} name={ed.account.displayName} size={24} tone="tool" />{/snippet}
        {#snippet hint()}{effectiveLabels(ed)} · {scopeText(ed, ownSets)}{#if ed.grantedBy && ed.grantedBy.id !== session.me?.id} · added by <bdi>{clean(ed.grantedBy.displayName)}</bdi>{/if}{/snippet}
        <button class="ui-btn secondary small" type="button" onclick={() => change(ed)}
          aria-label="Change what {iso(ed.account.displayName)} can do">Change</button>
        <Confirm small label="Remove" confirm="Click again to remove" busy={busyId === ed.account.id} onconfirm={() => remove(ed)} />
      </Row>
    {:else}
      <State bare title="No editors yet">Find a moderator or manager below, and pick what they can do.</State>
    {/each}

    {#if full}
      <p class="note">You have {MAX_EDITORS} editors, the most an account can have. Remove one to add someone else.</p>
    {:else}
      <form class="find" role="search" onsubmit={search}>
        <label class="find-l" for="find-editor">Add an editor</label>
        <input id="find-editor" class="ui-input" type="search" bind:value={query} placeholder="Search by name" autocomplete="off" spellcheck="false"
          maxlength="64" enterkeyhint="search" aria-describedby="find-hint" aria-invalid={shortHint || searchError ? 'true' : undefined} />
        <button class="ui-btn secondary" type="submit" disabled={searching}><Icon name="search" size={15} />{searching ? 'Searching…' : 'Search'}</button>
      </form>
      {#if shortHint}
        <p class="ui-error" id="find-hint" role="alert">Type at least two letters of their name.</p>
      {:else}
        <p class="note" id="find-hint">Their Twitch or Kick name. They need to have signed in to cyex.tv once. Press Enter to search.</p>
      {/if}
      {#if searchError}
        <State bare kind="error" title="Couldn’t search">{searchError}</State>
      {:else if results && results.length === 0}
        <State bare title="No one found">No one called {searched} has signed in to cyex.tv yet.</State>
      {:else if results}
        <div class="results" role="group" aria-label="People found">
          {#each results as p (p.account.id)}
            {@const self = p.account.id === session.me?.id}
            {@const already = editorById.has(p.account.id)}
            <Row label={clean(p.account.displayName)} hint={logins(p)}>
              {#snippet media()}<Avatar src={p.account.avatar} name={p.account.displayName} size={24} tone="tool" />{/snippet}
              <button class="ui-btn small {already ? 'ghost' : 'secondary'}" type="button" disabled={self} onclick={() => pick(p)}
                aria-label="{self ? 'That’s you' : already ? 'Change' : 'Add'} {iso(p.account.displayName)}">
                {self ? 'You' : already ? 'Change' : 'Add'}
              </button>
            </Row>
          {/each}
        </div>
      {/if}
    {/if}
  </Section>

  </div>
  <div class="col">
  <Section title="Editing for" count={data.editorOf.length}>
    {#each data.editorOf as acc (acc.account.id)}
      {@const can = effective(acc)}
      <Row label={clean(acc.account.displayName)}>
        {#snippet media()}<Avatar src={acc.account.avatar} name={acc.account.displayName} size={24} tone="tool" />{/snippet}
        {#snippet hint()}You can {effectivePhrase(acc)}{acc.allSets ? '' : ` · ${acc.setIds.length === 1 ? '1 set' : `${acc.setIds.length} sets`}`}{emotePermissionsIdle(acc) ? '. Uploads, emote managing and transfers need all sets, so they’re off' : ''}{/snippet}
        {#if acc.permissions.includes('EMOTES') || acc.permissions.includes('SETS') || acc.permissions.includes('EDITORS')}
          <a class="ui-btn secondary small" href={ownerSetsHref(acc.account.id)}>Sets</a>
        {/if}
        {#if acc.permissions.includes('CHANNELS') || acc.permissions.includes('SETS')}
          <a class="ui-btn secondary small" href="/me/channels#of-{acc.account.id}">Channels</a>
        {/if}
        {#if can.includes('EMOTE_MANAGE')}
          <a class="ui-btn secondary small" href="/users/{encodeURIComponent(acc.account.id)}">Emotes</a>
        {/if}
        {#if can.includes('EMOTE_UPLOAD')}
          <a class="ui-btn secondary small" href="/me/upload?for={encodeURIComponent(acc.account.id)}">Upload</a>
        {/if}
        <Confirm small label="Leave" confirm="Click again to leave" busy={busyId === acc.account.id}
          onconfirm={() => leave(acc.account.id, acc.account.displayName)} />
      </Row>
    {:else}
      <State bare title="You don’t edit anyone’s account">When someone adds you as their editor, their account shows up here.</State>
    {/each}
  </Section>
  </div>
  </div></div>
  {/snippet}
  </LoadGate>

<EditorDialog bind:this={dialog} {held} sets={ownSets} onsave={save} {describe} />

<style>
  /* Two flat columns from 1240px of content (your editors | editing for: its rows carry up to five buttons), one below. */
  .cq { container-type: inline-size; }
  .cols { display: grid; grid-template-columns: minmax(0, 1fr); }
  .col { min-width: 0; }
  .col + .col > :global(.sec:first-child) { border-top: 1px solid var(--border); }
  @container (min-width: 1240px) {
    /* The hairline sits in the middle of the gutter: 24px of air on each side of it. */
    .cols { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); column-gap: 0; }
    .col:first-child { padding-right: 24px; }
    .col + .col { padding-left: 24px; border-left: 1px solid var(--border); }
    .col + .col > :global(.sec:first-child) { border-top: 0; }
  }
  .find { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; padding: 12px 0 4px; }
  .find-l { flex: 1 1 100%; font-size: var(--t-ui); font-weight: 500; }
  .find input { flex: 1 1 12rem; }
  .find .ui-btn { flex: none; }
  .note { margin: 4px 0 0; font-size: var(--t-small); color: var(--muted); }
  .results { max-height: 320px; overflow-y: auto; overscroll-behavior: contain; }
</style>
