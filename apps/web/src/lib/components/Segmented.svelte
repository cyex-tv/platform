<script lang="ts" generics="T extends string">
  // A segmented control with radiogroup semantics: one tab stop, arrow keys move and select
  // (Home/End jump), the checked option carries a marker that slides between options.
  // No accent: the checked option gets the stronger surface and text (cyex-web).
  import { tick } from 'svelte';

  type Option = { value: T; label: string };
  type Props = {
    options: Option[];
    value: T;
    label: string;
    size?: 's' | 'm';
    onchange?: (value: T) => void;
  };
  let { options, value = $bindable(), label, size = 'm', onchange }: Props = $props();

  let root: HTMLDivElement;
  let marker = $state({ x: 0, w: 0, ready: false });

  function place() {
    const el = root?.querySelector<HTMLElement>('[aria-checked="true"]');
    if (!el) return;
    marker = { x: el.offsetLeft, w: el.offsetWidth, ready: true };
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

  async function pick(v: T, focus = false) {
    if (v !== value) {
      value = v;
      onchange?.(v);
    }
    if (focus) {
      await tick();
      root.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
    }
  }

  function onkeydown(e: KeyboardEvent) {
    const i = options.findIndex((o) => o.value === value);
    let n = -1;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') n = (i + 1) % options.length;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') n = (i - 1 + options.length) % options.length;
    else if (e.key === 'Home') n = 0;
    else if (e.key === 'End') n = options.length - 1;
    if (n < 0) return;
    e.preventDefault();
    pick(options[n].value, true);
  }
</script>

<div class="seg {size}" role="radiogroup" aria-label={label} bind:this={root} tabindex="-1" {onkeydown}>
  <span
    class="marker"
    class:ready={marker.ready}
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
  /* The marker appears in place first, then slides: no slide in from the left on load. */
  .marker.ready { opacity: 1; transition: transform var(--dur-move) var(--ease), width var(--dur-move) var(--ease); }
  button { position: relative; z-index: 1; display: inline-flex; align-items: center; justify-content: center; flex: none;
    height: 34px; padding: 0 14px; border: 0; background: none; border-radius: var(--radius-btn);
    font: inherit; font-size: 14.5px; font-weight: 500; color: var(--muted); cursor: pointer; white-space: nowrap;
    transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .m button { height: 40px; padding: 0 16px; }
  button[aria-checked='true'] { color: var(--text); }
  /* Before the script runs (server render), the checked option draws its own ground. */
  .seg:not(:has(.marker.ready)) button[aria-checked='true'] { background: var(--surface-2);
    box-shadow: 0 0 0 1px var(--border-strong) inset; }
  button:active { transform: scale(var(--press)); }
  button:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { button:hover { color: var(--text); } }
  @media (prefers-reduced-motion: reduce) { .marker.ready { transition: none; } }
</style>
