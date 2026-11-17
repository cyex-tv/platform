<script lang="ts" module>
  import type { Picture } from '$lib/emote';

  /** What a card needs: the emote's pictures, id and name; owner and flags when known. */
  export type CardEmote = Picture & {
    id: string;
    name: string;
    owner?: { displayName: string } | null;
    flags?: { zeroWidth: boolean } | null;
    /** The name it has in a set, when that differs from its own. */
    shownName?: string;
  };
  export type Density = 'comfortable' | 'compact';
  /** The art box's height per density (keep in step with --emote-art / --emote-art-compact). */
  export const ART = { comfortable: 112, compact: 80 } as const;
</script>

<script lang="ts">
  // One emote in a grid: name first, borderless, the art in a fixed box (no layout shift).
  // Animated emotes show their still first frame and play while hovered or focused; under
  // reduced motion they stay still. The tile's ground lifts one tone on hover.
  import { hasStill, isWide, ready, size, src, srcsetW } from '$lib/emote';

  type Props = {
    emote: CardEmote;
    density?: Density;
    creator?: boolean;
    labels?: boolean;
    href?: string;
    eager?: boolean;
  };
  let { emote: e, density = 'comfortable', creator = true, labels = true, href, eager = false }: Props = $props();

  let hot = $state(false);
  let loaded = $state(false);
  const wide = $derived(isWide(e));
  const still = $derived(hasStill(e));
  const dims = $derived(size(e));
  // The picture fills 3/4 of the box height; `sizes` lets the browser pick the right file.
  const shownW = $derived(Math.round((ART[density] * 0.75 * dims.width) / dims.height));
  const tags = $derived(
    labels ? [e.animated && 'animated', e.flags?.zeroWidth && 'zero-width', wide && 'wide'].filter(Boolean) as string[] : [],
  );

  function heat(on: boolean) {
    // Read at the moment it runs (cyex-design): reduced motion never plays.
    hot = on && e.animated && still && !matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!hot) loaded = false;
  }
</script>

<a
  class="card {density}"
  class:wide
  href={href ?? `/emotes/${e.id}`}
  onpointerenter={(ev) => ev.pointerType === 'mouse' && heat(true)}
  onpointerleave={() => heat(false)}
  onfocus={() => heat(true)}
  onblur={() => heat(false)}
>
  <span class="art">
    {#if ready(e)}
      <img
        class="pic"
        src={src(e, 1, still)}
        srcset={srcsetW(e, still)}
        sizes="{shownW}px"
        width={dims.width}
        height={dims.height}
        alt=""
        loading={eager ? 'eager' : 'lazy'}
        decoding="async"
      />
      {#if hot}
        <img
          class="pic moving"
          class:loaded
          src={src(e, 1)}
          srcset={srcsetW(e)}
          sizes="{shownW}px"
          width={dims.width}
          height={dims.height}
          alt=""
          decoding="async"
          onload={() => (loaded = true)}
        />
      {/if}
    {:else}
      <span class="processing">Processing</span>
    {/if}
  </span>
  <span class="name">{e.shownName ?? e.name}</span>
  {#if creator && e.owner}<span class="by">{e.owner.displayName}</span>{/if}
  {#if tags.length}
    <span class="tags">{#each tags as t}<span class="tag">{t}</span>{/each}</span>
  {/if}
</a>

<style>
  .card { --art: var(--emote-art); position: relative; display: flex; flex-direction: column; align-items: center; gap: 2px;
    height: 100%; padding: 10px 8px 12px; border-radius: var(--radius-m); background: transparent; text-align: center;
    transition: background-color var(--dur-color), transform var(--dur-press) ease-out; }
  .compact { --art: var(--emote-art-compact); padding: 8px 6px 10px; }
  .card:active { transform: scale(var(--press)); }
  .card:focus-visible { outline-offset: 0; border-radius: var(--radius-m); background: var(--surface); }
  .art { position: relative; display: grid; place-items: center; width: var(--art); height: var(--art); margin-bottom: 6px; }
  .wide .art { width: calc(var(--art) * 2); }
  .pic { grid-area: 1 / 1; display: block; height: calc(var(--art) * .75); width: auto; max-width: 100%; object-fit: contain; }
  .moving { opacity: 0; }
  .moving.loaded { opacity: 1; }
  .processing { font-size: var(--t-fine); color: var(--muted); }
  .name { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--t-ui); font-weight: 500; line-height: 1.35; color: var(--text); }
  .compact .name { font-size: var(--t-small); }
  .by { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--t-fine); line-height: 1.4; color: var(--muted); }
  .tags { display: flex; flex-wrap: wrap; justify-content: center; gap: 4px; margin-top: 4px; }
  .tag { display: inline-flex; align-items: center; height: 18px; padding: 0 5px; border: 1px solid var(--border);
    border-radius: var(--radius-btn); font-size: 11px; font-weight: 500; line-height: 1; color: var(--muted); white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) {
    .card:hover { background: var(--surface); }
  }
</style>
