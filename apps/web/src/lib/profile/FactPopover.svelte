<script lang="ts">
  // One collapsed fact of the profile band (Connections, Editors, more roles): a quiet chip with an
  // icon, a word and its count that opens a small panel on purpose (owner, 2026/11/20). The panel
  // is the shared Popover as a dialog: focus moves into it, Escape or a click outside closes it and
  // focus goes back to the chip; phones get the same content in a bottom sheet. The chip has the
  // same size whatever the count, so the band never grows with the list.
  import type { Snippet } from 'svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { count as fmt } from '$lib/format';

  type Props = {
    /** The panel's name (and the chip's accessible name with the count). */
    label: string;
    /** The chip's word, e.g. "Editors"; leave out for a bare "+N". */
    word?: string;
    icon?: IconName;
    count: number;
    children: Snippet;
    footer?: Snippet;
    width?: string;
  };
  let { label, word, icon, count, children, footer, width = '320px' }: Props = $props();
</script>

<Popover {label} sheet {width} {footer}>
  {#snippet trigger(props)}
    <button class="fact" type="button" {...props} aria-label="{label}, {fmt(count)}">
      {#if icon}<Icon name={icon} size={14} />{/if}
      {#if word}<span class="w">{word}</span><span class="n">{fmt(count)}</span>{:else}<span class="w">+{fmt(count)}</span>{/if}
      <Icon name="chevron-down" size={12} />
    </button>
  {/snippet}
  {@render children()}
</Popover>

<style>
  .fact { display: inline-flex; align-items: center; gap: 6px; height: 28px; padding: 0 8px; border: 1px solid var(--border);
    border-radius: var(--radius-btn); background: none; color: var(--text); font: inherit; font-size: var(--t-fine); font-weight: 500;
    cursor: pointer; white-space: nowrap; transition: border-color var(--fast), background-color var(--fast); }
  .fact > :global(svg) { flex: none; color: var(--dim); }
  .fact > :global(svg:last-child) { transition: rotate var(--fast) var(--ease); }
  .fact[aria-expanded='true'] { border-color: var(--border-strong); }
  .fact[aria-expanded='true'] > :global(svg:last-child) { rotate: 180deg; }
  .n { color: var(--muted); font-variant-numeric: tabular-nums; }
  .fact:active { opacity: .6; transition: none; }
  @media (hover: hover) and (pointer: fine) { .fact:hover { border-color: var(--border-strong); background: var(--row-hover); } }
  @media (pointer: coarse) { .fact { height: 36px; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .fact > :global(svg:last-child) { transition: none; } }
</style>
