<script lang="ts">
  // The review queue's ready reject reasons (review.settings): what the reject dialog offers in one
  // click. A short list, edited in place: change the words, move a reason up or down, remove it,
  // add one (at most 12, each up to 200 characters on one line). Save sends the whole list
  // (setReviewReasons, audited). Reviewers without review.settings see the list read-only.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import State from '$lib/components/State.svelte';
  import { count } from '$lib/format';
  import { REVIEW_REASONS, SET_REVIEW_REASONS } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';
  import { tick } from 'svelte';

  const MAX = 12;
  const LEN = 200;
  const editable = can('review.settings');

  type Row = { key: number; text: string };
  let seq = 0;
  let rows = $state<Row[]>([]);
  let saved = $state<string[]>([]);
  let busy = $state(false);
  let list = $state<HTMLOListElement>();

  const L = loader(async ({ fetch }) => {
    const r = (await gql(REVIEW_REASONS, {}, fetch)).reviewReasons;
    saved = r;
    rows = r.map((text) => ({ key: ++seq, text }));
    return r;
  });

  const cleaned = $derived(rows.map((r) => r.text.trim()));
  const dirty = $derived(cleaned.join('\n') !== saved.join('\n'));
  const blank = $derived(cleaned.some((t) => !t));
  const repeats = $derived(cleaned.filter((t, i) => t && cleaned.findIndex((o) => o.toLowerCase() === t.toLowerCase()) !== i).length);

  async function focusRow(i: number) {
    await tick();
    list?.querySelectorAll<HTMLInputElement>('input')[i]?.focus();
  }
  function add() {
    if (rows.length >= MAX) return;
    rows = [...rows, { key: ++seq, text: '' }];
    void focusRow(rows.length - 1);
  }
  function remove(i: number) {
    rows = rows.filter((_, k) => k !== i);
    void focusRow(Math.min(i, rows.length - 1));
  }
  function moveRow(i: number, d: -1 | 1) {
    const j = i + d;
    if (j < 0 || j >= rows.length) return;
    const next = [...rows];
    [next[i], next[j]] = [next[j], next[i]];
    rows = next;
  }
  function reset() {
    rows = saved.map((text) => ({ key: ++seq, text }));
  }
  async function save(ev: SubmitEvent) {
    ev.preventDefault();
    if (busy || !dirty || blank) return;
    busy = true;
    try {
      const r = (await gql(SET_REVIEW_REASONS, { reasons: cleaned })).setReviewReasons;
      saved = r;
      rows = r.map((text) => ({ key: ++seq, text }));
      notify('Reject reasons saved.');
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>Reject reasons - Admin - cyex.tv</title></svelte:head>

<Bar title="Reject reasons" back={{ href: '/admin/review', label: 'Review queue' }} count={L.data ? `${count(rows.length)} of ${MAX}` : null}>
  {#snippet actions()}
    {#if editable}
      {#if dirty}<button class="ui-btn secondary small" type="button" disabled={busy} onclick={reset}>Undo changes</button>{/if}
      <button class="ui-btn primary small" type="submit" form="reasons-form" disabled={busy || !dirty || blank}>Save</button>
    {/if}
  {/snippet}
</Bar>

<LoadGate source={L} label="Loading the reasons…">
{#snippet children()}
  <form id="reasons-form" class="reasons" onsubmit={save}>
    {#if rows.length === 0}
      <State bare title="No ready reasons.">Reviewers type every reason themselves.{#if editable} Add one to offer it in the reject dialog.{/if}</State>
    {:else}
      <ol class="adm-rows rows" aria-label="Reject reasons" bind:this={list}>
        {#each rows as r, i (r.key)}
          <li>
            <span class="no ui-num" aria-hidden="true">{i + 1}</span>
            <label class="field">
              <span class="sr-only">Reason {i + 1}</span>
              <input class="ui-input" type="text" bind:value={r.text} maxlength={LEN} readonly={!editable} spellcheck="true" autocomplete="off"
                aria-invalid={!r.text.trim() || undefined} />
            </label>
            {#if editable}
              <span class="tools">
                <IconButton label="Move up" icon="arrow-up" size="s" disabled={i === 0} onclick={() => moveRow(i, -1)} />
                <IconButton label="Move down" icon="arrow-down" size="s" disabled={i === rows.length - 1} onclick={() => moveRow(i, 1)} />
                <IconButton label="Remove" icon="trash" size="s" onclick={() => remove(i)} />
              </span>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
    {#if editable}
      <footer>
        <button class="ui-btn secondary small" type="button" disabled={rows.length >= MAX} onclick={add}><Icon name="plus" size={15} />Add a reason</button>
        {#if blank || repeats}<span class="ui-hint count">{#if blank}Fill in or remove the empty one{:else}Repeats are kept once{/if}</span>{/if}
      </footer>
    {:else}
      <p class="ui-hint ro">Changing these needs the Edit reject reasons permission.</p>
    {/if}
  </form>
{/snippet}
</LoadGate>

<style>
  .reasons { display: grid; gap: 12px; margin: 0; }
  .rows li { display: grid; grid-template-columns: 36px minmax(0, 1fr) auto; align-items: center; min-height: 48px; padding-right: 6px; }
  .rows li:focus-within { box-shadow: inset 0 0 0 1px var(--accent-border); }
  .no { color: var(--dim); font-size: var(--t-fine); text-align: center; }
  .field input { width: 100%; height: 44px; border: 0; border-radius: 0; background: transparent; padding-inline: 4px; }
  .field input:focus-visible { outline-offset: -2px; }
  .tools { display: flex; gap: 2px; }
  footer { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; }
  .ro { margin: 0; }
  @media (max-width: 520px) {
    .rows li { grid-template-columns: minmax(0, 1fr) auto; padding-left: 8px; }
    .no { display: none; }
  }
</style>
