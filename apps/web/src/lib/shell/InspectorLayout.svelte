<script lang="ts">
  // Page layout 3 of 3, the Inspector (redesign, 2026/11/21): the page's head, optional tabs and
  // the content on the left; on the right a panel (facts, actions, related) that runs the full
  // height under the header, sticks while the content scrolls and scrolls inside itself only when
  // it is taller than the window. The same frame as the profile (/users/<id>):
  //   <InspectorLayout title="Set" label="About this set">
  //     {#snippet head()}<PageHead … />{/snippet}
  //     …content…
  //     {#snippet panel()}<PanelBlock title="Facts">…</PanelBlock>{/snippet}
  //   </InspectorLayout>
  // Under 1100px it is one column and the panel becomes a bordered card: right under the head
  // (`narrow="before"`, the default: facts and actions first) or after the content
  // (`narrow="after"`: an editor whose content is the point). `fold` keeps it shut under a
  // toggle there, so a long panel doesn't push the content off the first screen.
  import type { Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';

  type Props = {
    /** The panel's visible label (uppercase, in its sticky head). */
    title: string;
    /** The panel's accessible name (defaults to the title). */
    label?: string;
    head?: Snippet;
    tabs?: Snippet;
    children: Snippet;
    panel: Snippet;
    /** Buttons in the panel's head, at the right (a close button). */
    panelActions?: Snippet;
    narrow?: 'before' | 'after';
    fold?: boolean;
    /** The rail layout's 20px above the head (off for a head with its own room, ProfileHeader). */
    headPad?: boolean;
    /** Panel width on wide screens. */
    width?: number;
    testid?: string;
  };
  let { title, label, head, tabs, children, panel, panelActions, narrow = 'before', fold = false, width = 360, headPad = true, testid }: Props = $props();
  let open = $state(false);
  const id = $props.id();
</script>

<div class="wrap inspector" class:after={narrow === 'after'} class:has-tabs={!!tabs} style:--ip-w="{width}px">
  {#if head}<div class="ip-head-area" class:pad={headPad}>{@render head()}</div>{/if}
  {#if tabs}<div class="ip-tabs">{@render tabs()}</div>{/if}
  <div class="ip-main">{@render children()}</div>
  <aside class="ip-panel" aria-label={label ?? title} data-testid={testid}>
    <div class="ip-top">
      <h2 class="ip-label" id="{id}-h">{title}</h2>
      {#if fold}
        <button class="ip-toggle" type="button" aria-expanded={open} aria-controls="{id}-b" onclick={() => (open = !open)}>
          <span class="ip-word">{title}</span><span class="sr-only">{label ?? title}</span><span aria-hidden="true">{open ? 'Hide' : 'Show'}</span><Icon name="chevron-down" size={14} />
        </button>
      {/if}
      {#if panelActions}<div class="ip-acts">{@render panelActions()}</div>{/if}
    </div>
    <div class="ip-body" id="{id}-b" class:shut={fold && !open}>{@render panel()}</div>
  </aside>
</div>

<style>
  .inspector { display: grid; grid-template-columns: minmax(0, 1fr) var(--ip-w); grid-template-areas: 'head panel' 'tabs panel' 'main panel';
    grid-template-rows: auto auto 1fr; column-gap: 24px; align-items: start; }
  .ip-head-area { grid-area: head; min-width: 0; }
  .ip-head-area.pad { padding-top: var(--rail-gap); }
  .ip-tabs { grid-area: tabs; min-width: 0; }
  .ip-main { grid-area: main; min-width: 0; padding: 0 0 48px; }
  .has-tabs .ip-main { padding-top: 20px; }
  .ip-head-area:empty { display: none; }
  .ip-panel { grid-area: panel; align-self: start; position: sticky; top: var(--nav-h); height: calc(100svh - var(--nav-h)); overflow-y: auto;
    overscroll-behavior: contain; margin-right: calc(-1 * var(--page-pad)); border-left: 1px solid var(--border); background: var(--surface);
    /* Past the 1520px wrap the panel's surface runs on to the window's edge (its content stays put),
       as on every page with a right panel: the shop, the admin inspector, the browse tray. */
    box-shadow: 0 0 0 100vw var(--surface); clip-path: inset(0 -100vw 0 0); }
  .ip-top { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; gap: 8px; height: 48px; padding: 0 8px 0 16px;
    border-bottom: 1px solid var(--border); background: var(--surface); }
  .ip-label { margin: 0; font-size: var(--t-label, 12px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); line-height: 1; }
  .ip-acts { display: flex; align-items: center; gap: 4px; margin-left: auto; }
  .ip-body { padding: 8px 16px 16px; }
  .ip-toggle { display: none; }

  /* One column: the panel is a card under the head (or after the content). */
  @media (max-width: 1099.98px) {
    .inspector { grid-template-columns: minmax(0, 1fr); grid-template-areas: 'head' 'panel' 'tabs' 'main'; grid-template-rows: none; }
    .inspector.after { grid-template-areas: 'head' 'tabs' 'main' 'panel'; }
    .after .ip-main { padding-bottom: 20px; }
    .after .ip-panel { margin-bottom: 48px; }
    .ip-head-area { padding-bottom: 0; }
    .ip-panel { position: static; height: auto; overflow: visible; margin: 0 0 16px; border: 1px solid var(--border); border-radius: var(--radius-card);
      box-shadow: none; clip-path: none; }
    .ip-top { position: static; height: 44px; border-radius: var(--radius-card) var(--radius-card) 0 0; }
    .ip-body { padding-top: 4px; }
    /* Folded: the head is the toggle. */
    .ip-panel:has(.ip-toggle) .ip-top { padding: 0; height: auto; border-bottom: 0; border-radius: var(--radius-card); }
    .ip-panel:has(.ip-toggle) .ip-label { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
    .ip-toggle { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; min-height: 44px; padding: 0 16px;
      border: 0; border-radius: var(--radius-card); background: none; color: var(--text); font: inherit; font-size: var(--t-small); font-weight: 600; cursor: pointer; }
    .ip-word { margin-right: auto; font-size: var(--t-label, 12px); letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
    .ip-toggle :global(svg) { color: var(--dim); transition: transform var(--fast) var(--ease); }
    .ip-toggle[aria-expanded='true'] :global(svg) { transform: rotate(180deg); }
    .ip-toggle:focus-visible { outline-offset: -2px; }
    .ip-panel:has(.ip-toggle) .ip-body { border-top: 1px solid var(--border); }
    .ip-body.shut { display: none; }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .ip-toggle :global(svg) { transition: none; } }
</style>
