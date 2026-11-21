<script lang="ts">
  // One section of an admin page that is about a single thing (the user dashboard): flat, a hairline
  // above, an uppercase caption with a count and actions at the right, then dense rows. Rows (`.ui-list > li`) inside
  // are 44px; `.box-sub` is a quiet uppercase caption between two lists in one panel; `.box-none`
  // is a one-line empty sentence. Build the contents with the usual tool classes.
  //   <Box title="Sessions" count={count(rows.length)}>…rows…</Box>
  import type { Snippet } from 'svelte';

  type Props = { title: string; count?: string | null; actions?: Snippet; children: Snippet };
  let { title, count = null, actions, children }: Props = $props();
  const id = $props.id();
</script>

<section class="box" aria-labelledby="{id}-h">
  <header>
    <h2 class="adm-label" id="{id}-h">{title}</h2>
    {#if count}<span class="n">{count}</span>{/if}
    {#if actions}<span class="acts">{@render actions()}</span>{/if}
  </header>
  {@render children()}
</section>

<style>
  /* A flat section (round 2): no box, a hairline above, the uppercase label as its head. */
  .box { min-width: 0; padding-bottom: 8px; border-top: 1px solid var(--border); }
  header { display: flex; align-items: center; gap: 8px; min-height: 44px; padding: 0; }
  header h2 { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .n { font-family: var(--font-mono); font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .acts { display: inline-flex; align-items: center; gap: 4px; }
  .box > :global(.ui-list) { border-top: 1px solid var(--border); }
  .box > :global(.ui-list > li) { min-height: 44px; padding: 6px 0; }
  .box :global(.box-sub) { display: flex; align-items: center; gap: 8px; min-height: 36px; margin: 0; padding: 0; border-top: 1px solid var(--border);
    font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .box :global(header + .box-sub), .box :global(.box-sub:first-child) { border-top: 0; }
  .box :global(.box-none) { margin: 0; padding: 4px 0 8px; font-size: var(--t-small); color: var(--muted); }
  .box :global(.box-none + .box-sub), .box :global(.ui-list + .box-sub) { border-top: 1px solid var(--border); }
</style>
