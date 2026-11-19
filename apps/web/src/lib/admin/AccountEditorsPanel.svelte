<script lang="ts">
  // An account's editors for staff with editors.manage_any: who edits it and with what, and
  // adding, changing and removing them on the owner's behalf (accountEditors, addAccountEditor,
  // updateAccountEditor and removeAccountEditor with ownerId). The API audits each change as a
  // staff action. Permissions are the Editors page's own (lib/tool/editors.ts).
  import { gql } from '$lib/api';
  import type { AccountEditorsQuery, EditorPermission, Platform } from '$lib/gql/graphql';
  import { ACCOUNT_EDITORS, ADD_ACCOUNT_EDITOR, REMOVE_ACCOUNT_EDITOR, UPDATE_ACCOUNT_EDITOR } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import { clean, iso } from '$lib/format';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { PERMISSIONS, permissionLabels, toggle, locked, MAX_EDITORS } from '$lib/tool/editors';

  type Row = AccountEditorsQuery['accountEditors'][number];
  let { accountId, name }: { accountId: string; name: string } = $props();

  // Reserved flags (stored, checked later) stay out of this short form; the Editors page has them.
  const OFFER = PERMISSIONS.filter((p) => !p.reserved);

  let rows = $state<Row[] | null>(null);
  let problem = $state(false);
  let busy = $state<string | null>(null);
  let editing = $state<string | null>(null);
  let draft = $state<EditorPermission[]>([]);
  let platform = $state<Platform>('TWITCH');
  let login = $state('');
  let fresh = $state<EditorPermission[]>(['EMOTES']);
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = false;
    try {
      const r = (await gql(ACCOUNT_EDITORS, { accountId })).accountEditors;
      if (mine === seq) rows = r;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void accountId;
    rows = null;
    editing = null;
    load();
  });

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const who = login.trim();
    if (!who || !fresh.length) return;
    busy = 'add';
    try {
      const r = (await gql(ADD_ACCOUNT_EDITOR, { input: { ownerId: accountId, platform, login: who, permissions: fresh } })).addAccountEditor;
      rows = [...(rows ?? []), { account: r.account, permissions: r.permissions }];
      login = '';
      notify(`${iso(r.account.displayName)} edits ${iso(name)}’s account now.`);
    } catch (err) {
      fail(err);
    } finally {
      busy = null;
    }
  }

  async function save(row: Row) {
    if (!draft.length) return;
    busy = row.account.id;
    try {
      const r = (await gql(UPDATE_ACCOUNT_EDITOR, { ownerId: accountId, accountId: row.account.id, permissions: draft })).updateAccountEditor;
      rows = (rows ?? []).map((x) => (x.account.id === row.account.id ? { ...x, permissions: r.permissions } : x));
      editing = null;
      notify('Saved.');
    } catch (err) {
      fail(err);
    } finally {
      busy = null;
    }
  }

  async function remove(row: Row) {
    busy = row.account.id;
    try {
      await gql(REMOVE_ACCOUNT_EDITOR, { ownerId: accountId, accountId: row.account.id });
      rows = (rows ?? []).filter((x) => x.account.id !== row.account.id);
      notify(`${iso(row.account.displayName)} no longer edits ${iso(name)}’s account.`);
    } catch (err) {
      fail(err);
    } finally {
      busy = null;
    }
  }
</script>

{#snippet flags(list: EditorPermission[], set: (v: EditorPermission[]) => void, idp: string)}
  <fieldset class="flags">
    <legend class="sr-only">Permissions</legend>
    {#each OFFER as p (p.key)}
      <label class="ui-check" title={p.line}>
        <input type="checkbox" checked={list.includes(p.key)} disabled={locked(list, p.key)} aria-describedby="{idp}-{p.key}"
          onchange={(e) => set(toggle(list, p.key, e.currentTarget.checked))} />
        <span>{p.label}</span>
        <span class="sr-only" id="{idp}-{p.key}">{p.line}</span>
      </label>
    {/each}
  </fieldset>
{/snippet}

<section class="ui-panel" aria-labelledby="ed-h">
  <header><h2 id="ed-h">Editors</h2>{#if rows}<span class="ui-hint ui-num">{rows.length}</span>{/if}</header>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading editors…" />
  {:else}
    {#if rows.length}
      <ul class="ui-list">
        {#each rows as r (r.account.id)}
          <li class="row">
            <span class="what">
              <a href="/admin/users/{encodeURIComponent(r.account.id)}"><bdi>{clean(r.account.displayName)}</bdi></a>
              <span class="ui-hint">{permissionLabels(r.permissions)}</span>
            </span>
            {#if editing !== r.account.id}
              <span class="acts">
                <button class="ui-btn ghost small" type="button" disabled={busy !== null} onclick={() => ((editing = r.account.id), (draft = [...r.permissions]))}>Change</button>
                <Confirm small label="Remove" confirm="Click again to remove" busy={busy === r.account.id} onconfirm={() => remove(r)} />
              </span>
            {:else}
              <div class="edit">
                {@render flags(draft, (v) => (draft = v), `ed-${r.account.id}`)}
                <div class="ui-actions">
                  <button class="ui-btn primary small" type="button" disabled={busy !== null || !draft.length} onclick={() => save(r)}>{busy === r.account.id ? 'Saving…' : 'Save'}</button>
                  <button class="ui-btn ghost small" type="button" onclick={() => (editing = null)}>Cancel</button>
                </div>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {:else}
      <p class="ui-hint pad">Nobody edits this account.</p>
    {/if}
    {#if rows.length < MAX_EDITORS}
      <form class="add" onsubmit={add} novalidate>
        <div class="who">
          <select class="ui-input" bind:value={platform} aria-label="Platform">
            <option value="TWITCH">Twitch</option>
            <option value="KICK">Kick</option>
          </select>
          <input class="ui-input" bind:value={login} placeholder="Login" aria-label="Login of the new editor" autocomplete="off" spellcheck="false" />
          <button class="ui-btn secondary" type="submit" disabled={busy !== null || !login.trim() || !fresh.length}>{busy === 'add' ? 'Adding…' : 'Add editor'}</button>
        </div>
        {@render flags(fresh, (v) => (fresh = v), 'ed-new')}
      </form>
    {/if}
  {/if}
</section>

<style>
  .row { flex-wrap: wrap; }
  .what { display: grid; flex: 1; min-width: 0; }
  .what a { display: block; min-height: 24px; line-height: 24px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acts { display: flex; gap: 6px; }
  .edit { flex-basis: 100%; display: grid; gap: 6px; }
  .flags { display: flex; flex-wrap: wrap; gap: 0 16px; margin: 0; padding: 0; border: 0; }
  .add { display: grid; gap: 6px; padding: 12px 14px 10px; border-top: 1px solid var(--border); }
  .who { display: flex; flex-wrap: wrap; gap: 8px; }
  .who select { flex: 0 0 auto; width: auto; }
  .who input { flex: 1 1 140px; min-width: 0; }
  .pad { margin: 0; padding: 10px 14px 12px; }
</style>
