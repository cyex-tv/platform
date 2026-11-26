<script lang="ts">
  // A badge picture at its real pixel size, up to 512 px (a bigger upload was scaled down to
  // 512): the `original` file, not a ladder size. Small pixel art stays crisp where a dense
  // screen or a zoom enlarges it, a bigger picture scales smoothly. A picture made before
  // originals existed shows its largest ladder size instead (never enlarged past its own size).
  // Animated badges show their first frame under reduced motion. With `caption` it sits on a stage
  // with its pixel size and whether it is the original or the largest size (a preview panel).
  import type { Badge } from './store.svelte';
  import { reduced } from './reduced.svelte';
  import { safeUrl } from './paint';
  import { badgeFull, enlarge } from './badgeFile';

  type Props = { badge: Pick<Badge, 'name' | 'tooltip' | 'image'>; caption?: boolean };
  let { badge, caption = false }: Props = $props();

  const still = $derived(reduced.on && badge.image.animated);
  const file = $derived(badgeFull(badge.image.files, still));
  const src = $derived(file ? safeUrl(`${badge.image.host.replace(/\/+$/, '')}/${file.name}`) : null);
  const side = $derived(file ? Math.max(file.width, file.height) : 0);
</script>

{#snippet picture(f: NonNullable<typeof file>, url: string)}
  <img class="orig" src={url} width={f.width} height={f.height} style:image-rendering={enlarge(side)} alt="" title={badge.tooltip || badge.name} decoding="async" data-original={f.name} />
{/snippet}

{#if file && src}
  {#if caption}
    <figure class="pv">
      <div class="stage">{@render picture(file, src)}</div>
      <figcaption>
        <span class="dm">{file.width} × {file.height} px</span>
        <span class="kind">{file.original ? 'Original' : 'Largest size'}</span>
      </figcaption>
    </figure>
  {:else}
    {@render picture(file, src)}
  {/if}
{/if}

<style>
  /* Real pixels, never larger than the column (a narrow phone shrinks it, smoothly). */
  .orig { display: block; max-width: 100%; height: auto; border-radius: var(--radius-sm); }
  /* A preview in the Inspector panel's language: a flat stage on the elevated ground, the facts in
     mono under it on a hairline. The stage keeps its height steady while the picture loads. */
  .pv { margin: 0; min-width: 0; border: 1px solid var(--border); }
  .stage { display: grid; grid-template-columns: minmax(0, auto); justify-content: center; place-items: center; min-height: 96px; padding: 16px; background: var(--elevated-2); overflow: hidden; }
  figcaption { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 2px 12px; padding: 6px 10px; border-top: 1px solid var(--border); font-size: var(--t-fine); color: var(--dim); }
  .dm { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  .kind { font-weight: 600; color: var(--muted); }
</style>
