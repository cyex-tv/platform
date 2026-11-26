<script lang="ts">
  // A cosmetic badge at 18px (or `size`), sharp on dense screens: the file is picked from the
  // badge size ladder (18 to 512 px, see ./badgeFile), the smallest at or above size × the pixel
  // ratio, else the largest. Its tooltip is the badge's own; the picture is decorative next to a
  // name unless `label` asks otherwise. Animated badges show their first frame under reduced
  // motion.
  import type { Badge } from './store.svelte';
  import { reduced } from './reduced.svelte';
  import { safeUrl } from './paint';
  import { badgeFiles, pickBadgeFile } from './badgeFile';
  import { display } from '$lib/dpr.svelte';

  type Props = { badge: Pick<Badge, 'name' | 'tooltip' | 'image'>; size?: number; label?: boolean };
  let { badge, size = 18, label = false }: Props = $props();

  const names = $derived(badge.image.files.map((f) => f.name));
  const still = $derived(reduced.on && badge.image.animated);
  const url = (name: string) => safeUrl(`${badge.image.host.replace(/\/+$/, '')}/${name}`);
  // Before mount a srcset (width descriptors, with `sizes`) lets the browser pick; after it the
  // file is picked here, with no srcset (Chromium keeps removed images that have one in memory:
  // see pictureFor in $lib/emote).
  const picked = $derived(pickBadgeFile(names, size, display.dpr, still));
  const one = $derived(picked ? url(picked.name) : null);
  const set = $derived(
    display.dpr === 0
      ? badgeFiles(names, still)
          .map((f) => (url(f.name) ? `${url(f.name)} ${f.px}w` : ''))
          .filter(Boolean)
          .join(', ') || undefined
      : undefined,
  );
</script>

{#if one}
  <img class="badge" src={one} srcset={set} sizes={set ? `${size}px` : undefined} width={size} height={size} alt={label ? badge.name : ''} title={badge.tooltip || badge.name} loading="lazy" decoding="async" />
{:else}
  <span class="badge ph" style:width="{size}px" style:height="{size}px" title={badge.tooltip || badge.name} aria-hidden={!label}></span>
{/if}

<style>
  .badge { flex: none; display: inline-block; vertical-align: -0.2em; border-radius: var(--radius-sm); }
  .ph { background: var(--surface-2); }
</style>
