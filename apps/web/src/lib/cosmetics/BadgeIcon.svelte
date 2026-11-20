<script lang="ts">
  // A cosmetic badge at 18px (or `size`), sharp on dense screens (1x, 2x, 4x files). Its tooltip
  // is the badge's own; the picture is decorative next to a name unless `label` asks otherwise.
  // Animated badges show their first frame under reduced motion.
  import { onMount } from 'svelte';
  import type { Badge } from './store.svelte';
  import { reduced, watchReduced } from './reduced.svelte';
  import { safeUrl } from './paint';
  import { densityFor } from '$lib/emote';
  import { display } from '$lib/dpr.svelte';

  type Props = { badge: Pick<Badge, 'name' | 'tooltip' | 'image'>; size?: number; label?: boolean };
  let { badge, size = 18, label = false }: Props = $props();
  onMount(watchReduced);

  const files = $derived(new Set(badge.image.files.map((f) => f.name)));
  const still = $derived(reduced.on && badge.image.animated);
  const at = (k: number) => {
    const name = still && files.has(`${k}x_static.webp`) ? `${k}x_static.webp` : `${k}x.webp`;
    return files.has(name) ? safeUrl(`${badge.image.host.replace(/\/+$/, '')}/${name}`) : null;
  };
  // Before mount a srcset lets the browser pick; after it the file is picked here, with no srcset
  // (Chromium keeps removed images that have one in memory: see pictureFor in $lib/emote).
  const scales = $derived([1, 2, 4].filter((k) => at(k)));
  const one = $derived(display.dpr === 0 ? at(1) : at(densityFor(display.dpr, scales)));
  const set = $derived(display.dpr === 0 ? scales.map((k) => `${at(k)} ${k}x`).join(', ') : undefined);
</script>

{#if one}
  <img class="badge" src={one} srcset={set} width={size} height={size} alt={label ? badge.name : ''} title={badge.tooltip || badge.name} loading="lazy" decoding="async" />
{:else}
  <span class="badge ph" style:width="{size}px" style:height="{size}px" title={badge.tooltip || badge.name} aria-hidden={!label}></span>
{/if}

<style>
  .badge { flex: none; display: inline-block; vertical-align: -0.2em; border-radius: var(--radius-sm); }
  .ph { background: var(--surface-2); }
</style>
