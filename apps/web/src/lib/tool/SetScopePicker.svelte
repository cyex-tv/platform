<script lang="ts">
  // Which sets an account editor may touch, for the permissions dialog: a trigger that reads
  // "All sets", the set's name or "3 sets", and a dropdown floating over the dialog (the shared
  // Popover, in the top layer, so it never pushes the layout) with a filter, "All sets" (only when
  // your own grant covers every set) and a box per set. It stays open while you tick several and
  // closes on Confirm, a click outside or Esc (Esc closes only the dropdown, not the dialog; focus
  // goes back to the trigger). Keys: Enter, Space or ↓ on the trigger open it with focus in the
  // filter; ↑ ↓ move between the filter and the rows; Space ticks; Enter confirms.
  //   <SetScopePicker bind:scope {mine} {sets} />
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { clean } from '$lib/format';
  import { ALL_SETS, covers, scopeLabel, toggleSet, type Scope } from './editors';

  type SetOption = { id: string; name: string; kind?: string };

  let {
    scope = $bindable(),
    mine = ALL_SETS,
    sets,
    open = $bindable(false),
    labelledby,
  }: {
    scope: Scope;
    /** The caller's own scope: a manager limited to some sets only sees theirs, and never "All sets". */
    mine?: Scope;
    sets: readonly SetOption[];
    open?: boolean;
    /** The id of the field label the trigger is read with. */
    labelledby?: string;
  } = $props();

  const uid = $props.id();
  let button = $state<HTMLButtonElement>();
  let list = $state<HTMLDivElement>();
  let query = $state('');
  let width = $state(320);

  const offered = $derived(mine.all ? sets : sets.filter((s) => covers(mine, s.id)));
  const shown = $derived.by(() => {
    const q = query.trim().toLocaleLowerCase();
    return q ? offered.filter((s) => s.name.toLocaleLowerCase().includes(q)) : offered;
  });
  const label = $derived(scope.all ? 'All sets' : scope.ids.length ? scopeLabel(scope, sets) : 'Pick sets');

  $effect(() => {
    if (!open) query = '';
  });

  function show() {
    width = button?.offsetWidth ?? 320;
    open = true;
  }

  function done() {
    open = false;
    button?.focus();
  }

  function pickSet(id: string, on: boolean) {
    const next = toggleSet(scope, id, on);
    // A limited manager can't hand out "All sets": unticking their last set leaves none picked.
    scope = !mine.all && next.all ? { all: false, ids: [] } : next;
  }

  function onTriggerKey(e: KeyboardEvent) {
    if (!open && e.key === 'ArrowDown') {
      e.preventDefault();
      show();
    }
  }

  // ↑ ↓ between the filter and the rows; Enter confirms (and never submits the dialog's form).
  function onListKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      done();
      return;
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    const items = [...(list?.querySelectorAll<HTMLElement>('input') ?? [])];
    if (!items.length) return;
    e.preventDefault();
    const at = items.indexOf(document.activeElement as HTMLElement);
    const step = e.key === 'ArrowDown' ? 1 : -1;
    items[(at + step + items.length) % items.length]?.focus();
  }

  // The dialog around this closes on Esc: while the dropdown is open, Esc is its alone. Cancelling
  // the keydown keeps the browser from asking the dialog to close; the Popover still closes itself.
  function guardEscape(e: KeyboardEvent) {
    if (open && e.key === 'Escape') e.preventDefault();
  }
</script>

<div class="scope" onkeydowncapture={guardEscape} role="presentation">
  <Popover label="Sets they can edit" bind:open width="{width}px">
    {#snippet trigger(props)}
      <button {...props} class="ui-input pick" type="button" bind:this={button} aria-labelledby="{labelledby ?? ''} {uid}-value"
        onclick={() => (open ? (open = false) : show())} onkeydown={onTriggerKey}>
        <span id="{uid}-value">{label}</span>
        <Icon name="chevron-down" size={16} />
      </button>
    {/snippet}
    <div class="list" bind:this={list} onkeydown={onListKey} role="presentation">
      <div class="find">
        <Icon name="search" size={13} />
        <input type="search" placeholder="Filter sets" aria-label="Filter sets" autocomplete="off" spellcheck="false" bind:value={query} />
      </div>
      {#if mine.all}
        <label class="row" class:on={scope.all}>
          <input type="checkbox" checked={scope.all} onchange={(e) => (scope = e.currentTarget.checked ? ALL_SETS : { all: false, ids: [] })} />
          <span class="name">All sets</span><span class="aside">New ones too</span>
        </label>
      {/if}
      {#each shown as s (s.id)}
        {@const on = !scope.all && scope.ids.includes(s.id)}
        <label class="row" class:on>
          <input type="checkbox" checked={on} onchange={(e) => pickSet(s.id, e.currentTarget.checked)} />
          <bdi class="name">{clean(s.name)}</bdi>{#if s.kind === 'PERSONAL'}<span class="aside">Personal</span>{/if}
        </label>
      {:else}
        <p class="none">{offered.length ? 'No set matches.' : 'No sets yet.'}</p>
      {/each}
    </div>
    {#snippet footer()}
      <button class="ui-btn primary small" type="button" onclick={done}>Confirm</button>
    {/snippet}
  </Popover>
</div>

<style>
  .scope { display: grid; min-width: 0; }
  .scope :global(.pop-root) { display: flex; min-width: 0; }
  .pick { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; min-height: 36px; padding: 6px 10px;
    font-size: var(--t-small); text-align: left; cursor: pointer; }
  .pick span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pick :global(svg) { flex: none; color: var(--muted); }
  .list { display: grid; }
  .find { position: relative; display: flex; align-items: center; margin-bottom: 4px; }
  .find :global(svg) { position: absolute; left: 9px; color: var(--dim); pointer-events: none; }
  .find input { width: 100%; height: 32px; padding: 0 8px 0 28px; border: 1px solid var(--field-border); border-radius: var(--radius-btn);
    background: var(--surface); color: var(--text); font: inherit; font-size: var(--t-small); }
  .find input::-webkit-search-cancel-button { display: none; }
  .find input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 0; }
  .row { display: flex; align-items: center; gap: 8px; min-height: 32px; padding: 0 6px; border-radius: var(--radius-btn); cursor: pointer;
    font-size: var(--t-small); }
  .row input { flex: none; width: 16px; height: 16px; margin: 0; accent-color: var(--accent); }
  .row input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row.on .name { font-weight: 600; }
  .aside { flex: none; font-size: var(--t-fine); color: var(--dim); }
  .none { margin: 0; padding: 8px 6px; font-size: var(--t-small); color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .row:hover { background: var(--surface-hover); } }
  @media (pointer: coarse) { .row { min-height: 44px; } .pick { min-height: 44px; } .find input { height: 44px; font-size: 16px; } }
  @media (max-width: 639.98px) { .find input { font-size: 16px; } }
</style>
