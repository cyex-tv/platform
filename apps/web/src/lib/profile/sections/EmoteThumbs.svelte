<script lang="ts">
  // Two rows of a set's first emotes (6 a row, animated ones playing), empty slots drawn as quiet squares so every card
  // has the same height. Decorative: the card's name and count are the text.
  import EmotePicture from '$lib/components/EmotePicture.svelte';
  import { ready, type Picture } from '$lib/emote';

  let { emotes, per = 6 }: { emotes: Picture[]; per?: number } = $props();
  const slots = $derived(Array.from({ length: per * 2 }, (_, i) => emotes[i] ?? null));
</script>

<span class="thumbs" style:--per={per} aria-hidden="true">
  {#each slots as e, i (i)}
    <span class="slot" class:empty={!e}>
      {#if e && ready(e)}<EmotePicture emote={e} class="pic" />{/if}
    </span>
  {/each}
</span>

<style>
  .thumbs { display: grid; grid-template-columns: repeat(var(--per), minmax(0, 1fr)); gap: 6px; }
  .slot { display: grid; place-items: center; aspect-ratio: 1; min-width: 0; }
  .slot.empty { border-radius: var(--radius); background: var(--elevated-2); }
  .slot :global(.pic) { max-width: 100%; max-height: 100%; width: auto; height: auto; object-fit: contain; }
</style>
