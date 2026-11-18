<script lang="ts">
  // /shop: what is on sale, each item drawn on your own name (or a made-up one signed out), buy it
  // for yourself or give it to a Twitch or Kick login. Public, signed out too; an item opens in a
  // dialog at /shop?item=<id>, so it has an address to share and to come back to after signing in.
  // Paying happens in Paddle's window; CheckoutStatus shows the wait and the result.
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { loadOwned } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import { SHOP_ITEM, SHOP_ITEMS } from '$lib/queries';
  import { flow } from '$lib/shop/checkout.svelte';
  import CheckoutStatus from '$lib/shop/CheckoutStatus.svelte';
  import ItemCard from '$lib/shop/ItemCard.svelte';
  import ItemDialog from '$lib/shop/ItemDialog.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  const me = $derived(session.me);
  const name = $derived(clean(me?.identities[0]?.displayName) || 'lumen_fox');

  // The list the server drew, replaced by one asked with the session once we know who is here
  // (owned flags), and again after a purchase.
  let fresh = $state<ShopItemFieldsFragment[] | null>(null);
  const items = $derived(fresh ?? data.items);

  async function refresh() {
    try {
      fresh = (await gql(SHOP_ITEMS, { all: false })).shopItems;
    } catch {
      // The server's list stays.
    }
  }
  $effect(() => {
    if (me) refresh();
  });
  $effect(() => {
    if (flow.phase === 'paid') refresh();
  });
  // Your look (for the account menu) doesn't change here, but a bought paint should show at once in Cosmetics.
  $effect(() => {
    if (flow.phase === 'paid' && me) loadOwned(me.id, true, true).catch(() => {});
  });

  // The open item: from the address. A hidden or scheduled item (staff) isn't in the list, so it is asked for.
  const wanted = $derived(page.url.searchParams.get('item'));
  let extra = $state<ShopItemFieldsFragment | null>(null);
  let giftFirst = $state(false);
  const open = $derived(wanted ? (items.find((i) => i.id === wanted) ?? (extra?.id === wanted ? extra : null)) : null);
  $effect(() => {
    const id = wanted;
    if (!id || items.some((i) => i.id === id)) return;
    gql(SHOP_ITEM, { id })
      .then((r) => {
        if (page.url.searchParams.get('item') === id) extra = r.shopItem ?? null;
      })
      .catch(() => {});
  });

  function show(item: ShopItemFieldsFragment, gift: boolean) {
    giftFirst = gift;
    goto(`/shop?item=${encodeURIComponent(item.id)}`, { noScroll: true, keepFocus: true });
  }
  function closed() {
    if (page.url.searchParams.has('item')) goto('/shop', { noScroll: true, keepFocus: true, replaceState: true });
  }

  const failed = $derived(data.error && !items.length);
</script>

<svelte:head>
  <title>Shop - cyex.tv</title>
  <meta name="description" content="Chat badges and name paints for Twitch and Kick, seen by everyone with the cyex.tv extension." />
  <meta property="og:title" content="Shop" />
</svelte:head>

<ColumnLayout>
  <header class="page-head head">
    <div>
      <h1 class="page-title">Shop</h1>
      <p class="lead">Badges and name paints for your chat name, seen by everyone with the cyex.tv extension on Twitch and Kick. Buy one for yourself or give it to someone.</p>
    </div>
    <a class="btn btn-s" href="/redeem">Redeem a code</a>
  </header>

  <CheckoutStatus />

  {#if failed}
    <Problem cause={data.error === 'offline' ? 'offline' : 'server'} onretry={() => invalidateAll()} />
  {:else if !items.length}
    <State title="Nothing is on sale right now.">
      New items show up here when they’re ready. Got a code?
      {#snippet actions()}<a class="btn btn-s" href="/redeem">Redeem a code</a>{/snippet}
    </State>
  {:else}
    <ul class="grid" aria-label="Items on sale">
      {#each items as item (item.id)}
        <ItemCard {item} {name} onopen={show} />
      {/each}
    </ul>
    <p class="end">Payments are handled by Paddle. What you buy is added to the account that receives it as soon as the payment is confirmed.</p>
  {/if}
</ColumnLayout>

<ItemDialog item={open} gift={giftFirst} {name} signedIn={!!me} onclose={closed} />

<style>
  .head { display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: space-between; gap: 16px 24px; }
  .lead { margin: 12px 0 0; max-width: 62ch; font-size: var(--t-body, 16px); line-height: 1.6; color: var(--muted); }
  .grid { list-style: none; margin: 0; padding: 0; display: grid; gap: 16px; grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 1fr)); }
  .end { margin: clamp(28px, 4vw, 40px) 0 0; max-width: 62ch; font-size: var(--t-small); color: var(--muted); }
</style>
