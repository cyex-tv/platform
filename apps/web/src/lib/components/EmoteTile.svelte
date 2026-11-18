<script lang="ts" module>
  import type { Picture } from '$lib/emote';

  /** What a tile needs: the emote's pictures, id and name; owner and flags when known. */
  export type CardEmote = Picture & {
    id: string;
    name: string;
    owner?: { displayName: string } | null;
    flags?: { zeroWidth: boolean } | null;
    /** The name it has in a set, when that differs from its own. */
    shownName?: string;
  };
  export type Density = 'comfortable' | 'compact';
  /** The art box's height per density (keep in step with the tile CSS below). */
  export const TILE_ART = { comfortable: 88, compact: 60 } as const;

  /** A status note's tint: the emote states get their colour, anything else stays neutral. */
  export function noteTone(note: string): string {
    const n = note.toLowerCase().replace(/\s+/g, '_');
    return ['pending', 'approved', 'rejected', 'taken_down'].includes(n) ? n : '';
  }
</script>

<script lang="ts">
  // One emote in a grid, the same tile everywhere emotes are listed (browse, creator, channel,
  // set, your emotes, admin review): a surface card with a hairline, the art in a fixed box (no
  // layout shift), the name, the creator. The whole tile is one link; hover steps the surface and
  // the border, it never lifts. Animated emotes show their still first frame and play while
  // hovered or focused (never under reduced motion). The `status` slot (or a `note`, e.g.
  // "pending") sits in the top-left corner, for /me and the admin pages.
  import type { Snippet } from 'svelte';
  import { hasStill, isWide, ready, size, src, srcsetW } from '$lib/emote';
  import { clean } from '$lib/format';
  import Icon from './Icon.svelte';

  type Props = {
    emote: CardEmote;
    density?: Density;
    creator?: boolean;
    labels?: boolean;
    href?: string;
    eager?: boolean;
    /** A short status shown as a chip in the corner (e.g. "pending"). */
    note?: string;
    /** Your own corner content instead of `note`. */
    status?: Snippet;
    /** Select mode: the tile toggles instead of opening. `selected` says whether it is picked. */
    onpick?: () => void;
    selected?: boolean;
  };
  let { emote: e, density = 'comfortable', creator = true, labels = true, href, eager = false, note, status, onpick, selected = false }: Props = $props();

  let hot = $state(false);
  let loaded = $state(false);
  const name = $derived(clean(e.shownName ?? e.name));
  const owner = $derived(e.owner ? clean(e.owner.displayName) : '');
  // A lazy still that is already cached shows at once; one that loads late fades in.
  function settle(node: HTMLImageElement) {
    if (node.complete || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    node.classList.add('fade');
    node.addEventListener('load', () => node.classList.remove('fade'), { once: true });
    node.addEventListener('error', () => node.classList.remove('fade'), { once: true });
  }
  const wide = $derived(isWide(e));
  const still = $derived(hasStill(e));
  const dims = $derived(size(e));
  // The picture fills 80% of the box height; `sizes` lets the browser pick the right file.
  const shownW = $derived(Math.round((TILE_ART[density] * 0.8 * dims.width) / dims.height));
  // Type marks stay out of the tile's flow (every tile the same height): they show in the art's
  // corner on hover or keyboard focus, and are always in the link's name.
  const kinds = $derived(
    (labels ? [e.animated && 'animated', e.flags?.zeroWidth && 'zero-width', wide && 'wide'] : []).filter(Boolean) as string[],
  );
  const title = $derived(owner && creator ? `${name} by ${owner}` : name);

  function heat(on: boolean) {
    hot = on && e.animated && still && !matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!hot) loaded = false;
  }
</script>

<a
  class="tile {density}"
  class:wide
  class:picked={selected}
  role={onpick ? 'checkbox' : undefined}
  aria-checked={onpick ? selected : undefined}
  onclick={onpick ? (ev) => { ev.preventDefault(); onpick(); } : undefined}
  onkeydown={onpick ? (ev) => { if (ev.key === ' ') { ev.preventDefault(); onpick(); } } : undefined}
  href={href ?? `/emotes/${e.id}`}
  {title}
  onpointerenter={(ev) => ev.pointerType === 'mouse' && heat(true)}
  onpointerleave={() => heat(false)}
  onfocus={() => heat(true)}
  onblur={() => heat(false)}
