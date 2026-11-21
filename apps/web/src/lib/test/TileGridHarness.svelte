<script lang="ts">
  // Test harness: a plain grid of emote tiles, as the browse, set and profile grids draw them.
  import EmoteTile, { type CardEmote } from '$lib/components/EmoteTile.svelte';
  let { items }: { items: CardEmote[] } = $props();
</script>

{#each items as e (e.id)}<EmoteTile emote={e} />{/each}
