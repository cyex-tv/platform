<script lang="ts">
  // The Workspace (redesign round 2, 2026/11/21): the browse page's shape for every page that has
  // groups or sections. A rail flush with the window's left edge and the header (a SideRail with
  // `fill`: groups with counts and set meters), the content in the middle (head, a slim Toolbar,
  // a contact sheet or flat Sections), and on the right an optional inspector panel (the
  // picked item or the page's facts), flush with the right edge on its own ground, sticky, with
  // an uppercase label in its head, the same panel as InspectorLayout.
  //   <WorkspaceLayout title="Selected" {rail} {panel}>
  //     {#snippet head()}<PageHead … />{/snippet}
  //     {#snippet toolbar()}<Toolbar …/>{/snippet}
  //     …content…
  //   </WorkspaceLayout>
  // 900-1099px: rail and content; the panel sits flat (hairlines, no card) before the content, or
  // after it with `narrow="after"`, folded behind its label with `fold`. Under 900px the rail is
  // its tab strip (SideRail) and everything is one column.
  import type { Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';

  type Props = {
    /** The rail: a SideRail with `fill`. */
    rail: Snippet;
    head?: Snippet;
    /** The slim bar over the content (a Toolbar). */
    toolbar?: Snippet;
    children: Snippet;
    /** The right panel; none: the content takes the room. */
    panel?: Snippet;
    /** The panel's visible label (uppercase, in its sticky head). */
    title?: string;
    /** The panel's accessible name (defaults to the title). */
    label?: string;
    /** Buttons in the panel's head, at the right (a close button). */
    panelActions?: Snippet;
    narrow?: 'before' | 'after';
    fold?: boolean;
    /** Panel width on wide screens. */
    width?: number;
    testid?: string;
  };
  let { rail, head, toolbar, children, panel, title = '', label, panelActions, narrow = 'before', fold = false, width = 360, testid }: Props = $props();
  let open = $state(false);
  const id = $props.id();
</script>

<div class="wrap ws" class:has-panel={!!panel} class:after={narrow === 'after'} style:--ip-w="{width}px">
  {@render rail()}
  {#if head}<div class="ws-head">{@render head()}</div>{/if}
  <div class="ws-main">
    {#if toolbar}<div class="ws-bar">{@render toolbar()}</div>{/if}
    {@render children()}
  </div>
  {#if panel}
    <aside class="ws-panel" aria-label={label ?? title} data-testid={testid}>
      <div class="ws-top">
        <h2 class="ws-label" id="{id}-h">{title}</h2>
        {#if fold}
          <button class="ws-toggle" type="button" aria-expanded={open} aria-controls="{id}-b" onclick={() => (open = !open)}>
            <span class="ws-word">{title}</span><span class="sr-only">{label ?? title}</span><span aria-hidden="true">{open ? 'Hide' : 'Show'}</span><Icon name="chevron-down" size={14} />
          </button>
        {/if}
        {#if panelActions}<div class="ws-acts">{@render panelActions()}</div>{/if}
      </div>
      <div class="ws-body" id="{id}-b" class:shut={fold && !open}>{@render panel()}</div>
    </aside>
  {/if}
</div>

<style>
  .ws { display: grid; grid-template-columns: var(--rail-w) minmax(0, 1fr); grid-template-areas: 'rail head' 'rail main'; grid-template-rows: auto 1fr;
    column-gap: 28px; align-items: start; padding-top: var(--rail-gap); }
  .ws.has-panel { grid-template-columns: var(--rail-w) minmax(0, 1fr) var(--ip-w); grid-template-areas: 'rail head panel' 'rail main panel'; }
  .ws > :global(aside.rail) { grid-area: rail; }
  .ws > :global(.strip), .ws > :global(.sheet-bar) { grid-column: 1 / -1; }
  .ws-head { grid-area: head; min-width: 0; }
  .ws-head > :global(.head) { margin-bottom: 8px; }
  .ws-main { grid-area: main; min-width: 0; padding-bottom: 48px; }
  .ws-panel { grid-area: panel; align-self: start; position: sticky; top: var(--nav-h); height: calc(100svh - var(--nav-h)); overflow-y: auto;
    overscroll-behavior: contain; margin: calc(-1 * var(--rail-gap)) calc(-1 * var(--page-pad)) 0 0; border-left: 1px solid var(--border);
    background: var(--surface);
    /* Past the 1520px wrap the panel's surface runs on to the window's edge (its content stays put). */
    box-shadow: 0 0 0 100vw var(--surface); clip-path: inset(0 -100vw 0 0); }
  .ws-top { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; gap: 8px; height: 48px; padding: 0 8px 0 16px;
    border-bottom: 1px solid var(--border); background: var(--surface); }
  .ws-label { margin: 0; font-size: var(--t-label, 12px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); line-height: 1; }
  .ws-acts { display: flex; align-items: center; gap: 4px; margin-left: auto; }
  .ws-body { padding: 8px 16px 16px; }
  .ws-toggle { display: none; }

  /* Rail and content: the panel lies flat in the content's column (hairlines, no card). */
  @media (max-width: 1099.98px) {
    .ws.has-panel { grid-template-columns: var(--rail-w) minmax(0, 1fr); grid-template-areas: 'rail head' 'rail panel' 'rail main'; grid-template-rows: auto auto 1fr; }
    .ws.has-panel.after { grid-template-areas: 'rail head' 'rail main' 'rail panel'; grid-template-rows: auto 1fr auto; }
    .ws-panel { position: static; height: auto; overflow: visible; margin: 0 0 20px; border: 0; border-block: 1px solid var(--border);
      background: none; box-shadow: none; clip-path: none; }
    .after .ws-panel { margin: 0 0 48px; }
    .after .ws-main { padding-bottom: 20px; }
    .ws-top { position: static; height: 44px; padding: 0; background: none; border-bottom: 0; }
    .ws-body { padding: 0 0 12px; }
    .ws-panel:has(.ws-toggle) .ws-label { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
    .ws-toggle { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; min-height: 44px; padding: 0;
      border: 0; background: none; color: var(--text); font: inherit; font-size: var(--t-small); font-weight: 600; cursor: pointer; }
    .ws-word { margin-right: auto; font-size: var(--t-label, 12px); letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
    .ws-toggle :global(svg) { color: var(--dim); transition: transform var(--fast) var(--ease); }
    .ws-toggle[aria-expanded='true'] :global(svg) { transform: rotate(180deg); }
    .ws-toggle:focus-visible { outline-offset: 2px; }
    .ws-body.shut { display: none; }
  }
  /* Phones: the rail is its tab strip (SideRail), one column: head, panel, content (or content, panel). */
  @media (max-width: 899.98px) {
    .ws, .ws.has-panel, .ws.has-panel.after { grid-template-columns: minmax(0, 1fr); grid-template-areas: none; grid-template-rows: none; padding-top: 0; }
    .ws-head, .ws-main, .ws-panel { grid-area: auto; }
    .ws-head { padding-top: var(--rail-gap); }
    /* No head: the first block keeps the one page top under the strip. */
    .ws:not(:has(> .ws-head)) > .ws-main { padding-top: var(--rail-gap); }
    .ws.has-panel:not(.after):not(:has(> .ws-head)) > .ws-main { padding-top: 0; }
    .ws.has-panel:not(.after):not(:has(> .ws-head)) > .ws-panel { margin-top: var(--rail-gap); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .ws-toggle :global(svg) { transition: none; } }
</style>
