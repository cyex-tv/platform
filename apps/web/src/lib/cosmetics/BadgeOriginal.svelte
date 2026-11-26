<script lang="ts">
  // A badge picture at its real pixel size, up to 512 px (a bigger upload was scaled down to
  // 512): the `original` file, not a ladder size. Small pixel art stays crisp where a dense
  // screen or a zoom enlarges it, a bigger picture scales smoothly. A picture made before
  // originals existed shows its largest ladder size instead (never enlarged past its own size).
  // Animated badges show their first frame under reduced motion.
  import { onMount } from 'svelte';
  import type { Badge } from './store.svelte';
  import { reduced, watchReduced } from './reduced.svelte';
  import { safeUrl } from './paint';
  import { badgeFiles, badgeOriginal, enlarge } from './badgeFile';

  type Props = { badge: Pick<Badge, 'name' | 'tooltip' | 'image'> };
  let { badge }: Props = $props();
  onMount(watchReduced);

  const still = $derived(reduced.on && badge.image.animated);
  const file = $derived.by(() => {
    const files = badge.image.files;
    const original = badgeOriginal(files, still);
    if (original) return original;
    // Before originals: the largest ladder size there is.
    const last = badgeFiles(files.map((f) => f.name), still).at(-1);
    const entry = files.find((f) => f.name === last?.name);
    return last ? { name: last.name, width: entry?.width || last.px, height: entry?.height || last.px } : null;
  });
  const src = $derived(file ? safeUrl(`${badge.image.host.replace(/\/+$/, '')}/${file.name}`) : null);
  const side = $derived(file ? Math.max(file.width, file.height) : 0);
</script>

{#if file && src}
  <img class="orig" src={src} width={file.width} height={file.height} style:image-rendering={enlarge(side)} alt="" title={badge.tooltip || badge.name} decoding="async" data-original={file.name} />
{/if}

<style>
  /* Real pixels, never larger than the column (a narrow phone shrinks it, smoothly). */
  .orig { display: block; max-width: 100%; height: auto; border-radius: var(--radius-sm); }
</style>
