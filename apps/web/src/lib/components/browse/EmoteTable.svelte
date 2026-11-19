<script lang="ts" module>
  import { wantLooks, type Look } from '$lib/cosmetics/store.svelte';
  import type { CardEmote } from '../EmoteTile.svelte';

  /** What a table row shows beyond a tile: when it was added and how many channels use it. */
  export type RowEmote = CardEmote & {
    animated?: boolean;
    createdAt?: string;
    channelCount?: number;
    owner?: { id?: string; displayName: string } | null;
  };
  export type TableSort = 'relevance' | 'top' | 'new';
</script>

<script lang="ts">
  // The table view of emotes: preview, name, creator (in their paint when the shared cosmetics
  // cache knows it), type, channels using it, and the day it was added. Channels and Added sort
  // by the same orders as the sort control (Top, New) and carry aria-sort. In select mode a
  // checkbox leads each row (Space toggles it); the name stays the link to the emote.
  import EmotePicture from '$lib/components/EmotePicture.svelte';
  import { isWide, ready } from '$lib/emote';
  import { clean, count, date } from '$lib/format';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import Icon from '../Icon.svelte';
  import { COLUMN_SORT } from './params';

  type Props = {
    items: RowEmote[];
    sort?: TableSort;
    /** Without it the Channels and Added headers are plain text (a list that has one fixed order). */
    onsort?: (s: 'top' | 'new') => void;
    looks?: Record<string, Look>;
    picked?: ReadonlySet<string>;
    onpick?: (e: RowEmote) => void;
    label?: string;
    eager?: number;
  };
  let { items, sort = 'relevance', onsort, looks = {}, picked, onpick, label = 'Emotes', eager = 0 }: Props = $props();
  // The creators' paints and badges, one batched ask for the whole table (shared cosmetics cache).
  $effect(() => wantLooks(items.map((e) => e.owner?.id)));

  const kinds = (e: RowEmote) =>
    [e.animated ? 'Animated' : 'Still', e.flags?.zeroWidth && 'zero-width', isWide(e) && 'wide'].filter(Boolean).join(', ');
  const aria = (col: keyof typeof COLUMN_SORT) => (sort === COLUMN_SORT[col] ? 'descending' : undefined);
</script>

<div class="wrap">
  <table class:pick={!!onpick}>
    <caption class="sr-only">{label}</caption>
    <thead>
      <tr>
        {#if onpick}<th class="chk"><span class="sr-only">Select</span></th>{/if}
        <th class="prev"><span class="sr-only">Preview</span></th>
        <th class="nm">Name</th>
        <th class="by">Creator</th>
        <th class="ty">Type</th>
        <th class="num" aria-sort={onsort ? aria('channels') : undefined}>
          {#if onsort}<button type="button" class:on={sort === 'top'} onclick={() => onsort('top')}>Channels{#if sort === 'top'}<Icon name="arrow-down" size={14} />{/if}</button>{:else}Channels{/if}
        </th>
        <th class="when" aria-sort={onsort ? aria('added') : undefined}>
          {#if onsort}<button type="button" class:on={sort === 'new'} onclick={() => onsort('new')}>Added{#if sort === 'new'}<Icon name="arrow-down" size={14} />{/if}</button>{:else}Added{/if}
        </th>
      </tr>
    </thead>
    <tbody>
      {#each items as e, i (e.id + (e.shownName ?? ''))}
        {@const on = picked?.has(e.id) ?? false}
        {@const look = e.owner?.id ? looks[e.owner.id] : undefined}
        <tr class:on>
          {#if onpick}
            <td class="chk">
              <input type="checkbox" checked={on} onchange={() => onpick(e)} aria-label="Select {clean(e.name)}" />
            </td>
          {/if}
          <td class="prev" aria-hidden="true">
            <span class="box">
              {#if ready(e)}
                <EmotePicture emote={e} class="pic" eager={i < eager} />
              {/if}
            </span>
          </td>
          <th class="nm" scope="row"><a href="/emotes/{e.id}"><bdi>{clean(e.name)}</bdi></a></th>
          <td class="by">
            {#if e.owner}
              {#if e.owner.id}<a href="/users/{e.owner.id}"><PaintedName name={e.owner.displayName} paint={look?.paint} badge={look?.badge} /></a>
              {:else}<bdi>{clean(e.owner.displayName)}</bdi>{/if}
            {:else}<span class="none">None</span>{/if}
          </td>
          <td class="ty">{kinds(e)}</td>
          <td class="num">{count(e.channelCount ?? 0)}</td>
          <td class="when">{e.createdAt ? date(e.createdAt) : ''}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .wrap { overflow-x: auto; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  table { width: 100%; border-collapse: collapse; font-size: var(--t-nav); }
  th, td { padding: 6px 12px; text-align: left; vertical-align: middle; font-weight: 400; white-space: nowrap; }
  thead th { height: 40px; font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim);
    border-bottom: 1px solid var(--border); padding-block: 0; }
  tbody tr + tr > * { border-top: 1px solid var(--border); }
  .chk { width: 44px; padding-inline: 12px 0; }
  .chk input { width: 18px; height: 18px; margin: 0; accent-color: var(--accent); cursor: pointer; vertical-align: middle; }
  .prev { width: 64px; padding-right: 0; }
  .box { display: grid; place-items: center; width: 48px; height: 40px; }
  .box :global(.pic) { max-width: 100%; max-height: 100%; width: auto; height: auto; object-fit: contain; }
  .nm { max-width: 260px; }
  .nm a { display: block; overflow: hidden; text-overflow: ellipsis; color: var(--text); font-weight: 500; }
  .by { max-width: 220px; color: var(--muted); }
  .by a { display: inline-flex; max-width: 100%; color: inherit; }
  .none { color: var(--dim); }
  .ty { color: var(--muted); }
  .num, .when { font-variant-numeric: tabular-nums; }
  thead .num, .num { text-align: right; }
  .when { color: var(--muted); }
  /* Sortable headers are buttons that read as the header text; the sorted one is bold with an arrow. */
  th button { display: inline-flex; align-items: center; gap: 4px; height: 40px; padding: 0; border: 0; background: none; color: inherit;
    font: inherit; letter-spacing: inherit; text-transform: inherit; cursor: pointer; transition: color var(--fast); }
  th button.on { color: var(--text); font-weight: 700; }
  th button:focus-visible { outline-offset: 2px; border-radius: var(--radius); }
  /* A picked row: bold name and the checked box, never a filled row. */
  tr.on .nm a { font-weight: 700; }
  tbody tr:has(.nm a:focus-visible) { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) {
    tbody tr:hover { background: var(--row-hover); }
    th button:hover { color: var(--text); }
    .nm a:hover { text-decoration: underline; }
  }
  @media (max-width: 760px) { .ty, .by { display: none; } th, td { padding-inline: 8px; } .nm { max-width: none; } }
  @media (max-width: 520px) { .when { display: none; } }
  @media (pointer: coarse) { .chk input { width: 22px; height: 22px; } tbody td, tbody th { padding-block: 8px; } }
  @media (forced-colors: active) { tr.on { outline: 2px solid Highlight; outline-offset: -2px; } }
</style>
