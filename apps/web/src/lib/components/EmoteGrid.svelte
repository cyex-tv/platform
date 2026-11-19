<script lang="ts">
  // A grid of EmoteTiles (the 7TV-style grid: ~150px tiles, 16px gap). One size everywhere; the
  // compact density is only for small pickers (adding emotes to a set), never a switch. Every tile
  // in a row is as tall as the row (the cells stretch, the tile fills its cell). Wide emotes take
  // two cells (the grid packs densely so no holes open). While `loading`, skeleton tiles follow
  // the items (or stand alone on a first load). `ranked` numbers the tiles (#1, #2 …) for Trending.
  import type { Snippet } from 'svelte';
  import { isWide } from '$lib/emote';
  import EmoteSkeleton from './EmoteSkeleton.svelte';
  import EmoteTile, { type CardEmote, type Density } from './EmoteTile.svelte';

  type Props = {
    items: CardEmote[];
    density?: Density;
    creator?: boolean;
    labels?: boolean;
    loading?: boolean;
    skeletons?: number;
    empty?: string;
    /** How many of the first tiles load eagerly (above the fold). */
    eager?: number;
    label?: string;
    href?: (e: CardEmote) => string;
    /** A short status per tile (a chip in its corner), or nothing. */
    note?: (e: CardEmote) => string | undefined;
    /** Your own corner content per tile, instead of `note`. */
    status?: Snippet<[CardEmote]>;
    /** Controls under each tile (rename, remove, add), outside the tile's link. */
    below?: Snippet<[CardEmote]>;
    /** Select mode: picking toggles `id` in `picked` through `onpick`; tiles stop being links. */
    picked?: ReadonlySet<string>;
    onpick?: (e: CardEmote) => void;
    /** Number the tiles from 1 in this order (a ranking, e.g. Trending). */
    ranked?: boolean;
  };
  let {
    items,
    density = 'comfortable',
    creator = true,
    labels = true,
    loading = false,
    skeletons = 12,
    empty = 'Nothing here yet.',
    eager = 0,
    label,
    href,
    note,
    status: corner,
    below,
    picked,
    onpick,
    ranked = false,
  }: Props = $props();
</script>

{#if items.length === 0 && !loading}
  <p class="none">{empty}</p>
{:else}
  <ul class="grid {density}" aria-label={label} aria-busy={loading}>
    {#each items as e, i (e.id + (e.shownName ?? ''))}
      <li class:wide={isWide(e)}>
        {#if corner}
          <EmoteTile emote={e} {density} {creator} {labels} eager={i < eager} href={href?.(e)}>
            {#snippet status()}{@render corner(e)}{/snippet}
          </EmoteTile>
        {:else}
          <EmoteTile emote={e} {density} {creator} {labels} eager={i < eager} href={href?.(e)} note={note?.(e)}
            onpick={onpick ? () => onpick(e) : undefined} selected={picked?.has(e.id) ?? false} rank={ranked ? i + 1 : undefined} />
        {/if}
        {#if below}<div class="below">{@render below(e)}</div>{/if}
      </li>
    {/each}
    {#if loading}
      {#each Array(skeletons) as _, i (i)}
        <li><EmoteSkeleton {density} {creator} /></li>
      {/each}
    {/if}
  </ul>
{/if}

<style>
  .none { padding: 24px 0; color: var(--muted); }
  .grid { list-style: none; margin: 0; padding: 0; display: grid; gap: 16px;
    grid-template-columns: repeat(auto-fill, minmax(var(--emote-tile), 1fr)); grid-auto-flow: row dense; }
  .compact { grid-template-columns: repeat(auto-fill, minmax(var(--emote-tile-compact), 1fr)); gap: 8px; }
  .wide { grid-column: span 2; }
  li { min-width: 0; }
  li:has(.below) { display: grid; grid-template-rows: 1fr auto; gap: 6px; }
  .below { display: flex; align-items: center; justify-content: center; gap: 4px; min-height: 40px; min-width: 0; }
  @media (max-width: 520px) {
    .grid { gap: 8px; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); }
    .compact { grid-template-columns: repeat(auto-fill, minmax(96px, 1fr)); }
  }
</style>
