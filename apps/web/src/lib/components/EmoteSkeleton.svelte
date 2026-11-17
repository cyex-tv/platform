<script lang="ts">
  // A placeholder tile the same size as an EmoteCard, while emotes load.
  import type { Density } from './EmoteCard.svelte';

  let { density = 'comfortable', creator = true }: { density?: Density; creator?: boolean } = $props();
</script>

<span class="sk {density}" aria-hidden="true">
  <span class="art"></span>
  <span class="line name"></span>
  {#if creator}<span class="line by"></span>{/if}
</span>

<style>
  .sk { --art: var(--emote-art); display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 10px 8px 12px; }
  .compact { --art: var(--emote-art-compact); padding: 8px 6px 10px; }
  .art { width: calc(var(--art) * .75); height: calc(var(--art) * .75); margin: calc(var(--art) * .125) 0 calc(var(--art) * .125 + 2px);
    border-radius: var(--radius-m); background: var(--surface-2); }
  .line { height: 12px; border-radius: var(--radius-btn); background: var(--surface-2); }
  .name { width: 64%; margin-top: 3px; }
  .by { width: 40%; height: 10px; }
  .sk > * { animation: breathe 1.6s ease-in-out infinite alternate; }
  @keyframes breathe { from { opacity: 1; } to { opacity: .55; } }
  @media (prefers-reduced-motion: reduce) { .sk > * { animation: none; } }
</style>
