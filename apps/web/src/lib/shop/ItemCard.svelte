<script lang="ts">
  // One item for sale: what it looks like on your name, what's in it, the price, and one button.
  // The name is a real link (/shop?item=<id>) that opens the item dialog, so it can be opened in
  // a new tab too. Owned items say so (bold, accent tick) and offer a gift instead.
  import Icon from '$lib/components/Icon.svelte';
  import { clean, count } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import ItemPreview from './ItemPreview.svelte';
  import { price, saleEndText, stockText } from './shop';

  type Props = { item: ShopItemFieldsFragment; name: string; onopen: (item: ShopItemFieldsFragment, gift: boolean) => void };
  let { item, name, onopen }: Props = $props();

  const parts = $derived(
    [
      item.badges.length ? `${count(item.badges.length)} ${item.badges.length === 1 ? 'badge' : 'badges'}` : '',
      item.paints.length ? `${count(item.paints.length)} ${item.paints.length === 1 ? 'paint' : 'paints'}` : '',
    ]
      .filter(Boolean)
      .join(' and '),
  );
  const stock = $derived(stockText(item));
  const ends = $derived(saleEndText(item));
  const soldOut = $derived(item.stockLeft === 0);
</script>

<li class="card" class:gone={soldOut}>
  <ItemPreview badges={item.badges} paints={item.paints} {name} />
  <div class="info">
    <h2 class="nm">
      <a href="/shop?item={encodeURIComponent(item.id)}" onclick={(e) => { if (e.metaKey || e.ctrlKey || e.shiftKey || e.button) return; e.preventDefault(); onopen(item, false); }}><bdi>{clean(item.name)}</bdi></a>
    </h2>
    <p class="price">{price(item.priceCents, item.currency)}</p>
  </div>
  <p class="facts">
    <span>{parts}</span>
    {#if stock && !soldOut}<span class:low={(item.stockLeft ?? 99) <= 10}>{stock}</span>{/if}
    {#if ends}<span>{ends}</span>{/if}
  </p>
  <div class="act">
    {#if item.owned}
      <span class="own"><Icon name="check" size={15} />Owned</span>
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
  .card { display: grid; align-content: start; gap: 12px; min-width: 0; padding: 14px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .gone .info, .gone .facts { opacity: .7; }
  .info { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; min-width: 0; }
  .nm { margin: 0; min-width: 0; font-size: var(--t-ui); font-weight: 600; line-height: 1.35; }
  .nm a { color: var(--text); overflow-wrap: anywhere; text-decoration: none; }
  @media (hover: hover) and (pointer: fine) { .nm a:hover { text-decoration: underline; text-underline-offset: 3px; } }
  .price { margin: 0; flex: none; font-size: var(--t-ui); font-weight: 600; font-variant-numeric: tabular-nums; }
  .facts { display: flex; flex-wrap: wrap; gap: 2px 12px; margin: -4px 0 0; font-size: var(--t-fine); color: var(--muted); font-variant-numeric: tabular-nums; }
  .low { color: var(--text); font-weight: 600; }
  .act { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 2px; }
  .own { display: inline-flex; align-items: center; gap: 6px; min-height: 40px; padding-right: 8px; font-size: var(--t-small); font-weight: 700; color: var(--text); }
  .own :global(svg) { color: var(--accent-text); }
  .muted { min-height: 40px; display: inline-flex; align-items: center; font-size: var(--t-small); color: var(--muted); }
  /* a touch target of at least 24px (WCAG 2.5.8) */
  .nm a { display: inline-flex; align-items: center; min-height: 24px; }
</style>
