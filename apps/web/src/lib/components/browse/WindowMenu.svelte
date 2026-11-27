<script lang="ts">
  // Trending's timeframe (Day, 7 days, 30 days) as an inline menu at the end of the sort row: a
  // flat button in the row's own language (muted text, accent calendar icon, the chosen window in
  // bold, a chevron) that opens a small menu of the three windows, the chosen one bold with the
  // accent tick. A real menu: arrows, Home/End, Escape back to the button; on phones a sheet.
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { WINDOWS, type Window } from './params';

  type Props = { value: Window; onchange: (w: Window) => void };
  let { value, onchange }: Props = $props();
  let open = $state(false);
  const current = $derived(WINDOWS.find((w) => w.value === value) ?? WINDOWS[0]);

  function pick(w: Window) {
    open = false;
    if (w !== value) onchange(w);
  }
</script>

<Popover label="Trending window" kind="menu" sheet width="200px" bind:open>
  {#snippet trigger(props)}
    <button class="win" type="button" {...props} aria-label="Trending window: {current.label}">
      <Icon name="calendar" size={15} /><span class="v">{current.short}</span><Icon name="chevron-down" size={14} />
    </button>
  {/snippet}
  <div class="menu">
    {#each WINDOWS as w (w.value)}
      <button class="item" class:on={w.value === value} type="button" role="menuitemradio" aria-checked={w.value === value} tabindex="-1" onclick={() => pick(w.value)}>
        <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span><span class="lb">{w.label}</span>
      </button>
    {/each}
  </div>
</Popover>

<style>
  .win { display: inline-flex; align-items: center; gap: 7px; height: 34px; padding: 0 8px 0 12px; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--text); font: inherit; font-size: 14.5px; font-weight: 700; cursor: pointer; white-space: nowrap;
    transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .win :global(svg:first-child) { color: var(--accent-text); }
  .win :global(svg:last-child) { color: var(--dim); }
  .win:active { transform: scale(var(--press)); }
  .win:focus-visible { outline-offset: -2px; }
  .menu { display: grid; gap: 1px; }
  .item { display: flex; align-items: center; gap: 10px; min-height: 36px; padding: 0 10px 0 8px; border: 0; border-radius: var(--radius);
    background: none; color: var(--muted); font: inherit; font-size: var(--t-nav); font-weight: 500; text-align: left; cursor: pointer;
    transition: background-color var(--fast), color var(--fast); }
  .tick { display: inline-flex; color: var(--accent-text); visibility: hidden; }
  .on .tick { visibility: visible; }
  .item.on { color: var(--text); font-weight: 700; }
  .item:focus-visible { outline-offset: -2px; }
  .item:active { opacity: .6; transition: none; }
  @media (pointer: coarse) { .win { min-height: 44px; } .item { min-height: 48px; } }
  @media (hover: hover) and (pointer: fine) { .item:hover { background: var(--row-hover); color: var(--text); } }
  @media (forced-colors: active) { .item.on { outline: 1px solid Highlight; outline-offset: -2px; } }
</style>
