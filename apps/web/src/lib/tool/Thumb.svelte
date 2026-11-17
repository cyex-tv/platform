<script lang="ts">
  import { ready, src, srcset, type Picture } from '$lib/emote';

  let { emote, name, box = 56 }: { emote: Picture; name: string; box?: number } = $props();
</script>

<span class="thumb" style:width="{box}px" style:height="{box}px">
  {#if ready(emote)}
    <img src={src(emote, 1)} srcset={srcset(emote)} alt={name} loading="lazy" />
  {:else}
    <span class="ui-spinner" role="img" aria-label="Files still being made"></span>
  {/if}
</span>

<style>
  .thumb { display: grid; place-items: center; flex: none; overflow: hidden; }
  img { max-width: 100%; max-height: 100%; height: auto; image-rendering: auto; }
</style>
