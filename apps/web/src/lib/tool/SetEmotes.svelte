<script lang="ts">
  import { gql } from '$lib/api';
  import type { SetFieldsFragment } from '$lib/gql/graphql';
  import { REMOVE_SET_EMOTE, UPDATE_SET_EMOTE } from '$lib/queries';
  import { nameProblem } from '$lib/rules';
  import Thumb from './Thumb.svelte';
  import { STATE_LABEL } from './state';
  import { fail, notify } from './notices.svelte';

  let { set, onchange }: { set: SetFieldsFragment; onchange: () => void } = $props();

  let filter = $state('');
  let editing = $state<string | null>(null);
  let alias = $state('');
  let zeroWidth = $state(false);
  let dialog: HTMLDialogElement;
  const aliasError = $derived(nameProblem(alias));

  const shown = $derived(
    set.emotes.filter((m) => {
      const q = filter.trim().toLowerCase();
      return !q || m.name.toLowerCase().includes(q) || m.emote.name.toLowerCase().includes(q);
    }),
  );

  /** Its own name when renamed here, who made it, and zero-width. */
  function hint(m: SetFieldsFragment['emotes'][number]): string {
    return [m.name !== m.emote.name ? m.emote.name : null, m.emote.owner?.displayName ?? 'Unknown', m.zeroWidth ? 'zero-width' : null]
      .filter(Boolean)
      .join(' · ');
  }

  function edit(m: SetFieldsFragment['emotes'][number]) {
    editing = m.emote.id;
    alias = m.name;
    zeroWidth = m.zeroWidth;
    dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (!editing || aliasError) return;
    try {
      await gql(UPDATE_SET_EMOTE, { setId: set.id, emoteId: editing, input: { name: alias.trim(), zeroWidth } });
      dialog.close();
      notify('Saved.');
      onchange();
    } catch (err) {
      fail(err);
    }
  }

  async function remove(m: SetFieldsFragment['emotes'][number]) {
    try {
      await gql(REMOVE_SET_EMOTE, { setId: set.id, emoteId: m.emote.id });
      notify(`${m.name} removed.`);
      onchange();
    } catch (err) {
      fail(err);
    }
  }
</script>

<section class="ui-panel">
  <header>
    <h2>In this set</h2>
    <input class="ui-input filter" type="search" bind:value={filter} placeholder="Filter…" aria-label="Filter the set’s emotes" />
  </header>
  {#if set.emotes.length === 0}
    <div class="ui-empty"><p>Nothing in this set yet. Add emotes below.</p></div>
  {:else if shown.length === 0}
    <div class="ui-empty"><p>No emote here matches “{filter}”.</p></div>
  {:else}
    <ul class="ui-list">
      {#each shown as m (m.emote.id)}
        <li>
          <Thumb emote={m.emote} name={m.name} box={36} />
          <span class="what">
            <strong>{m.name}</strong>
            <span class="ui-hint">{hint(m)}</span>
          </span>
          {#if m.emote.state !== 'APPROVED'}<span class="ui-chip {m.emote.state.toLowerCase()}">{STATE_LABEL[m.emote.state]}</span>{/if}
          <button class="ui-icon" type="button" aria-label="Rename {m.name}" title="Rename" onclick={() => edit(m)}>
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4ZM14 6l4 4" /></svg>
          </button>
          <button class="ui-icon danger" type="button" aria-label="Remove {m.name}" title="Remove" onclick={() => remove(m)}>
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M5 7h14M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="alias-title">
  <form onsubmit={save}>
    <div class="body">
      <h2 id="alias-title">In this set</h2>
      <label class="ui-field">
        <span class="ui-label">Name here</span>
        <input class="ui-input" bind:value={alias} maxlength="100" spellcheck="false" aria-invalid={aliasError ? 'true' : undefined} />
        {#if aliasError}<span class="ui-error">{aliasError}</span>{:else}<span class="ui-hint">Only this set; the emote keeps its own name.</span>{/if}
      </label>
      <label class="ui-row">
        <div><strong>Zero-width</strong><span class="ui-hint">Drawn over the emote before it.</span></div>
        <input class="ui-switch" type="checkbox" bind:checked={zeroWidth} />
      </label>
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={!!aliasError}>Save</button>
    </footer>
  </form>
</dialog>

<style>
  .filter { width: min(220px, 50%); padding: 6px 10px; font-size: .82rem; background: var(--bg); }
  .what { display: grid; min-width: 0; flex: 1; }
  .what strong, .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) { .danger:hover { color: var(--red); } }
  .ui-row .ui-hint { display: block; }
</style>
