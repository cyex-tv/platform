<script lang="ts">
  // A set's capacity: a number field with the allowed range under it, saved with updateSet. The
  // range comes from setCapacityRange (the owner's role limit, staff up to 1.000.000, never below
  // what the set holds). Used in the set editor's panel and in the right-click "Change capacity".
  //   <CapacityForm setId={id} capacity={set.capacity} range={range} onsaved={(n) => …} />
  import { ApiError, gql } from '$lib/api';
  import { count } from '$lib/format';
  import { UPDATE_SET } from '$lib/queries';
  import { capacityProblem, capacityValue, rangeHint, type CapacityRange } from './capacity';

  type Props = {
    setId: string;
    capacity: number;
    range: CapacityRange;
    /** In a dialog: autofocus, and the dialog's own footer holds Cancel. */
    compact?: boolean;
    onsaved: (capacity: number) => void;
    oncancel?: () => void;
  };
  let { setId, capacity, range, compact = false, onsaved, oncancel }: Props = $props();
  const uid = $props.id();

  let raw = $state('');
  let seeded = '';
  $effect(() => {
    const key = `${setId}:${capacity}`;
    if (seeded !== key) {
      seeded = key;
      raw = String(capacity);
    }
  });
  let tried = $state(false);
  let busy = $state(false);
  let failure = $state('');
  const problem = $derived(capacityProblem(raw, range));
  const value = $derived(capacityValue(raw));
  const same = $derived(value === capacity);
  const shownError = $derived((tried || (raw.trim() !== '' && !same)) && problem ? problem : failure);

  async function save(e: SubmitEvent) {
    e.preventDefault();
    tried = true;
    failure = '';
    if (busy || problem || value == null) return;
    if (same) {
      onsaved(capacity);
      return;
    }
    busy = true;
    try {
      const r = await gql(UPDATE_SET, { id: setId, input: { capacity: value } });
      onsaved(r.updateSet.capacity);
    } catch (err) {
      failure = err instanceof ApiError && err.message ? err.message : 'That didn’t work. Try again.';
    } finally {
      busy = false;
    }
  }
</script>

<form class="cap-form" class:compact onsubmit={save} novalidate>
  <label class="sr-only" for="{uid}-cap">Capacity</label>
  <div class="row">
    <input id="{uid}-cap" class="ui-input ui-num" inputmode="numeric" autocomplete="off" bind:value={raw}
      aria-invalid={shownError ? 'true' : undefined} aria-describedby="{uid}-hint{shownError ? ` ${uid}-err` : ''}" />
    <span class="unit">slots</span>
    {#if compact && oncancel}<button class="ui-btn secondary" type="button" onclick={oncancel}>Cancel</button>{/if}
    <button class="ui-btn {compact ? 'primary' : 'secondary'}" type="submit" disabled={busy || (!compact && same)}>{busy ? 'Saving…' : 'Save'}</button>
  </div>
  <p class="ui-hint" id="{uid}-hint">{rangeHint(range)}</p>
  {#if shownError}<p class="ui-error" id="{uid}-err" role="alert">{shownError}</p>{/if}
  {#if !compact && range.max > 0}<span class="sr-only">Now {count(capacity)}.</span>{/if}
</form>

<style>
  .cap-form { display: grid; gap: 6px; }
  p { margin: 0; }
  .row { display: flex; align-items: center; gap: 8px; }
  .row input { flex: 1 1 120px; min-width: 0; font-variant-numeric: tabular-nums; }
  .unit { flex: none; font-size: var(--t-small); color: var(--muted); }
  .compact .row input { flex-basis: 140px; }
</style>
