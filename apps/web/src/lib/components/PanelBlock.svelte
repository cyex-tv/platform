<script lang="ts">
  // One block of an Inspector panel (InspectorLayout, the profile's About): an uppercase label,
  // an optional count and link at its right, then the block. Blocks after the first get a hairline
  // above. `fold` turns the label into a disclosure (shut until opened, like About's Connections),
  // for lists that can grow. Inside, `<dl class="kv">` is the panel's facts list (label left,
  // value right, tabular numbers) and `.pb-actions` a column of full-width buttons.
  //   <PanelBlock title="Facts"><dl class="kv"><dt>Emotes</dt><dd>24</dd></dl></PanelBlock>
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  type Props = {
    title: string;
    count?: string | number;
    /** A link at the label's right ("Manage"). */
    more?: { href: string; label: string };
    fold?: boolean;
    /** Open at first (with `fold`). */
    open?: boolean;
    /** The heading level of the label (3 under a panel whose own label is the h2). */
    level?: 2 | 3;
    children: Snippet;
  };
  let { title, count, more, fold = false, open = $bindable(false), level = 3, children }: Props = $props();
  const id = $props.id();
</script>

<section class="pb" aria-labelledby="{id}-h">
  <svelte:element this={level === 2 ? 'h2' : 'h3'} class="pb-head" id="{id}-h">
    {#if fold}
      <button class="pb-dis" type="button" aria-expanded={open} aria-controls="{id}-b" onclick={() => (open = !open)}>
        <span class="pb-chev" class:on={open} aria-hidden="true"><Icon name="chevron-right" size={14} /></span>
        <span class="pb-label">{title}</span>{#if count !== undefined}<span class="pb-n">{count}</span>{/if}
      </button>
    {:else}
      <span class="pb-label">{title}</span>{#if count !== undefined}<span class="pb-n">{count}</span>{/if}
    {/if}
    {#if more}<a class="pb-more" href={more.href}>{more.label}<Icon name="chevron-right" size={14} /></a>{/if}
  </svelte:element>
  <div class="pb-body" id="{id}-b" hidden={fold && !open}>{@render children()}</div>
</section>

<style>
  .pb { display: grid; min-width: 0; padding-bottom: 8px; }
  :global(.pb + .pb) { padding-top: 8px; border-top: 1px solid var(--border); }
  .pb-head { display: flex; align-items: center; gap: 6px; min-height: 36px; margin: 0; font-size: inherit; font-weight: inherit; letter-spacing: 0; }
  .pb-label { font-size: var(--t-label, 12px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .pb-n { font-size: var(--t-small); font-weight: 500; color: var(--muted); font-variant-numeric: tabular-nums; }
  .pb-dis { display: flex; align-items: center; gap: 6px; flex: 1; min-width: 0; min-height: 36px; margin: 0 -6px; padding: 0 6px; border: 0;
    border-radius: var(--radius); background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .pb-dis .pb-n { margin-left: auto; }
  .pb-dis[aria-expanded='true'] .pb-label { color: var(--text); }
  .pb-dis:focus-visible { outline-offset: -2px; }
  .pb-chev { display: inline-flex; color: var(--dim); transition: transform var(--fast) var(--ease); }
  .pb-chev.on { transform: rotate(90deg); color: var(--accent-text); }
  .pb-more { display: inline-flex; align-items: center; gap: 2px; min-height: 32px; margin-left: auto; font-size: var(--t-small); font-weight: 500;
    color: var(--muted); text-decoration: none; }
  .pb-body { display: grid; gap: 10px; min-width: 0; }
  .pb-body[hidden] { display: none; }
  .pb-body :global(p) { margin: 0; }

  /* The facts list: label left, value right, a hairline between rows. */
  .pb-body :global(dl.kv) { display: grid; grid-template-columns: minmax(0, 1fr) auto; margin: 0; }
  .pb-body :global(dl.kv > dt), .pb-body :global(dl.kv > dd) { margin: 0; padding: 9px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); line-height: 1.4; }
  .pb-body :global(dl.kv > dt) { color: var(--muted); padding-right: 16px; }
  .pb-body :global(dl.kv > dd) { min-width: 0; text-align: right; font-weight: 600; font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
  .pb-body :global(dl.kv > dt:last-of-type), .pb-body :global(dl.kv > dd:last-of-type) { border-bottom: 0; }
  /* A column of buttons, each the panel's width. */
  .pb-body :global(.pb-actions) { display: grid; gap: 8px; }
  .pb-body :global(.pb-actions > *) { width: 100%; }
  .pb-body :global(.pb-actions .pair) { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .pb-body :global(.pb-actions .pair > *) { width: 100%; min-width: 0; }
  /* A hint under a block. */
  .pb-body :global(.pb-hint) { font-size: var(--t-small); line-height: 1.5; color: var(--muted); text-wrap: pretty; }

  @media (hover: hover) and (pointer: fine) {
    .pb-dis:hover { background: var(--row-hover); }
    .pb-more:hover { color: var(--text); }
  }
  @media (pointer: coarse) { .pb-dis, .pb-more { min-height: 44px; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .pb-chev { transition: none; } }
</style>
