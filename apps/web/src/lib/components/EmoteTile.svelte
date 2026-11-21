<script lang="ts" module>
  import type { Picture } from '$lib/emote';

  /** What a tile needs: the emote's pictures, id and name; owner and flags when known. */
  export type CardEmote = Picture & {
    id: string;
    name: string;
    owner?: { id?: string; displayName: string } | null;
    flags?: { zeroWidth: boolean; private?: boolean } | null;
    /** Who can find it (EmoteVisibility); your own unlisted and private emotes get a mark. */
    visibility?: string | null;
    /** The name it has in a set, when that differs from its own. */
    shownName?: string;
  };
  export type Density = 'comfortable' | 'compact';
  /** The art box's height per density (keep in step with the tile CSS below). */
  export const TILE_ART = { comfortable: 88, compact: 60 } as const;

  /** A status note's tint: the emote states get their colour, anything else stays neutral. */
  /** The mark on your own emote's tile when not everyone can find it: the emote page's words (flagsOf). */
  export function reachMark(e: CardEmote, me: string | null | undefined): { key: 'unlisted' | 'private'; label: string; tip: string } | null {
    if (!me || !e.owner?.id || e.owner.id !== me) return null;
    if (e.visibility === 'PRIVATE' || e.flags?.private) return { key: 'private', label: 'Private', tip: 'Only you and your editors can add it to sets' };
    if (e.visibility === 'UNLISTED') return { key: 'unlisted', label: 'Unlisted', tip: 'Search leaves it out: only people with the link find it' };
    return null;
  }

  export function noteTone(note: string): string {
    const n = note.toLowerCase().replace(/\s+/g, '_');
    return ['pending', 'approved', 'rejected', 'taken_down'].includes(n) ? n : '';
  }
</script>

