<script lang="ts" generics="T extends string">
  // A segmented control with radiogroup semantics: one tab stop, arrow keys move and select
  // (Home/End jump). The checked option is bold with an accent icon (text colour when it has
  // none): no fill, border or bar, ever (owner rule). Hover only lightens the text colour.
  // Controlled: it shows `value` and reports a pick through `onchange`; the parent sets `value`.
  // The label is set twice (the second copy bold and invisible) so picking never changes widths.
  //   <Segmented label="Sort" options={sorts} value={sort} onchange={(v) => (sort = v)} />
  import { tick } from 'svelte';
  import Icon, { type IconName } from './Icon.svelte';
  import { nextSegment } from './segmented';

  type Option = { value: T; label: string; icon?: IconName; disabled?: boolean };
  type Props = {
    options: Option[];
    value: T;
    label: string;
    size?: 's' | 'm';
    /** Option height in px, overriding the size's (34 for 's', 40 for 'm'). */
    height?: number;
    onchange: (value: T) => void;
  };
  let { options, value, label, size = 'm', height, onchange }: Props = $props();

  let root: HTMLDivElement;

  async function pick(v: T, focus = false) {
    if (v !== value) onchange(v);
    if (focus) {
      // The option asked for, by value: the parent may set `value` later (after a navigation).
      await tick();
      const i = options.findIndex((o) => o.value === v);
      root.querySelectorAll<HTMLElement>('[role="radio"]')[i]?.focus();
    }
  }

  function onkeydown(e: KeyboardEvent) {
    // From the focused option (it is the last pick even when `value` hasn't caught up yet).
    const radios = [...root.querySelectorAll<HTMLElement>('[role="radio"]')];
    const at = radios.indexOf(document.activeElement as HTMLElement);
    const i = at >= 0 ? at : options.findIndex((o) => o.value === value);
    const n = nextSegment(e.key, i, options.map((o) => !!o.disabled));
    if (n < 0) return;
    e.preventDefault();
    pick(options[n].value, true);
  }

  // The one tab stop: the checked option (or the first enabled one when none is).
  const stop = $derived(options.some((o) => o.value === value) ? value : options.find((o) => !o.disabled)?.value);
</script>

<div class="seg {size}" role="radiogroup" aria-label={label} bind:this={root} tabindex="-1" {onkeydown}
  style:--seg-h={height ? `${height}px` : undefined}>
  {#each options as o (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={o.value === value}
      tabindex={o.value === stop ? 0 : -1}
      disabled={o.disabled}
      onclick={() => pick(o.value)}
    >
      {#if o.icon}<Icon name={o.icon} size={size === 's' ? 15 : 16} />{/if}
      <span class="lb" data-t={o.label}>{o.label}</span>
    </button>
  {/each}
</div>

<style>
  .seg { position: relative; display: inline-flex; flex: none; gap: 2px; max-width: 100%; overflow-x: auto; scrollbar-width: none; }
  .seg:focus { outline: none; }
  /* 34px to see in the 's' size, 40px to hit: the hit area takes in the gap. */
  button { position: relative; display: inline-flex; align-items: center; justify-content: center; flex: none; gap: 7px;
    height: var(--seg-h, 34px); padding: 0 12px; border: 0; background: none; border-radius: var(--radius-btn);
    font: inherit; font-size: 14.5px; font-weight: 500; color: var(--muted); cursor: pointer; white-space: nowrap;
    transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .m button { height: var(--seg-h, 40px); padding: 0 14px; }
  button::after { content: ''; position: absolute; inset: -3px 0; }
  .lb { display: inline-grid; }
  .lb::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; pointer-events: none; }
  button :global(svg) { color: var(--dim); transition: color var(--dur-color); }
  button[aria-checked='true'] { color: var(--text); font-weight: 700; }
  button[aria-checked='true'] :global(svg) { color: var(--accent-text); }
  button:disabled { opacity: .45; cursor: not-allowed; }
  button:active:not(:disabled) { transform: scale(var(--press)); }
  button:focus-visible { outline-offset: -2px; }
  /* High contrast drops bold and the accent icon: the selected option keeps a system-colour outline. */
  @media (forced-colors: active) { button[aria-checked='true']:not(:focus-visible) { outline: 1px solid Highlight; outline-offset: -2px; } }
  /* Phones: tighter options so four icon + label options still fit a card without scrolling. */
  @media (max-width: 480px) { .seg { gap: 0; } button { gap: 4px; padding: 0 7px; } .m button { padding: 0 10px; } }
  @media (pointer: coarse) { button { min-height: 44px; } }
  @media (hover: hover) and (pointer: fine) { button:hover:not(:disabled) { color: var(--text); } }
</style>
