<script lang="ts">
  // A user's name drawn in their paint, with their badge before it, wherever a name appears
  // (profile, account menu, editors, admin, channel pages). The name stays real text: copyable,
  // read by screen readers as the name, isolated in <bdi>. Without a paint it is plain text in the
  // surrounding colour. Image paints show their still frame under reduced motion.
  //   <PaintedName name={who.displayName} paint={look.paint} badge={look.badge} />
  import { onMount } from 'svelte';
  import { clean } from '$lib/format';
  import BadgeIcon from './BadgeIcon.svelte';
  import { paintStyle, type PaintLike } from './paint';
  import { reduced, watchReduced } from './reduced.svelte';
  import type { Badge } from './store.svelte';

  type Props = {
    name: string;
    paint?: PaintLike | null;
    badge?: Pick<Badge, 'name' | 'tooltip' | 'image'> | null;
    /** The badge's size in px (18 in rows, bigger in a profile head). */
    badgeSize?: number;
    class?: string;
  };
  let { name, paint = null, badge = null, badgeSize = 18, class: cls = '' }: Props = $props();
  onMount(watchReduced);

  const style = $derived(paintStyle(paint, reduced.on));
</script>

<span class="pn {cls}">
  {#if badge}<BadgeIcon {badge} size={badgeSize} />{/if}
  <bdi class="txt" class:painted={!!style} {style}>{clean(name)}</bdi>
</span>

<style>
  .pn { display: inline-flex; align-items: center; gap: .35em; min-width: 0; max-width: 100%; vertical-align: bottom; }
  .txt { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* Room for drop shadows and descenders inside the clipped box. */
  .painted { padding: 0 .08em; margin: 0 -.08em; }
  @media (forced-colors: active) { .painted { background: none !important; -webkit-text-fill-color: CanvasText !important; filter: none !important; } }
</style>
