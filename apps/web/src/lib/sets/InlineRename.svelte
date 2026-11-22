<script lang="ts">
  // Renaming a set where its name is (the set menu's Rename): the name becomes a field, Enter or
  // leaving it saves, Escape puts the old name back. Saved with updateSet; a refusal shows as a
  // toast and the field stays so it can be fixed.
  //   {#if renaming === s.id}<InlineRename id={s.id} name={s.name} ondone={(n) => …} />{/if}
  import { ApiError, gql } from '$lib/api';
  import { UPDATE_SET } from '$lib/queries';
  import { notify } from '$lib/tool/notices.svelte';

  type Props = {
    id: string;
    name: string;
    /** Looks like the text it replaces: the page title or a card's name. */
    size?: 'title' | 'row';
    /** The new name, or null when nothing changed (Escape, or the same name). */
    ondone: (name: string | null) => void;
  };
  let { id, name, size = 'row', ondone }: Props = $props();

  let value = $state('');
  let input = $state<HTMLInputElement>();
  let busy = false;
  let finished = false;
  $effect(() => {
    value = name;
    queueMicrotask(() => {
      input?.focus();
      input?.select();
    });
  });

  async function save() {
    if (busy || finished) return;
    const next = value.trim();
    if (!next || next === name) {
      finished = true;
      ondone(null);
      return;
    }
    if (next.length > 100) {
      notify('Set names are 1 to 100 characters.', true);
      return;
    }
    busy = true;
    try {
      const r = await gql(UPDATE_SET, { id, input: { name: next } });
      finished = true;
      notify('Renamed.');
      ondone(r.updateSet.name);
    } catch (err) {
      notify(err instanceof ApiError && err.message ? err.message : 'That didn’t work. Try again.', true);
      input?.focus();
    } finally {
      busy = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Enter') {
      e.preventDefault();
      void save();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      finished = true;
      ondone(null);
    }
  }
</script>

<!-- svelte-ignore a11y_autofocus -->
<input class="rename {size}" bind:this={input} bind:value maxlength="100" aria-label="Set name" autocomplete="off" spellcheck="false"
  {onkeydown} onblur={() => void save()} onclick={(e) => { e.preventDefault(); e.stopPropagation(); }} />

<style>
  .rename { width: 100%; min-width: 0; margin: 0; padding: 0 6px; border: 1px solid var(--accent-border, var(--border-strong)); border-radius: var(--radius-btn);
    background: var(--bg); color: var(--text); font: inherit; font-weight: 600; }
  .rename:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 0; }
  .row { height: 28px; font-size: inherit; }
  .title { height: 44px; margin-left: -7px; font-size: inherit; font-weight: inherit; letter-spacing: inherit; }
  @media (pointer: coarse) { .rename { font-size: 16px; } }
</style>
