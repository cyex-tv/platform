<script lang="ts">
  // A section's head (cyex-web): optional eyebrow, the heading (16ch, ends with a full stop),
  // an optional lead and optional actions. `level` 1 for a page title, 2 (default) for a section.
  import type { Snippet } from 'svelte';
  import Eyebrow from './Eyebrow.svelte';

  type Props = {
    title: string;
    eyebrow?: string;
    lit?: 'top' | 'middle' | 'bottom';
    lead?: string;
    level?: 1 | 2;
    size?: 'section' | 'page';
    center?: boolean;
    id?: string;
    actions?: Snippet;
  };
  let { title, eyebrow, lit = 'middle', lead, level = 2, size = 'section', center = false, id, actions }: Props = $props();
</script>

<header class="head s-{size}" class:center>
  {#if eyebrow}<Eyebrow {lit}>{eyebrow}</Eyebrow>{/if}
  <svelte:element this={level === 1 ? 'h1' : 'h2'} {id}>{title}</svelte:element>
  {#if lead}<p class="lead">{lead}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .head :global(h1), .head :global(h2) { max-width: 16ch; }
  .s-section :global(h1), .s-section :global(h2) { font-size: var(--t-h2); line-height: 1.02; letter-spacing: -.04em; }
  .s-page :global(h1), .s-page :global(h2) { font-size: var(--t-h3-sub); line-height: 1.1; letter-spacing: -.03em; max-width: 24ch; }
  .lead { margin: 16px 0 0; font-size: var(--t-lead); line-height: 1.55; color: var(--muted); max-width: 44ch; }
  .s-page .lead { margin-top: 12px; }
  .actions { margin-top: 28px; }
  .center { text-align: center; }
  .center :global(h1), .center :global(h2), .center .lead { margin-inline: auto; }
  .center :global(.eyebrow), .center .actions { justify-content: center; }
</style>
