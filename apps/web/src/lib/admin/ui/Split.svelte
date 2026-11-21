<script lang="ts">
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
    children: Snippet;
  };
  let { label, open, onclose, panel, empty, children }: Props = $props();

  let narrow = $state(false);
  $effect(() => {
    const mq = matchMedia('(max-width: 1099.98px)');
    narrow = mq.matches;
    const on = () => (narrow = mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });
  let sheet = $state(false);
  $effect(() => {
    sheet = narrow && open;
  });
</script>

<div class="split">
  <div class="list">{@render children()}</div>
  {#if !narrow}
    <aside class="panel" aria-label={label} use:fitSticky>
      {#if open}{@render panel()}{:else if empty}<div class="none">{@render empty()}</div>{/if}
    </aside>
  {/if}
</div>
{#if narrow}
  <BottomSheet bind:open={sheet} {label} onclose={() => open && onclose()}>
    {#if open}<div class="in-sheet">{@render panel()}</div>{/if}
  </BottomSheet>
{/if}

<style>
  .split { display: grid; grid-template-columns: minmax(0, 1fr); align-items: start; }
  .list { min-width: 0; }
  @media (min-width: 1100px) {
    /* The panel starts right under the header: the shell's top gap moves into the list column. */
    .split { grid-template-columns: minmax(0, 1fr) 360px; column-gap: 24px; margin: calc(-1 * var(--rail-gap)) 0 -48px; }
    .list { padding: var(--rail-gap) 0 48px; }
    .panel { position: sticky; top: var(--nav-h); align-self: start; min-height: calc(100svh - var(--nav-h)); min-width: 0;
      margin-right: calc(-1 * var(--page-pad)); border-left: 1px solid var(--border); background: var(--surface); }
    .panel:global([data-tall]) { position: relative; top: auto; }
  }
  .none { padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .in-sheet { margin: 0 -4px; }
  .in-sheet :global(.adm-ip-head) { position: static; }
</style>
