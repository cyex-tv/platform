<script lang="ts">
  // A grid of EmoteCards. Wide emotes take two cells (the grid packs densely so no holes
  // open). While `loading`, skeleton tiles follow the items (or stand alone on a first load).
  import { isWide } from '$lib/emote';
  import EmoteCard, { type CardEmote, type Density } from './EmoteCard.svelte';
  import EmoteSkeleton from './EmoteSkeleton.svelte';

  type Props = {
    items: CardEmote[];
    density?: Density;
    creator?: boolean;
    labels?: boolean;
    loading?: boolean;
    skeletons?: number;
    empty?: string;
    /** How many of the first cards load eagerly (above the fold). */
    eager?: number;
    label?: string;
    href?: (e: CardEmote) => string;
    /** A short status per card (shown as a label), or nothing. */
    note?: (e: CardEmote) => string | undefined;
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
  }: Props = $props();
</script>

{#if items.length === 0 && !loading}
  <p class="none">{empty}</p>
{:else}
  <ul class="grid {density}" aria-label={label} aria-busy={loading}>
    {#each items as e, i (e.id + (e.shownName ?? ''))}
      <li class:wide={isWide(e)}>
        <EmoteCard emote={e} {density} {creator} {labels} eager={i < eager} href={href?.(e)} note={note?.(e)} />
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
  .grid { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px;
    grid-template-columns: repeat(auto-fill, minmax(136px, 1fr)); grid-auto-flow: row dense; }
  .compact { grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); gap: 2px; }
  .wide { grid-column: span 2; }
  li { min-width: 0; }
  @media (max-width: 360px) { .grid { grid-template-columns: repeat(auto-fill, minmax(124px, 1fr)); } }
</style>