<script lang="ts">
  // One emote in a grid, the same tile everywhere emotes are listed (browse, creator, channel,
  // set, your emotes, admin review): a surface card with a hairline, the art in a fixed box (no
  // layout shift), the name, the creator. The whole tile is one link; hover steps the surface and
  // the border, it never lifts. Animated emotes always play (EmotePicture: the still first frame
  // under reduced motion). The `status` slot (or a `note`, e.g. "pending") sits in the top-left
  // corner, for /me and the admin pages; `rank` (the Trending view) sits there too, as "#1".
  // The creator's name is drawn in their paint (no badge): every tile on the page asks for its
  // creator in the same tick, so a grid costs one request per 100 new creators (wantLooks).
  import type { Snippet } from 'svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { isWide, ready, size } from '$lib/emote';
  import { clean } from '$lib/format';
  import EmotePicture from './EmotePicture.svelte';
  import Icon from './Icon.svelte';
  import { session } from '$lib/tool/session.svelte';

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
    /** Its place in a ranking (the Trending view), shown as "#1" in the corner. */
    rank?: number;
    /** The browse target set's name when the emote is already in it: an "In set" mark. */
    inSet?: string;
  };
  let { emote: e, density = 'comfortable', creator = true, labels = true, href, eager = false, note, status, onpick, selected = false, rank, inSet }: Props = $props();
  const reach = $derived(reachMark(e, session.me?.id));
  const name = $derived(clean(e.shownName ?? e.name));
  const owner = $derived(e.owner ? clean(e.owner.displayName) : '');
  const ownerId = $derived(creator ? e.owner?.id : undefined);
  $effect(() => {
    if (ownerId) wantLooks([ownerId]);
  });
  const paint = $derived(ownerId ? (looks[ownerId]?.paint ?? null) : null);
  // A lazy picture that is already cached shows at once; one that loads late fades in.
  function settle(node: HTMLImageElement) {
    if (node.complete || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    node.classList.add('fade');
    node.addEventListener('load', () => node.classList.remove('fade'), { once: true });
    node.addEventListener('error', () => node.classList.remove('fade'), { once: true });
  }
  const wide = $derived(isWide(e));
  const dims = $derived(size(e));
  // The picture fills 80% of the box height; its shown width picks the right file.
  const shownW = $derived(Math.round((TILE_ART[density] * 0.8 * dims.width) / dims.height));
  // Type marks (animated, zero-width, wide) are in the link's name only; nothing is drawn over the
  // picture (every emote plays anyway, and a label over the art read as a stray tag).
  const kinds = $derived(
    (labels ? [e.animated && 'animated', e.flags?.zeroWidth && 'zero-width', wide && 'wide'] : []).filter(Boolean) as string[],
  );
  const title = $derived(owner && creator ? `${name} by ${owner}` : name);
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
>
  {#if onpick}
    <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>
  {/if}
  {#if status || note || rank || reach || inSet}
    <span class="status">
      {#if status}{@render status()}{:else if note}<span class="ui-chip {noteTone(note)}">{note}</span>{/if}
      {#if rank}<span class="rank" class:top={rank <= 3}><span class="sr-only">Trending </span>#{rank}</span>{/if}
      {#if reach}<span class="mark" title={reach.tip}><Icon name={reach.key === 'private' ? 'shield' : 'link'} size={12} /><span class="mk">{reach.label}</span></span>{/if}
      {#if inSet}<span class="mark in" title="Already in {inSet}"><Icon name="check" size={12} /><span class="mk">In <bdi>{inSet}</bdi></span></span>{/if}
    </span>
  {/if}
  <span class="art">
    {#if ready(e)}
      <EmotePicture emote={e} class="pic" shown={shownW} {eager} use={settle} />
    {:else}
      <span class="processing">Processing</span>
    {/if}
  </span>
  <span class="name"><bdi title={name}>{name}</bdi></span>
  {#if creator && owner}<span class="by"><PaintedName name={owner} {paint} /></span>{/if}
  {#if kinds.length}<span class="sr-only">, {kinds.join(', ')}</span>{/if}
  {#if reach}<span class="sr-only">, {reach.label.toLowerCase()}</span>{/if}
  {#if inSet}<span class="sr-only">, already in {inSet}</span>{/if}
</a>

<style>
  .tile { --art: 88px; position: relative; display: flex; flex-direction: column; align-items: center; gap: 2px; min-width: 0;
    height: 100%; padding: 14px 10px 12px; border: 1px solid var(--border); border-radius: var(--radius-card);
    background: var(--surface); text-align: center;
    transition: background-color var(--fast), border-color var(--fast), transform var(--dur-press) ease-out; }
  .compact { --art: 60px; padding: 10px 8px 9px; }
  .tile:active { transform: scale(var(--press)); }
  .tile:focus-visible { outline-offset: 2px; border-radius: var(--radius-card); }
  .status { position: absolute; top: 6px; left: 6px; z-index: 1; display: flex; gap: 4px; max-width: calc(100% - 48px); }
  /* A fixed art box: however wide the picture, it fits inside (contain) and never widens the tile. */
  .art { position: relative; display: grid; grid-template-columns: minmax(0, 1fr); place-items: center; width: 100%; min-width: 0;
    height: var(--art); margin-bottom: 8px; overflow: hidden; }
  .art :global(.pic) { grid-area: 1 / 1; display: block; height: calc(var(--art) * .8); width: auto; min-width: 0; max-width: 100%; object-fit: contain; }
  .art :global(.pic.fade) { opacity: 0; }
  .art :global(.pic:not(.fade)) { transition: opacity var(--dur-color); }
  /* The rank: plain tabular text on the tile's ground; the top three in the accent. */
  .rank { display: inline-flex; align-items: center; height: 20px; padding: 0 6px; border: 1px solid var(--border); border-radius: var(--radius);
    background: var(--bg); color: var(--muted); font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums; line-height: 1; }
  .rank.top { color: var(--accent-text); border-color: var(--accent-border); }
  /* Your own unlisted or private emote, and "In set" while browsing for a set: quiet chips on the
     tile's ground, an icon and a word; In set's tick is the accent (it answers the target set). */
  .mark { display: inline-flex; align-items: center; gap: 4px; min-width: 0; height: 20px; padding: 0 6px; border: 1px solid var(--border);
    border-radius: var(--radius); background: var(--bg); color: var(--muted); font-size: 12px; font-weight: 600; line-height: 1; white-space: nowrap; }
  .mark :global(svg) { flex: none; color: var(--dim); }
  .mark.in :global(svg) { color: var(--accent-text); }
  .mark.in { color: var(--text); }
  .mark .mk { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .compact .mk { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
  .compact .mark { padding: 0 4px; }
  .processing { font-size: var(--t-fine); color: var(--muted); }
  /* Truncated names stay centred: the bdi is a block that ends in an ellipsis. */
  .name bdi { display: block; overflow: hidden; text-overflow: ellipsis; }
  /* The creator is always a PaintedName (plain until the paint arrives), so the box never changes
     when it does: the paint only colours the text. It shrinks to the tile and ends in an ellipsis. */
  .name { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--t-nav); font-weight: 500; line-height: 1.35; color: var(--text); }
  .compact .name { font-size: var(--t-fine); }
  .by { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: 12px; line-height: 1.4; color: var(--muted); }
  /* Picked: an accent hairline, an accent tick and a bold name; never a filled box. */
  .tick { position: absolute; top: 6px; right: 6px; display: inline-grid; place-items: center; width: 20px; height: 20px;
    border: 1px solid var(--border-strong); border-radius: var(--radius); color: transparent; background: var(--bg); }
  .picked, .picked .tick { border-color: var(--accent-border); }
  .picked .tick { color: var(--accent-text); }
  .picked .name { font-weight: 700; }
  @media (hover: hover) and (pointer: fine) {
    .tile:hover { background: var(--surface-hover); border-color: var(--border-strong); }
    .tile.picked:hover { border-color: var(--accent-text); }
  }
  @media (prefers-contrast: more) { .tile { border-color: var(--border-strong); } }
</style>
