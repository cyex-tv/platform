<script lang="ts">
  import { gql } from '$lib/api';
  import type { SetFieldsFragment } from '$lib/gql/graphql';
  import { REMOVE_SET_EMOTE, UPDATE_SET_EMOTE } from '$lib/queries';
  import { nameProblem } from '$lib/rules';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import type { CardEmote } from '$lib/components/EmoteTile.svelte';
  import { count, iso } from '$lib/format';
  import State from '$lib/components/State.svelte';
  import { session } from './session.svelte';
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

  type Member = SetFieldsFragment['emotes'][number];
  const byId = $derived(new Map(set.emotes.map((m) => [m.emote.id, m])));
  /** An emote as this set shows it: its set name when renamed, zero-width as set here. */
  const tiles = $derived<CardEmote[]>(
    shown.map((m) => ({ ...m.emote, shownName: m.name !== m.emote.name ? m.name : undefined, flags: { ...m.emote.flags, zeroWidth: m.zeroWidth } })),
  );
  const slots = $derived(Math.min(100, (set.emotes.length / Math.max(1, set.capacity)) * 100));

  function edit(m: Member) {
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

  async function remove(m: Member) {
    try {
      await gql(REMOVE_SET_EMOTE, { setId: set.id, emoteId: m.emote.id });
      notify(`${iso(m.name)} removed.`);
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
  <div class="meter" role="meter" aria-label="Slots used" aria-valuemin={0} aria-valuemax={set.capacity} aria-valuenow={set.emotes.length}>
    <span class="bar"><span style:width="{slots}%"></span></span>
    <span class="ui-hint ui-num">{count(set.emotes.length)} / {count(set.capacity)}</span>
  </div>
  {#if set.emotes.length === 0}
    <State bare title="Nothing in this set yet">Add emotes from the panel below.</State>
  {:else if shown.length === 0}
    <State bare title="No match">No emote here matches “{filter}”.</State>
  {:else}
    <div class="grid">
      <EmoteGrid
        items={tiles}
        label="Emotes in this set"
        eager={12}
        href={(e) => (byId.get(e.id)?.emote.owner?.id === session.me?.id ? `/me/emotes/${e.id}` : `/emotes/${e.id}`)}
        note={(e) => {
          const st = byId.get(e.id)?.emote.state;
          return st && st !== 'APPROVED' ? STATE_LABEL[st].toLowerCase() : undefined;
        }}
      >
        {#snippet below(e)}
          {@const m = byId.get(e.id)}
          {#if m}
            <button class="ui-icon" type="button" aria-label="Rename {iso(m.name)}" title="Rename" onclick={() => edit(m)}>
              <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4ZM14 6l4 4" /></svg>
            </button>
            <button class="ui-icon danger" type="button" aria-label="Remove {iso(m.name)}" title="Remove" onclick={() => remove(m)}>
              <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M5 7h14M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg>
            </button>
          {/if}
        {/snippet}
      </EmoteGrid>
    </div>
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
  .filter { width: min(240px, 50%); }
  .meter { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-bottom: 1px solid var(--border); }
  .bar { flex: 1; height: 4px; border-radius: 2px; background: var(--border-strong); overflow: hidden; }
  .bar > span { display: block; height: 100%; background: var(--accent); border-radius: 2px; }
  .grid { padding: 16px; }
  @media (hover: hover) and (pointer: fine) { .danger:hover { color: var(--red); } }
  .ui-row .ui-hint { display: block; }
</style>
