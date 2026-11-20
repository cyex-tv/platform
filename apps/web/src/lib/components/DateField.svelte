<script lang="ts">
  // A day filter as a button in the cyex format (2026/11/20) that opens a small popover with a
  // date field, Go and Clear, the way the profile's Activity tab picks a date. The value is
  // `YYYY-MM-DD`, or empty for "any".
  import { dayLabel } from '$lib/admin/audit';
  import Icon from './Icon.svelte';
  import Popover from './Popover.svelte';

  type Props = { label: string; value: string; onchange: (day: string) => void };
  let { label, value, onchange }: Props = $props();

  let open = $state(false);
  let day = $state('');
  const uid = $props.id();

  $effect(() => {
    if (open) day = value;
  });

  function go(e: SubmitEvent) {
    e.preventDefault();
    open = false;
    onchange(day);
  }
</script>

<div class="df">
  <span class="ui-label">{label}</span>
  <Popover label={`Pick the ${label.toLowerCase()} date`} width="260px" bind:open>
    {#snippet trigger(props)}
      <button class="ui-btn secondary date" type="button" {...props} aria-label="{label}: {dayLabel(value) || 'any date'}">
        <Icon name="calendar" size={14} /><span class="ui-num" class:none={!value}>{dayLabel(value) || 'Any date'}</span>
      </button>
    {/snippet}
    <form class="pop" onsubmit={go}>
      <label class="sr-only" for="{uid}-d">{label} date</label>
      <input id="{uid}-d" class="ui-input" type="date" bind:value={day} />
      <div class="acts">
        <button class="ui-btn ghost small" type="button" onclick={() => { open = false; onchange(''); }}>Clear</button>
        <button class="ui-btn primary small" type="submit">Go</button>
      </div>
    </form>
  </Popover>
</div>

<style>
  .df { display: grid; gap: 4px; }
  .date { justify-content: flex-start; gap: 6px; width: 100%; font-variant-numeric: tabular-nums; }
  .date :global(svg) { color: var(--muted); }
  .none { color: var(--muted); font-weight: 400; }
  .pop { display: grid; gap: 10px; padding: 12px; }
  .acts { display: flex; justify-content: flex-end; gap: 8px; }
</style>
