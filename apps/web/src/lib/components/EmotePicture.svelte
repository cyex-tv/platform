<script lang="ts">
  // An emote's picture as every list shows it: animated emotes play, no hover needed. Under
  // reduced motion the still first frame shows instead, picked in script from the site's setting
  // (Settings → Appearance; on "System" the matchMedia patch in app.html follows the OS live). The
  // server renders the moving file. Once mounted the file for the screen is picked in script too,
  // with no srcset and no <picture><source media>: Chromium kept every removed page that showed
  // one in memory (see `pictureFor`). Lazy and async by default, so only pictures near the screen
  // load and decoding never blocks the page.
  import type { Action } from 'svelte/action';
  import { hasStill, pictureFor, size, type Picture } from '$lib/emote';
  import { reduced } from '$lib/cosmetics/reduced.svelte';
  import { display } from '$lib/dpr.svelte';

  type Props = {
    emote: Picture;
    /** The shown CSS width in px; left out, the files are picked by density (1x, 2x, 4x). */
    shown?: number;
    eager?: boolean;
    alt?: string;
    class?: string;
    /** Run on the <img> (e.g. a fade-in once loaded). */
    use?: Action<HTMLImageElement>;
  };
  let { emote: e, shown, eager = false, alt = '', class: cls = '', use = () => {} }: Props = $props();

  const dims = $derived(size(e));
  const still = $derived(hasStill(e));
  const frozen = $derived(still && reduced.on);
  const pic = $derived(pictureFor(e, display.dpr, frozen, shown));
</script>

<img
  class={cls}
  src={pic.src}
  srcset={pic.srcset}
  sizes={pic.srcset && shown ? `${shown}px` : undefined}
  width={dims.width}
  height={dims.height}
  {alt}
  loading={eager ? 'eager' : 'lazy'}
  decoding="async"
  data-still={still ? '' : undefined}
  use:use
/>
