<script lang="ts" module>
  // When the last tooltip hid, so moving along a row of icon buttons shows the next tooltip at
  // once instead of waiting out the delay again (Emil: warm tooltips).
  let lastHide = 0;
  const WARM_MS = 500;
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
    tip?: 'bottom' | 'top' | 'none';
    pressed?: boolean;
  };
  let { label, icon, href, size = 'm', variant = 'ghost', tip = 'bottom', pressed, class: cls = '', ...rest }: Props = $props();

  let warm = $state(false);
  const enter = () => (warm = performance.now() - lastHide < WARM_MS);
  const leave = () => {
    lastHide = performance.now();
    warm = false;
  };
</script>

{#if href}
  <a class="ib {size} {variant} {cls}" class:warm {href} aria-label={label} onpointerenter={enter} onpointerleave={leave}>
    <Icon name={icon} size={size === 's' ? 16 : 18} />
    {#if tip !== 'none'}<span class="tip {tip}" aria-hidden="true">{label}</span>{/if}
  </a>
{:else}
  <button
    class="ib {size} {variant} {cls}"
    class:warm
    type="button"
    aria-label={label}
    aria-pressed={pressed}
    {...rest}
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
    {#if tip !== 'none'}<span class="tip {tip}" aria-hidden="true">{label}</span>{/if}
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
  .ib[aria-pressed='true'] { color: var(--text); background: var(--surface-2); border-color: var(--border-strong); }
  .ib:active { transform: scale(var(--press)); }
  .ib:focus-visible { color: var(--text); }

  .tip { position: absolute; left: 50%; z-index: var(--z-tip); padding: 5px 8px; border-radius: var(--radius-btn);
    background: var(--text); color: var(--bg); font-size: var(--t-fine); font-weight: 500; line-height: 1.2; white-space: nowrap;
    pointer-events: none; opacity: 0; transform: translateX(-50%) scale(.97);
    transition: opacity var(--dur-pop) ease-out, transform var(--dur-pop) ease-out; }
  .tip.bottom { top: calc(100% + 6px); transform-origin: 50% 0; }
  .tip.top { bottom: calc(100% + 6px); transform-origin: 50% 100%; }
  .ib:focus-visible .tip { opacity: 1; transform: translateX(-50%); }

  @media (hover: hover) and (pointer: fine) {
    .ib:hover { color: var(--text); background: var(--surface-2); }
    .outline:hover { border-color: var(--border-strong); }
    .ib:hover .tip { opacity: 1; transform: translateX(-50%); transition-delay: var(--delay-tip); }
    /* Warm: another tooltip just hid, so this one shows at once and without the scale. */
    .ib.warm:hover .tip { transition-delay: 0s; transition-duration: 0s; }
  }
  @media (forced-colors: active) {
    .ib[aria-pressed='true'] { outline: 2px solid Highlight; outline-offset: -2px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .tip, .ib:focus-visible .tip { transform: translateX(-50%); }
  }
</style>
