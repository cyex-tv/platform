<script lang="ts">
  // An emote's picture as every list shows it: animated emotes play, no hover needed. Under
  // reduced motion the browser picks the still first frame instead (a <picture> source, so it
  // holds before any script runs and follows the setting live). Lazy and async by default, so only
  // pictures near the screen load and decoding never blocks the page. The <picture> box is
  // `display: contents`: style the <img> through `class`.
  import type { Action } from 'svelte/action';
  import { hasStill, size, src, srcset, srcsetW, type Picture } from '$lib/emote';

  type Props = {
    emote: Picture;
    /** The shown width as a `sizes` value (width descriptors); left out, the files are offered by density (1x, 2x, 4x). */
    sizes?: string;
    eager?: boolean;
    alt?: string;
    class?: string;
    /** Run on the <img> (e.g. a fade-in once loaded). */
    use?: Action<HTMLImageElement>;
  };
  let { emote: e, sizes, eager = false, alt = '', class: cls = '', use = () => {} }: Props = $props();

  const dims = $derived(size(e));
  const still = $derived(hasStill(e));
</script>

<picture>
  {#if still}
    <source media="(prefers-reduced-motion: reduce)" srcset={sizes ? srcsetW(e, true) : srcset(e, true)} {sizes} />
  {/if}
  <img
    class={cls}
    src={src(e, 1)}
    srcset={sizes ? srcsetW(e) : srcset(e)}
    {sizes}
    width={dims.width}
    height={dims.height}
    {alt}
    loading={eager ? 'eager' : 'lazy'}
    decoding="async"
    use:use
  />
</picture>

<style>
  picture { display: contents; }
</style>
