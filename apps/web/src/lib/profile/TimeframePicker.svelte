<script lang="ts">
  // The activity feed's timeframe and date, the cyex logs way: a timeframe switch (All time, Day,
  // Month) and the date beside it as a button in the cyex format (2026/11/19, 2026/11, or the span
  // of what has loaded). The button opens a small popover, "Jump to date" (a date field) or "Jump
  // to month" (a year and a month), with Go; a future date is refused in a line under the field,
  // as logs does. All time has nothing to jump to: picking a date there switches to that day.
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import { dayOf, dayProblem, frameTo, labelOf, monthProblem, type Frame, type Timeframe } from './timeframe';

  type Props = { value: Timeframe; shown?: readonly string[]; onchange: (t: Timeframe) => void };
  let { value, shown = [], onchange }: Props = $props();

  const FRAMES: { value: Frame; label: string }[] = [
    { value: 'all', label: 'All time' },
    { value: 'day', label: 'Day' },
    { value: 'month', label: 'Month' },
  ];
  const MONTHS = Array.from({ length: 12 }, (_, i) => new Date(2000, i, 1).toLocaleString('en', { month: 'long' }));

  let open = $state(false);
  let day = $state('');
  let year = $state(0);
  let month = $state(1);
  let problem = $state('');
  const uid = $props.id();
  const monthly = $derived(value.frame === 'month');

  // Each opening starts from what is shown.
  $effect(() => {
    if (!open) return;
    const now = new Date();
    problem = '';
    day = value.frame === 'day' ? value.day : dayOf(now);
    year = value.frame === 'month' ? value.year : now.getFullYear();
    month = value.frame === 'month' ? value.month : now.getMonth() + 1;
  });

  function go(e: SubmitEvent) {
    e.preventDefault();
    const why = monthly ? monthProblem(Number(year), Number(month)) : dayProblem(day);
    if (why) {
      problem = why;
      return;
    }
    open = false;
    onchange(monthly ? { frame: 'month', year: Number(year), month: Number(month) } : { frame: 'day', day });
  }
</script>

<div class="tf">
  <Segmented label="Timeframe" size="s" options={FRAMES} value={value.frame} onchange={(f) => onchange(frameTo(f))} />
  <Popover label={monthly ? 'Jump to month' : 'Jump to date'} width="260px" bind:open>
    {#snippet trigger(props)}
      <button class="ui-btn secondary small date" type="button" {...props} title={monthly ? 'Jump to a different month' : 'Jump to a different date'}>
        <Icon name="calendar" size={14} /><span class="ui-num">{labelOf(value, shown)}</span>
      </button>
    {/snippet}
    <form class="pop" onsubmit={go} novalidate>
      <p class="pop-title">{monthly ? 'Jump to month' : 'Jump to date'}</p>
      {#if monthly}
        <div class="row">
          <label class="sr-only" for="{uid}-y">Year</label>
          <input id="{uid}-y" class="ui-input year" type="number" inputmode="numeric" min="2026" max={new Date().getFullYear()} bind:value={year}
            aria-invalid={!!problem} aria-describedby={problem ? `${uid}-p` : undefined} />
          <label class="sr-only" for="{uid}-m">Month</label>
          <select id="{uid}-m" class="ui-input" bind:value={month}>
            {#each MONTHS as name, i (i)}<option value={i + 1}>{name}</option>{/each}
          </select>
        </div>
      {:else}
        <label class="sr-only" for="{uid}-d">Date</label>
        <input id="{uid}-d" class="ui-input" type="date" max={dayOf(new Date())} bind:value={day}
          aria-invalid={!!problem} aria-describedby={problem ? `${uid}-p` : undefined} />
      {/if}
      {#if problem}<p class="problem" id="{uid}-p" role="alert">{problem}</p>{/if}
      <button class="ui-btn primary small go" type="submit">Go</button>
    </form>
  </Popover>
</div>

<style>
  .tf { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .date { gap: 6px; font-variant-numeric: tabular-nums; }
  .date :global(svg) { color: var(--muted); }
  .pop { display: grid; gap: 10px; padding: 12px; }
  .pop-title { margin: 0; font-size: var(--t-ui); font-weight: 600; }
  .row { display: flex; gap: 8px; }
  .year { width: 96px; flex: none; }
  .row select { flex: 1; min-width: 0; }
  .problem { margin: 0; font-size: var(--t-fine); color: var(--red); }
  .go { width: 100%; justify-content: center; }
</style>
