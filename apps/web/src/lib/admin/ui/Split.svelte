<script lang="ts">
  import { media, NO_PANEL } from '$lib/media.svelte';
  // An admin list with an inspector: the list on the left, the picked item's facts and actions in
  // a 360px panel on the right that runs the full height of the window (the profile's ABOUT panel
  // shape: flush with the window's right edge, a hairline beside it, one step up in colour). The
  // panel sticks while it fits and scrolls with the page when it doesn't (no scroll bar of its
  // own). Under 1100px the panel becomes a bottom sheet that opens when something is picked.
  //   <Split label="Badge" open={!!picked} onclose={() => pick(null)}>
  //     {#snippet panel()}…{/snippet}  …the list…
  //   </Split>
  import type { Snippet } from 'svelte';
  import BottomSheet from '$lib/components/BottomSheet.svelte';
  import { fitSticky } from '../shell/fit';

  type Props = {
    /** The panel's name for assistive tech and the sheet's title ("Badge", "Report"). */
    label: string;
    /** Something is picked: the panel shows it (and the sheet opens on narrow windows). */
    open: boolean;
    onclose: () => void;
    panel: Snippet;
    /** What the panel says with nothing picked (wide windows only). */
    empty?: Snippet;
    /** Under 1100px: a bottom sheet that opens when something is picked (lists), or the panel
     *  inline above the content (a page about one thing, always open: the user dashboard), or
     *  'bottom', inline after it (the overview: what waits comes first on a phone). */
    stack?: 'sheet' | 'top' | 'bottom';
    /** The panel's width: 'm' 380px (facts and actions), 'l' a large inspector (about 46%, the
     *  emote page's right column: the review queue, reports). */
    size?: 'm' | 'l';
    children: Snippet;
  };
  let { label, open, onclose, panel, empty, stack = 'sheet', size = 'm', children }: Props = $props();

  // Unknown until the first check, so neither layout flashes before the other.
  const query = media(NO_PANEL);
  const narrow = $derived(query.ready ? query.matches : null);
  let sheet = $state(false);
  $effect(() => {
    sheet = !!narrow && stack === 'sheet' && open;
  });
</script>

<div class="split" class:l={size === 'l'}>
  {#if narrow && stack === 'top' && open}<aside class="top" aria-label={label}>{@render panel()}</aside>{/if}
  <div class="list">{@render children()}</div>
  {#if narrow && stack === 'bottom' && open}<aside class="top bottom" aria-label={label}>{@render panel()}</aside>{/if}
  {#if narrow === false}
    <aside class="panel" aria-label={label} use:fitSticky>
      {#if open}{@render panel()}{:else if empty}{@render empty()}{/if}
    </aside>
  {/if}
</div>
{#if narrow && stack === 'sheet'}
  <BottomSheet bind:open={sheet} {label} title={false} onclose={() => open && onclose()}>
    {#if open}<div class="in-sheet">{@render panel()}</div>{/if}
  </BottomSheet>
{/if}

<style>
  .split { display: grid; grid-template-columns: minmax(0, 1fr); align-items: start; }
  .list { min-width: 0; }
  @media (min-width: 1100px) {
    /* The panel starts right under the header: the shell's top gap moves into the list column. */
    .split { grid-template-columns: minmax(0, 1fr) 380px; column-gap: 24px; margin: calc(-1 * var(--rail-gap)) 0 -48px; }
    .list { padding: var(--rail-gap) 0 48px; }
    .panel { position: sticky; top: var(--nav-h); align-self: start; min-height: calc(100svh - var(--nav-h)); min-width: 0;
      margin-right: calc(-1 * var(--page-pad)); border-left: 1px solid var(--border); background: var(--surface);
      }
    .split.l { grid-template-columns: minmax(0, 1fr) clamp(460px, 46%, 720px); }
    .panel:global([data-tall]) { position: relative; top: auto; }
  }
  .top { margin-bottom: 16px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); overflow: hidden; }
  .top :global(.adm-ip-head) { position: static; }
  .bottom { margin: 24px 0 0; }
  .in-sheet { margin: 0 -4px; }
  .in-sheet :global(.adm-ip-head) { position: static; }
  /* The sheet has its own close button; the panel head's would be a second one. */
  .in-sheet :global(.adm-ip-head [aria-label="Close"]) { display: none; }
</style>
