<script lang="ts">
  // An emote's small picture in lists and rows; animated ones play (still under reduced motion).
  import EmotePicture from '$lib/components/EmotePicture.svelte';
  import { ready, type Picture } from '$lib/emote';

  let { emote, name, box = 56 }: { emote: Picture; name: string; box?: number } = $props();
</script>

<span class="thumb" style:width="{box}px" style:height="{box}px">
  {#if ready(emote)}
    <EmotePicture {emote} class="pic" alt={name} />
  {:else}
    <span class="ui-spinner" role="img" aria-label="Files still being made"></span>
  {/if}
</span>

<style>
  .thumb { display: grid; place-items: center; flex: none; overflow: hidden; }
  .thumb :global(.pic) { max-width: 100%; max-height: 100%; width: auto; height: auto; image-rendering: auto; }
</style>
