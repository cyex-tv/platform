<script lang="ts">
  // A user's name drawn in their paint, with their badge before it, wherever a name appears
  // (profile, account menu, editors, admin, channel pages). The name stays real text: copyable,
  // read by screen readers as the name, isolated in <bdi>. Without a paint it is plain text in the
  // surrounding colour. Image paints show their still frame under reduced motion.
  //   <PaintedName name={who.displayName} paint={look.paint} badge={look.badge} />
  import { onMount } from 'svelte';
  import { clean } from '$lib/format';
  import BadgeIcon from './BadgeIcon.svelte';
  import { paintFloor, paintStyle, type PaintLike } from './paint';
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
  // Where the paint is too close to the ground to read (pale on light, dark on dark): a hairline
  // outline in that theme only (see the styles). The paint itself is untouched.
  const floor = $derived(style ? paintFloor(paint) : null);
</script>

<span class="pn {cls}">
  {#if badge}<BadgeIcon {badge} size={badgeSize} />{/if}
  {#if floor}
    <span class="fl" class:l={floor !== 'dark'} class:d={floor !== 'light'}><bdi class="txt painted" {style} title={clean(name)}>{clean(name)}</bdi></span>
  {:else}
    <bdi class="txt" class:painted={!!style} {style} title={clean(name)}>{clean(name)}</bdi>
  {/if}
</span>

<style>
  .pn { display: inline-flex; align-items: center; gap: .35em; min-width: 0; max-width: 100%; vertical-align: bottom; }
  /* A link around a name draws its own underline on the name: an inline-flex box would drop it. */
  .pn, .fl, .txt { text-decoration: inherit; }
  .fl { display: inline-flex; min-width: 0; max-width: 100%; }
  /* Readability floor: a soft dark halo in the light theme, a light one in the dark theme, drawn
     over the paint's own shadows (it filters the finished name). Stronger with prefers-contrast. */
  :global([data-theme='light']) .fl.l { filter: drop-shadow(0 0 .5px rgb(0 0 0 / .55)); }
  :global(:root:not([data-theme='light'])) .fl.d { filter: drop-shadow(0 0 .5px rgb(255 255 255 / .55)); }
  @media (prefers-contrast: more) {
    :global([data-theme='light']) .fl.l { filter: drop-shadow(0 0 .6px rgb(0 0 0 / .95)) drop-shadow(0 0 .6px rgb(0 0 0 / .95)); }
    :global(:root:not([data-theme='light'])) .fl.d { filter: drop-shadow(0 0 .6px rgb(255 255 255 / .95)) drop-shadow(0 0 .6px rgb(255 255 255 / .95)); }
  }
  .txt { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* Room for drop shadows and descenders inside the clipped box. */
  .painted { padding: 0 .08em; margin: 0 -.08em; }
  @media (forced-colors: active) { .fl { filter: none !important; } .painted { background: none !important; -webkit-text-fill-color: CanvasText !important; filter: none !important; } }
</style>
