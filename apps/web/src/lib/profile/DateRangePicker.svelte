<script lang="ts">
  // The Activity feed's date range, for the account's owner and staff (owner rule, 2026/11/22:
  // all time or a date range, not day or month). One button in the toolbar says what is shown
  // ("All time", "2026/11/01 – 2026/11/22") and opens a small popover: the shortcuts (All time,
  // Last 7, 30 and 90 days; the one in use bold with the accent check), then From and To fields in
  // the cyex date format (either may stay empty: from the start, up to now) and Apply. A future
  // day or a backwards range is refused in a line under the fields, as cyex logs does.
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { ALL_TIME, PRESETS, dayOf, labelOf, lastDays, presetOf, rangeProblem, readDay, slashDay, type DayRange } from './range';

  type Props = { value: DayRange; onchange: (r: DayRange) => void };
  let { value, onchange }: Props = $props();

  let open = $state(false);
  let from = $state('');
  let to = $state('');
  let problem = $state('');
  const uid = $props.id();
  const current = $derived(presetOf(value));
  const shortcuts = $derived([{ days: 0, label: 'All time' }, ...PRESETS]);

  // Each opening starts from what is shown.
  $effect(() => {
    if (!open) return;
    problem = '';
    from = value.from ? slashDay(value.from) : '';
    to = value.to ? slashDay(value.to) : '';
  });

  function pick(days: number) {
    open = false;
    onchange(days ? lastDays(days) : ALL_TIME);
  }

  function apply(e: SubmitEvent) {
    e.preventDefault();
    const a = readDay(from);
    const b = readDay(to);
    if ('problem' in a || 'problem' in b) {
      problem = 'problem' in a ? `From: ${a.problem}` : `To: ${(b as { problem: string }).problem}`;
      return;
    }
    const next = { from: a.day, to: b.day };
    const why = rangeProblem(next);
    if (why) {
      problem = why;
      return;
    }
    open = false;
    onchange(next);
  }
</script>

<Popover label="Date range" width="280px" align="end" sheet bind:open>
  {#snippet trigger(props)}
    <button class="ui-btn secondary small trig" type="button" {...props} data-testid="activity-range">
      <Icon name="calendar" size={14} /><span class="ui-num">{labelOf(value)}</span><Icon name="chevron-down" size={14} />
    </button>
  {/snippet}
  <div class="pop">
    <p class="pop-title">Date range</p>
    <ul class="presets">
      {#each shortcuts as p (p.days)}
        <li>
          <button type="button" class="preset" aria-pressed={current === p.days} onclick={() => pick(p.days)}>
            <span class="tick" aria-hidden="true">{#if current === p.days}<Icon name="check" size={14} />{/if}</span>{p.label}
          </button>
        </li>
      {/each}
    </ul>
    <form class="custom" onsubmit={apply} novalidate>
      <div class="row">
        <label for="{uid}-f">From</label>
        <input id="{uid}-f" class="ui-input" type="text" inputmode="numeric" placeholder="2026/11/01" autocomplete="off" spellcheck="false" bind:value={from}
          aria-invalid={!!problem} aria-describedby={problem ? `${uid}-p` : undefined} />
      </div>
      <div class="row">
        <label for="{uid}-t">To</label>
        <input id="{uid}-t" class="ui-input" type="text" inputmode="numeric" placeholder={slashDay(dayOf(new Date()))} autocomplete="off" spellcheck="false" bind:value={to}
          aria-invalid={!!problem} aria-describedby={problem ? `${uid}-p` : undefined} />
      </div>
      {#if problem}<p class="problem" id="{uid}-p" role="alert">{problem}</p>{/if}
      <button class="ui-btn primary small go" type="submit">Apply</button>
    </form>
  </div>
</Popover>

<style>
  /* As tall as the toolbar's other controls, its text the same size: one line, one centre. */
  .ui-btn.trig { gap: 6px; height: var(--ctl-h, 34px); font-size: var(--t-small); font-variant-numeric: tabular-nums; }
  .trig :global(svg) { color: var(--muted); }
  .pop { display: grid; gap: 8px; padding: 12px; }
  .pop-title { margin: 0; font-size: var(--t-label, .7rem); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--muted); }
  .presets { list-style: none; margin: 0 -4px; padding: 0; display: grid; }
  .preset { display: flex; align-items: center; gap: 6px; width: 100%; min-height: 32px; padding: 0 4px; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--text); font: inherit; font-size: var(--t-small); text-align: left; cursor: pointer; }
  .preset:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .preset:hover { background: var(--surface-2); } }
  /* The range in use: bold with the accent check (owner rule: no fill, no bar). */
  .preset[aria-pressed='true'] { font-weight: 700; }
  .tick { display: inline-grid; place-items: center; width: 14px; color: var(--accent-text); }
  .custom { display: grid; gap: 8px; padding-top: 10px; border-top: 1px solid var(--border); }
  .row { display: grid; grid-template-columns: 44px 1fr; align-items: center; gap: 8px; }
  .row label { font-size: var(--t-fine); color: var(--muted); }
  .problem { margin: 0; font-size: var(--t-fine); color: var(--red); }
  .go { width: 100%; justify-content: center; }
  @media (max-width: 719.98px) {
    .preset { min-height: 44px; }
    .row input { font-size: 16px; min-height: 44px; }
    .ui-btn.go { min-height: 44px; }
    /* The sheet's own head already says Date range. */
    .pop-title { display: none; }
  }
</style>
