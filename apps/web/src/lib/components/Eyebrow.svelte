<script lang="ts">
  // A section eyebrow (cyex-web): the mark with one bar lit, beside the section name.
  // Light a different bar to mark position: top for the first section, middle, bottom.
  import type { Snippet } from 'svelte';
  import Mark from './Mark.svelte';

  let { lit = 'middle', children }: { lit?: 'top' | 'middle' | 'bottom'; children: Snippet } = $props();
</script>

<p class="eyebrow"><Mark size={18} {lit} />{@render children()}</p>

<style>
  .eyebrow { display: flex; align-items: center; gap: 10px; margin: 0 0 22px;
    font-size: var(--t-ui); font-weight: 500; color: var(--muted); }
</style>
