<script lang="ts" generics="T extends string">
  // A segmented control with radiogroup semantics: one tab stop, arrow keys move and select
  // (Home/End jump), the checked option carries a marker that slides between options.
  // No accent: the checked option gets the stronger surface and text (cyex-web).
  // Controlled: it shows `value` and reports a pick through `onchange`; the parent sets `value`.
  // The marker glides only after a pick; placement on load, resize or a font swap is instant.
  //   <Segmented label="Sort" options={sorts} value={sort} onchange={(v) => (sort = v)} />
  import { tick, untrack } from 'svelte';
  import { still } from '$lib/motion';

  type Option = { value: T; label: string };
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
  let marker = $state({ x: 0, w: 0, ready: false });
  let glide = $state(false);

  function place() {
    const el = root?.querySelector<HTMLElement>('[aria-checked="true"]');
    if (!el) return;
    const x = el.offsetLeft;
    const w = el.offsetWidth;
    const prev = untrack(() => marker);
    if (x === prev.x && w === prev.w) glide = false; // nothing to travel, no transitionend coming
    marker = { x, w, ready: true };
  }

  $effect(() => {
    void value;
    void options.length;
    place();
  });

  $effect(() => {
    const ro = new ResizeObserver(place);
    ro.observe(root);
    return () => ro.disconnect();
  });

  // `slide`: false for a held arrow key (key repeat), so the marker jumps instead of chasing.
  async function pick(v: T, focus = false, slide = true) {
    if (v !== value) {
      glide = slide && marker.ready && !still();
      onchange(v);
    }
    if (focus) {
      // The option asked for, by value: the parent may set `value` later (after a navigation),
      // so the checked option can still be the old one here.
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
    let n = -1;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') n = (i + 1) % options.length;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') n = (i - 1 + options.length) % options.length;
    else if (e.key === 'Home') n = 0;
    else if (e.key === 'End') n = options.length - 1;
    if (n < 0) return;
    e.preventDefault();
    pick(options[n].value, true, !e.repeat);
  }
</script>

<div class="seg {size}" role="radiogroup" aria-label={label} bind:this={root} tabindex="-1" {onkeydown}
  style:--seg-h={height ? `${height}px` : undefined}>
  <span
    class="marker"
    class:ready={marker.ready}
    class:glide
    ontransitionend={(e) => {
      if (e.target === e.currentTarget) glide = false;
    }}
    style:width="{marker.w}px"
    style:transform="translateX({marker.x}px)"
    aria-hidden="true"
  ></span>
  {#each options as o (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={o.value === value}
      tabindex={o.value === value ? 0 : -1}
      onclick={() => pick(o.value)}>{o.label}</button
    >
  {/each}
</div>

<style>
  .seg { position: relative; display: inline-flex; flex: none; padding: 3px; border: 1px solid var(--border);
    background: var(--surface); border-radius: calc(var(--radius-btn) + 3px); max-width: 100%; overflow-x: auto;
    scrollbar-width: none; }
  .seg:focus { outline: none; }
  .marker { position: absolute; top: 3px; bottom: 3px; left: 0; border-radius: var(--radius-btn);
    background: var(--surface-2); box-shadow: 0 0 0 1px var(--border-strong) inset; opacity: 0; }
  /* The marker appears in place first and only slides after a pick (.glide), so loading,
     resizing or a font swap never animates it. */
  .marker.ready { opacity: 1; }
  .marker.glide { transition: transform var(--dur-move) var(--ease), width var(--dur-move) var(--ease); }
  /* 34px to see in the 's' size, 40px to hit: the hit area takes in the 3px frame. */
  button::after { content: ''; position: absolute; inset: -3px 0; }
  button { position: relative; z-index: 1; display: inline-flex; align-items: center; justify-content: center; flex: none;
    height: var(--seg-h, 34px); padding: 0 14px; border: 0; background: none; border-radius: var(--radius-btn);
    font: inherit; font-size: 14.5px; font-weight: 500; color: var(--muted); cursor: pointer; white-space: nowrap;
    transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .m button { height: var(--seg-h, 40px); padding: 0 16px; }
  button[aria-checked='true'] { color: var(--text); }
  /* Before the script runs (server render), the checked option draws its own ground. */
  .seg:not(:has(.marker.ready)) button[aria-checked='true'] { background: var(--surface-2);
    box-shadow: 0 0 0 1px var(--border-strong) inset; }
  button:active { transform: scale(var(--press)); }
  @media (pointer: coarse) { button { min-height: 44px; } }
  button:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { button:hover { color: var(--text); } }
  /* Forced colours drop the marker's ground: outline the checked option instead. */
  @media (forced-colors: active) {
    .marker { display: none; }
    button[aria-checked='true'] { outline: 2px solid Highlight; outline-offset: -2px; }
  }
  @media (prefers-reduced-motion: reduce) { .marker.glide { transition: none; } }
</style>
