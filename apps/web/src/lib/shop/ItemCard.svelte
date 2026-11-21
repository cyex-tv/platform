<script lang="ts">
  // One item for sale, a cell of the shop's contact sheet (cells share hairlines, no gaps): the
  // item drawn on your own name on a stage (the name large, then a chat line at real chat size),
  // then its name, its price in mono, what it holds and how many are left. The whole cell is one
  // link (/shop?item=<id>) that opens the item in the inspector on the right (or the dialog on
  // narrow windows), so it opens in a new tab too. The open item is bold with an accent tick (no
  // fill, no ring). Owned items say so with the accent tick.
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
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
  const badge = $derived(item.badges[0] ?? null);
  const paint = $derived(item.paints[0] ?? null);
  const kind = $derived(isBundle(item) ? 'Bundle' : item.paints.length ? 'Paint' : item.badges.length ? 'Badge' : 'Item');
</script>

<li class="cell" class:gone={soldOut} class:on={current}>
  <a class="hit" href="/shop?item={encodeURIComponent(item.id)}" aria-current={current ? 'true' : undefined} data-item={item.id}
    onclick={(e) => { if (e.metaKey || e.ctrlKey || e.shiftKey || e.button) return; e.preventDefault(); onopen(item, false); }}>
    <span class="stage" aria-hidden="true">
      <span class="big"><PaintedName {name} {paint} {badge} badgeSize={22} /></span>
      <span class="chat"><PaintedName {name} {paint} {badge} /><span class="msg">: hi chat</span></span>
      {#if item.badges.length && !item.paints.length && badge}
        <span class="sizes">{#each [18, 36, 54] as s (s)}<BadgeIcon {badge} size={s} />{/each}</span>
      {/if}
    </span>
    <span class="info">
      <span class="nm">{#if current}<Icon name="check" size={13} stroke={2.6} />{/if}<bdi>{clean(item.name)}</bdi></span>
      <span class="price">{price(item.priceCents, item.currency)}</span>
    </span>
    <span class="facts"><span class="kind">{kind}</span><span class="dot" aria-hidden="true">·</span><span>{contentsText(item)}</span></span>
    <span class="state">
      {#if item.owned}<span class="own"><Icon name="check" size={12} stroke={2.6} />Owned</span>
      {:else if soldOut}<span class="mono">Sold out</span>
      {:else if stock}<span class="mono" class:low={(item.stockLeft ?? 99) <= 10}>{stock}</span>
      {:else}<span class="mono">On sale</span>{/if}
      {#if ends}<span class="mono">{ends}</span>{/if}
    </span>
  </a>
</li>

<style>
  /* Hairlines right of and under every cell; the sheet draws the top and left ones. */
  .cell { min-width: 0; box-shadow: 1px 0 0 var(--border), 0 1px 0 var(--border); }
  .hit { display: grid; grid-template-rows: auto auto auto 1fr; height: 100%; min-width: 0; color: var(--text); text-decoration: none;
    transition: background-color var(--fast); }
  .hit:focus-visible { outline-offset: -2px; }
  /* The stage: the page's own chat ground, the item at the size people see it. */
  .stage { position: relative; display: grid; place-items: center; align-content: center; gap: 10px; height: 148px; padding: 20px 16px 14px;
    border-bottom: 1px solid var(--border); overflow: hidden; }
  .big { display: flex; max-width: 100%; min-width: 0; font-size: 20px; font-weight: 700; letter-spacing: -.01em; }
  .chat { display: flex; align-items: center; max-width: 100%; min-width: 0; font-size: 13px; font-weight: 700; }
  .msg { flex: none; font-weight: 400; color: var(--muted); }
  .sizes { display: flex; align-items: flex-end; gap: 12px; }
  .info { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; min-width: 0; padding: 12px 14px 2px; }
  .nm { display: inline-flex; align-items: baseline; gap: 5px; min-width: 0; font-size: var(--t-small); font-weight: 600; line-height: 1.35; }
  .nm bdi { min-width: 0; overflow-wrap: anywhere; }
  .nm :global(svg) { flex: none; align-self: center; color: var(--accent-text); }
  .on .nm { font-weight: 700; }
  .price { flex: none; font-family: var(--font-mono); font-size: var(--t-small); font-weight: 600; font-variant-numeric: tabular-nums; }
  .facts { display: flex; flex-wrap: wrap; align-items: center; gap: 2px 6px; min-width: 0; padding: 2px 14px 0; font-size: var(--t-fine); color: var(--muted); }
  .kind { color: var(--text); font-weight: 600; }
  .dot { color: var(--dim); }
  .state { display: flex; flex-wrap: wrap; align-items: flex-start; gap: 2px 12px; padding: 6px 14px 12px; font-size: var(--t-fine); }
  .mono { font-family: var(--font-mono); font-variant-numeric: tabular-nums; color: var(--dim); }
  .low { color: var(--text); font-weight: 600; }
  .own { display: inline-flex; align-items: center; gap: 4px; font-weight: 700; color: var(--text); }
  .own :global(svg) { color: var(--accent-text); }
  .gone .stage { opacity: .55; }
  @media (hover: hover) and (pointer: fine) {
    .hit:hover { background: var(--row-hover); }
  }
</style>
