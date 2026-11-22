<script lang="ts">
  // Page layout 1 of 2: a SideRail beside the content, both in the full-width wrapper (logo, rail and
  // footer share one left edge). Rail 240, gap 20, content the rest. Below 900px the rail turns
  // into its tab strip or sheet button above the content (SideRail decides), never the whole rail.
  //   <RailLayout>{#snippet rail()}<SideRail … />{/snippet}…content…</RailLayout>
  import type { Snippet } from 'svelte';

  /** `top`: a band across both columns above them (the profile's identity band). */
  let { rail, top, children }: { rail: Snippet; top?: Snippet; children: Snippet } = $props();
</script>

<div class="wrap rail-layout">
  {#if top}<div class="top">{@render top()}</div>{/if}
  {@render rail()}
  <div class="content">{@render children()}</div>
</div>

<style>
  .rail-layout { display: grid; grid-template-columns: var(--rail-w) minmax(0, 1fr); gap: var(--rail-gap); align-items: start;
    padding-top: var(--rail-gap); }
  .content { min-width: 0; }
  .top { grid-column: 1 / -1; min-width: 0; }
  @media (max-width: 899.98px) {
    .rail-layout { grid-template-columns: minmax(0, 1fr); gap: 0; padding-top: 0; }
    .content { padding-top: var(--rail-gap); } /* the one page top, with or without the tab strip above */
    .top { grid-column: auto; margin-top: var(--rail-gap); }
  }
</style>
