<script lang="ts" module>
  // When the last tooltip hid, so moving along a row of icon buttons shows the next tooltip at
  // once instead of waiting out the delay again (Emil: warm tooltips).
  let lastHide = 0;
  const WARM_MS = 500;
  /** The gap a tooltip keeps from the window's edge. */
  const EDGE = 4;
  /** How far (px) a tooltip centred on a button must move sideways to stay inside the window. */
  export function tipShift(buttonLeft: number, buttonWidth: number, tipWidth: number, windowWidth: number): number {
    const left = buttonLeft + buttonWidth / 2 - tipWidth / 2;
    const right = left + tipWidth;
    if (tipWidth + 2 * EDGE >= windowWidth) return 0;
    if (left < EDGE) return EDGE - left;
    if (right > windowWidth - EDGE) return windowWidth - EDGE - right;
    return 0;
  }
</script>

<script lang="ts">
  // An icon-only control: `label` is its accessible name and its tooltip (shown on mouse hover
  // after a short delay, at once within half a second of another one hiding, and at once on
  // keyboard focus). A link when `href` is set. Touch screens get a 44px target.
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon, { type IconName } from './Icon.svelte';

  type Props = Omit<HTMLButtonAttributes, 'children'> & {
    label: string;
    icon: IconName;
    href?: string;
    size?: 's' | 'm';
    variant?: 'ghost' | 'outline';
    /** Where the tooltip shows; `bottom-end` lines it up with the button's right edge (a button at the window's edge). */
    tip?: 'bottom' | 'bottom-end' | 'top' | 'none';
    pressed?: boolean;
  };
  let { label, icon, href, size = 'm', variant = 'ghost', tip = 'bottom', pressed, class: cls = '', ...rest }: Props = $props();

  let warm = $state(false);
  let shift = $state(0);
  let tipEl: HTMLElement | undefined = $state();
  let host: HTMLElement | undefined = $state();
  /** Keeps a centred tooltip inside the window: a button at the edge would push the page wider. */
  const fit = () => {
    if (!tipEl || !host || tip === 'bottom-end') return;
    const b = host.getBoundingClientRect();
    shift = tipShift(b.left, b.width, tipEl.offsetWidth, document.documentElement.clientWidth);
  };
  // A hidden tooltip still takes room: at the window's edge it would widen the page, so fit it as
  // soon as it exists and again when the window changes size.
  $effect(() => {
    if (tipEl && host) fit();
  });
  const enter = () => {
    fit();
    warm = performance.now() - lastHide < WARM_MS;
  };
  const leave = () => {
    lastHide = performance.now();
    warm = false;
  };
</script>

<svelte:window onresize={fit} />

{#if href}
  <a class="ib {size} {variant} {cls}" class:warm {href} aria-label={label} bind:this={host} style:--tip-x="{shift}px" onpointerenter={enter} onfocus={fit} onpointerleave={leave}>
    <Icon name={icon} size={size === 's' ? 16 : 18} />
    {#if tip !== 'none'}<span class="tip {tip}" aria-hidden="true" bind:this={tipEl}>{label}</span>{/if}
  </a>
{:else}
  <button
    class="ib {size} {variant} {cls}"
    class:warm
    bind:this={host}
    style:--tip-x="{shift}px"
    type="button"
    aria-label={label}
    aria-pressed={pressed}
    {...rest}
    onfocus={(e) => {
      fit();
      rest.onfocus?.(e);
    }}
    onpointerenter={(e) => {
      enter();
      rest.onpointerenter?.(e);
    }}
    onpointerleave={(e) => {
      leave();
      rest.onpointerleave?.(e);
    }}
  >
    <Icon name={icon} size={size === 's' ? 16 : 18} />
    {#if tip !== 'none'}<span class="tip {tip}" aria-hidden="true" bind:this={tipEl}>{label}</span>{/if}
  </button>
{/if}

<style>
  .ib { position: relative; flex: none; display: inline-grid; place-items: center; padding: 0;
    border-radius: var(--radius-btn); border: 1px solid transparent; background: transparent; color: var(--muted);
    cursor: pointer; font: inherit;
    transition: color var(--dur-color), border-color var(--dur-color), background-color var(--dur-color),
      transform var(--dur-press) ease-out; }
  .m { width: 40px; height: 40px; }
  .s { width: 32px; height: 32px; }
  .s::after { content: ''; position: absolute; inset: -4px; } /* 40px to hit, 32px to see */
  /* Touch: 44px whatever the size (mobile-native). */
  @media (pointer: coarse) { .m, .s { width: 44px; height: 44px; } .s::after { inset: 0; } }
  .outline { border-color: var(--border); }
  .ib[aria-pressed='true'] { color: var(--accent-text); } /* pressed = the accent icon: no fill, border change or bar */
  .ib:active { transform: scale(var(--press)); }
  .ib:focus-visible { color: var(--text); }
  /* High contrast drops the accent icon: a pressed button keeps a system-colour outline. */
  @media (forced-colors: active) { .ib[aria-pressed='true']:not(:focus-visible) { outline: 1px solid Highlight; outline-offset: -2px; } }

  .tip { position: absolute; left: 50%; z-index: var(--z-tip); padding: 5px 8px; border-radius: var(--radius-btn);
    background: var(--text); color: var(--bg); font-size: var(--t-fine); font-weight: 500; line-height: 1.2; white-space: nowrap;
    pointer-events: none; opacity: 0; transform: translateX(calc(-50% + var(--tip-x, 0px))) scale(.97);
    transition: opacity var(--dur-pop) ease-out, transform var(--dur-pop) ease-out; }
  .tip.bottom { top: calc(100% + 6px); transform-origin: 50% 0; }
  .tip.top { bottom: calc(100% + 6px); transform-origin: 50% 100%; }
  /* At the right edge of a row: lined up with the button's right edge, so it never runs past the window. */
  .tip.bottom-end { top: calc(100% + 6px); left: auto; right: 0; transform: scale(.97); transform-origin: 100% 0; }
  .ib:focus-visible .tip { opacity: 1; transform: translateX(calc(-50% + var(--tip-x, 0px))); }
  .ib:focus-visible .tip.bottom-end { transform: none; }

  @media (hover: hover) and (pointer: fine) {
    .ib:hover { color: var(--text); }
    .ib[aria-pressed='true']:hover { color: var(--accent-text); }
    .outline:hover { border-color: var(--border-strong); }
    .ib:hover .tip { opacity: 1; transform: translateX(calc(-50% + var(--tip-x, 0px))); transition-delay: var(--delay-tip); }
    .ib:hover .tip.bottom-end { transform: none; }
    /* Warm: another tooltip just hid, so this one shows at once and without the scale. */
    .ib.warm:hover .tip { transition-delay: 0s; transition-duration: 0s; }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(html:not([data-motion='full'])) .tip, :global(html:not([data-motion='full'])) .ib:focus-visible .tip { transform: translateX(calc(-50% + var(--tip-x, 0px))); }
    :global(html:not([data-motion='full'])) .tip.bottom-end { transform: none; }
  }
</style>
