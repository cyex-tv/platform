<script lang="ts">
  // One item for sale, a cell in the shop's hairline grid: what it looks like on your name, its
  // name and price, what it holds, and one button. The name is a real link (/shop?item=<id>) that
  // opens the item (the right panel, or the dialog on narrow windows), so it can be opened in a
  // new tab too. The open item's cell is ringed in the accent (aria-current). Owned items say so
  // (bold, accent tick) and offer a gift instead.
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import ItemPreview from './ItemPreview.svelte';
  import { contentsText, isBundle, price, saleEndText, stockText } from './shop';

  type Props = {
    item: ShopItemFieldsFragment;
    name: string;
    /** This item is the one open in the panel. */
    current?: boolean;
    onopen: (item: ShopItemFieldsFragment, gift: boolean) => void;
  };
  let { item, name, current = false, onopen }: Props = $props();

  const stock = $derived(stockText(item));
  const ends = $derived(saleEndText(item));
  const soldOut = $derived(item.stockLeft === 0);
</script>

<li class="cell" class:gone={soldOut} class:on={current}>
  <ItemPreview badges={item.badges} paints={item.paints} {name} />
  <div class="info">
    <h2 class="nm">
      <a href="/shop?item={encodeURIComponent(item.id)}" aria-current={current ? 'true' : undefined} data-item={item.id}
        onclick={(e) => { if (e.metaKey || e.ctrlKey || e.shiftKey || e.button) return; e.preventDefault(); onopen(item, false); }}><bdi>{clean(item.name)}</bdi></a>
    </h2>
    <p class="price">{price(item.priceCents, item.currency)}</p>
  </div>
  <p class="facts">
    <span class="holds">{#if isBundle(item)}<span class="kind">Bundle</span><span class="sep" aria-hidden="true">·</span>{/if}<span>{contentsText(item)}</span></span>
    {#if stock && !soldOut}<span class:low={(item.stockLeft ?? 99) <= 10}>{stock}</span>{/if}
    {#if ends}<span>{ends}</span>{/if}
  </p>
  <div class="act">
    {#if item.owned}
      <span class="own"><Icon name="check" size={15} stroke={2.4} />Owned</span>
      <button class="btn btn-s" type="button" disabled={soldOut} onclick={() => onopen(item, true)}>Gift</button>
    {:else if soldOut}
      <span class="muted">Sold out</span>
    {:else}
      <button class="btn btn-s btn-primary" type="button" onclick={() => onopen(item, false)}>Buy</button>
      <button class="btn btn-s" type="button" onclick={() => onopen(item, true)}>Gift</button>
    {/if}
  </div>
</li>

<style>
  p, h2 { margin: 0; }
  /* Hairlines right of and under every cell; the grid's frame clips the outer ones. */
  .cell { display: grid; align-content: start; gap: 12px; min-width: 0; padding: 16px; box-shadow: 1px 0 0 var(--border), 0 1px 0 var(--border);
    transition: background-color var(--fast); }
  .cell.on { box-shadow: inset 0 0 0 1px var(--accent-border), 1px 0 0 var(--border), 0 1px 0 var(--border); }
  .gone .info, .gone .facts { opacity: .7; }
  .info { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; min-width: 0; }
  .nm { min-width: 0; font-size: var(--t-ui); font-weight: 600; line-height: 1.35; }
  /* a touch target of at least 24px (WCAG 2.5.8) */
  .nm a { display: inline-flex; align-items: center; min-height: 24px; color: var(--text); overflow-wrap: anywhere; text-decoration: none; }
  .nm a[aria-current='true'] { font-weight: 700; }
  .price { flex: none; font-size: var(--t-ui); font-weight: 600; font-variant-numeric: tabular-nums; }
  .facts { display: flex; flex-wrap: wrap; gap: 2px 12px; margin-top: -6px; font-size: var(--t-fine); color: var(--muted); font-variant-numeric: tabular-nums; }
  .holds { display: inline-flex; gap: 6px; }
  .kind { color: var(--text); font-weight: 600; }
  .sep { color: var(--dim); }
  .low { color: var(--text); font-weight: 600; }
  .act { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 2px; }
  .own { display: inline-flex; align-items: center; gap: 6px; min-height: 40px; padding-right: 8px; font-size: var(--t-small); font-weight: 700; color: var(--text); }
  .own :global(svg) { color: var(--accent-text); }
  .muted { min-height: 40px; display: inline-flex; align-items: center; font-size: var(--t-small); color: var(--muted); }
  @media (hover: hover) and (pointer: fine) {
    .cell:hover { background: var(--row-hover); }
    .nm a:hover { text-decoration: underline; text-underline-offset: 3px; }
  }
</style>
