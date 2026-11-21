<script lang="ts">
  // Staff-only notes, newest first, with author and time, and a field to add one. You can delete
  // your own. Shared by account notes (users.notes) and emote review notes (emotes.review): the
  // caller hands in how to load, add and delete, keyed by `key` (it reloads when that changes).
  import State from '$lib/components/State.svelte';
  import { dateTime } from '$lib/format';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import AccountLink from './AccountLink.svelte';
  import More from './ui/More.svelte';

  type Note = { id: string; authorId?: string | null; body: string; createdAt: string };
  type Props = {
    key: string;
    load: () => Promise<Note[]>;
    add: (body: string) => Promise<Note>;
    remove: (id: string) => Promise<unknown>;
    placeholder: string;
    /** How many notes show before Show all (default 5). */
    limit?: number;
    /** Reports the count, for a tab label. */
    oncount?: (n: number) => void;
  };
  let { key, load, add, remove, placeholder, limit = 5, oncount }: Props = $props();
  const MAX = 2000;
  const uid = $props.id();

  let notes = $state<Note[] | null>(null);
  let problem = $state(false);
  let body = $state('');
  let busy = $state(false);
  let all = $state(false);
  let seq = 0;

  async function fetchNotes() {
    const mine = ++seq;
    problem = false;
    try {
      const r = await load();
      if (mine === seq) notes = r;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void key;
    notes = null;
    all = false;
    fetchNotes();
  });
  $effect(() => {
    if (notes) oncount?.(notes.length);
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    const text = body.trim();
    if (!text || busy) return;
    busy = true;
    try {
      const n = await add(text);
      notes = [n, ...(notes ?? [])];
      body = '';
      notify('Note saved.');
    } catch (err) {
      fail(err);
    } finally {
      busy = false;
    }
  }

  async function drop(n: Note) {
    try {
      await remove(n.id);
      notes = (notes ?? []).filter((x) => x.id !== n.id);
      notify('Note deleted.');
    } catch (err) {
      fail(err);
    }
  }
</script>

<form class="add" onsubmit={submit}>
  <label class="sr-only" for="{uid}-body">A note for staff</label>
  <textarea id="{uid}-body" class="ui-input" rows="2" maxlength={MAX} bind:value={body} {placeholder}></textarea>
  <div class="row">
    <span class="ui-hint ui-num" aria-live="polite">{body.length > MAX - 200 ? `${MAX - body.length} left` : ''}</span>
    <button class="ui-btn secondary small" type="submit" disabled={busy || !body.trim()}>{busy ? 'Saving…' : 'Add note'}</button>
  </div>
</form>
{#if problem}
  <State bare kind="error" title="That didn’t load">
    {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={fetchNotes}>Try again</button>{/snippet}
  </State>
{:else if !notes}
  <State bare kind="loading" title="Loading notes…" />
{:else if notes.length === 0}
  <p class="empty">No notes yet.</p>
{:else}
  <ul class="ui-list notes">
    {#each all ? notes : notes.slice(0, limit) as n (n.id)}
      <li>
        <div class="note">
          <p class="body">{n.body}</p>
          <span class="ui-hint"><AccountLink id={n.authorId} /> · <span class="ui-num">{dateTime(n.createdAt)}</span></span>
        </div>
        {#if n.authorId === session.me?.id}<Confirm small label="Delete" confirm="Click again" onconfirm={() => drop(n)} />{/if}
      </li>
    {/each}
  </ul>
  <More total={notes.length} {limit} bind:expanded={all} />
{/if}

<style>
  .add { display: grid; gap: 6px; padding: 8px 0 12px; border-bottom: 1px solid var(--border); }
  textarea { resize: vertical; min-height: 56px; background: var(--bg); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 28px; }
  .empty { margin: 0; padding: 12px 0; font-size: var(--t-small); color: var(--muted); }
  .notes > li { align-items: flex-start; padding-left: 0; padding-right: 0; }
  .note { display: grid; gap: 2px; flex: 1; min-width: 0; }
  .body { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font-size: var(--t-small); }
</style>
