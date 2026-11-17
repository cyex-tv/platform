<script lang="ts">
  // A placeholder the same size as an EmoteTile, while emotes load.
  import type { Density } from './EmoteTile.svelte';

  let { density = 'comfortable', creator = true }: { density?: Density; creator?: boolean } = $props();
</script>

<span class="sk {density}" aria-hidden="true">
  <span class="art"></span>
  <span class="line name"></span>
  {#if creator}<span class="line by"></span>{/if}
</span>

<style>
  .sk { --art: 88px; display: flex; flex-direction: column; align-items: center; gap: 6px; height: 100%; padding: 14px 10px 12px;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .compact { --art: 60px; padding: 10px 8px 9px; }
  .art { width: calc(var(--art) * .8); height: calc(var(--art) * .8); margin: calc(var(--art) * .1) 0 calc(var(--art) * .1 + 2px);
    border-radius: var(--radius); background: var(--surface-hover); }
  .line { height: 12px; border-radius: var(--radius); background: var(--surface-hover); }
  .name { width: 64%; }
  .by { width: 40%; height: 10px; }
  .sk > * { animation: breathe 1.6s ease-in-out infinite alternate; }
  @keyframes breathe { from { opacity: 1; } to { opacity: .55; } }
  @media (prefers-reduced-motion: reduce) { .sk > * { animation: none; } }
</style>
