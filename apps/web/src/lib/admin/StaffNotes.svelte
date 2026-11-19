<script lang="ts">
  // Staff notes on an account: newest first, with author and time. Only staff with users.notes
  // see them; the account never does. You can delete your own notes.
  import { gql } from '$lib/api';
  import type { StaffNotesQuery } from '$lib/gql/graphql';
  import { ADD_STAFF_NOTE, DELETE_STAFF_NOTE, STAFF_NOTES } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import { dateTime } from '$lib/format';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import AccountLink from './AccountLink.svelte';

  type Note = StaffNotesQuery['staffNotes'][number];
  let { accountId }: { accountId: string } = $props();
  const MAX = 2000;

  let notes = $state<Note[] | null>(null);
  let problem = $state(false);
  let body = $state('');
  let busy = $state(false);
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = false;
    try {
      const r = (await gql(STAFF_NOTES, { accountId })).staffNotes;
      if (mine === seq) notes = r;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void accountId;
    notes = null;
    load();
  });

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const text = body.trim();
    if (!text || busy) return;
    busy = true;
    try {
      const n = (await gql(ADD_STAFF_NOTE, { accountId, body: text })).addStaffNote;
      notes = [n, ...(notes ?? [])];
      body = '';
      notify('Note saved.');
    } catch (err) {
      fail(err);
    } finally {
      busy = false;
    }
  }

  async function remove(n: Note) {
    try {
      await gql(DELETE_STAFF_NOTE, { id: n.id });
      notes = (notes ?? []).filter((x) => x.id !== n.id);
      notify('Note deleted.');
    } catch (err) {
      fail(err);
    }
  }
</script>

<section class="ui-panel" aria-labelledby="sn-h">
  <header><h2 id="sn-h">Staff notes</h2>{#if notes?.length}<span class="ui-hint ui-num">{notes.length}</span>{/if}</header>
  <form class="add" onsubmit={add}>
    <label class="sr-only" for="sn-body">A note for staff</label>
    <textarea id="sn-body" class="ui-input" rows="2" maxlength={MAX} bind:value={body} placeholder="A note only staff see: context, warnings, what was agreed."></textarea>
    <div class="row">
      <span class="ui-hint ui-num" aria-live="polite">{body.length > MAX - 200 ? `${MAX - body.length} left` : ''}</span>
      <button class="ui-btn secondary small" type="submit" disabled={busy || !body.trim()}>{busy ? 'Saving…' : 'Add note'}</button>
    </div>
  </form>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !notes}
    <State bare kind="loading" title="Loading notes…" />
  {:else if notes.length === 0}
    <p class="ui-hint empty">No notes yet.</p>
  {:else}
    <ul class="ui-list notes">
      {#each notes as n (n.id)}
        <li>
          <div class="note">
            <p class="body">{n.body}</p>
            <span class="ui-hint"><AccountLink id={n.authorId} /> · <span class="ui-num">{dateTime(n.createdAt)}</span></span>
          </div>
          {#if n.authorId === session.me?.id}<Confirm small label="Delete" confirm="Click again" onconfirm={() => remove(n)} />{/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .add { display: grid; gap: 6px; padding: 10px 14px; border-bottom: 1px solid var(--border); }
  textarea { resize: vertical; min-height: 56px; background: var(--bg); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .empty { margin: 0; padding: 10px 14px 12px; }
  .notes > li { align-items: flex-start; }
  .note { display: grid; gap: 4px; flex: 1; min-width: 0; }
  .body { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font-size: var(--text-sub); }
</style>