>
  {#if onpick}
    <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>
  {/if}
  {#if status || note}
    <span class="status">
      {#if status}{@render status()}{:else if note}<span class="ui-chip {noteTone(note)}">{note}</span>{/if}
    </span>
  {/if}
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
        use:settle
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
    {#if kinds.length}<span class="kinds" aria-hidden="true">{kinds.join(' · ')}</span>{/if}
  </span>
  <span class="name"><bdi>{name}</bdi></span>
  {#if creator && owner}<span class="by"><bdi>{owner}</bdi></span>{/if}
  {#if kinds.length}<span class="sr-only">, {kinds.join(', ')}</span>{/if}
</a>

<style>
  .tile { --art: 88px; position: relative; display: flex; flex-direction: column; align-items: center; gap: 2px; min-width: 0;
    height: 100%; padding: 14px 10px 12px; border: 1px solid var(--border); border-radius: var(--radius-card);
    background: var(--surface); text-align: center;
    transition: background-color var(--fast), border-color var(--fast), transform var(--dur-press) ease-out; }
  .compact { --art: 60px; padding: 10px 8px 9px; }
  .tile:active { transform: scale(var(--press)); }
  .tile:focus-visible { outline-offset: 2px; border-radius: var(--radius-card); }
  .status { position: absolute; top: 6px; left: 6px; z-index: 1; display: flex; gap: 4px; max-width: calc(100% - 12px); }
  /* A fixed art box: however wide the picture, it fits inside (contain) and never widens the tile. */
  .art { position: relative; display: grid; grid-template-columns: minmax(0, 1fr); place-items: center; width: 100%; min-width: 0;
    height: var(--art); margin-bottom: 8px; overflow: hidden; }
  .pic { grid-area: 1 / 1; display: block; height: calc(var(--art) * .8); width: auto; min-width: 0; max-width: 100%; object-fit: contain; }
  .moving { opacity: 0; transition: opacity var(--dur-color); }
  .fade { opacity: 0; }
  .pic:not(.fade):not(.moving) { transition: opacity var(--dur-color); }
  .moving.loaded { opacity: 1; }
  .processing { font-size: var(--t-fine); color: var(--muted); }
  /* Truncated names stay centred: the bdi is a block that ends in an ellipsis. */
  .name bdi, .by bdi { display: block; overflow: hidden; text-overflow: ellipsis; }
  .name { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--t-nav); font-weight: 500; line-height: 1.35; color: var(--text); }
  .compact .name { font-size: var(--t-fine); }
  .by { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: 12px; line-height: 1.4; color: var(--muted); }
  .kinds { position: absolute; left: 50%; bottom: 0; translate: -50% 0; max-width: 100%; overflow: hidden; text-overflow: ellipsis;
    padding: 0 6px; border-radius: var(--radius);
    background: var(--elevated-2); color: var(--muted); font-size: 12px; font-weight: 500; line-height: 16px; white-space: nowrap;
    opacity: 0; transition: opacity var(--fast) ease-out; pointer-events: none; }
  /* Picked: bold name, accent border and a tick; never a filled box. */
  .tick { position: absolute; top: 6px; right: 6px; display: inline-grid; place-items: center; width: 20px; height: 20px;
    border: 1px solid var(--border-strong); border-radius: var(--radius); color: transparent; background: var(--bg); }
  .picked { border-color: var(--accent); }
  .picked .tick { border-color: var(--accent); color: var(--accent-text); }
  .picked .name { font-weight: 700; }
  .tile:focus-visible .kinds { opacity: 1; }
  @media (hover: hover) and (pointer: fine) {
    .tile:hover { background: var(--surface-hover); border-color: var(--border-strong); }
    .tile:hover .kinds { opacity: 1; }
  }
  @media (prefers-contrast: more) { .tile { border-color: var(--border-strong); } }
</style>
