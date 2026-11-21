<script lang="ts">
  // /shop, in the Inspector shape (like a profile): the page head, a kind filter and the grid of
  // items on the left; on the right a panel that shows how buying works until an item is open,
  // then that item in full (ItemBuy: on your own name, what it holds, buy it or give it). Each item
  // is drawn on your own name (or a made-up one signed out). Public, signed out too.
  // The address holds the state: ?item=<id> is the open item (to share, and to come back to after
  // signing in), ?kind= the filter. Under 1100px there is no panel: the item opens in a dialog and
  // the "how it works" panel follows the grid. Paying happens in Paddle's window; CheckoutStatus
  // shows the wait and the result above the grid.
  import { tick } from 'svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import FactList from '$lib/components/FactList.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import RetryNote from '$lib/components/RetryNote.svelte';
  import State from '$lib/components/State.svelte';
  import { quiet } from '$lib/quiet';
  import { loadOwned } from '$lib/cosmetics/store.svelte';
  import { clean, count } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import { SHOP_ITEM, SHOP_ITEMS } from '$lib/queries';
  import { flow } from '$lib/shop/checkout.svelte';
  import CheckoutStatus from '$lib/shop/CheckoutStatus.svelte';
  import ItemBuy from '$lib/shop/ItemBuy.svelte';
  import ItemCard from '$lib/shop/ItemCard.svelte';
  import ItemDialog from '$lib/shop/ItemDialog.svelte';
  import { ofKind, SHOP_KINDS, shopKind, type ShopKind } from '$lib/shop/shop';
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

  // The kind filter: only when the shop has more than one kind to tell apart.
  const KIND_LABEL: Record<ShopKind, string> = { all: 'All', paints: 'Paints', badges: 'Badges', bundles: 'Bundles' };
  const kind = $derived(shopKind(page.url.searchParams.get('kind')));
  const kinds = $derived(SHOP_KINDS.map((k) => ({ k, n: items.filter((i) => ofKind(i, k)).length })));
  const filterable = $derived(kinds.filter((x) => x.k !== 'all' && x.n > 0).length > 1);
  const shown = $derived(filterable ? items.filter((i) => ofKind(i, kind)) : items);
  function kindHref(k: ShopKind) {
    const u = new URL(page.url);
    if (k === 'all') u.searchParams.delete('kind');
    else u.searchParams.set('kind', k);
    u.searchParams.delete('item');
    return u.pathname + u.search;
  }

  // The open item: from the address. A hidden or scheduled item (staff) isn't in the list, so it is asked for.
  const wanted = $derived(page.url.searchParams.get('item'));
  let extra = $state<ShopItemFieldsFragment | null>(null);
  let giftFirst = $state(false);
  let extraFailed = $state(false);
  const open = $derived(wanted ? (items.find((i) => i.id === wanted) ?? (extra?.id === wanted ? extra : null)) : null);
  $effect(() => {
    const id = wanted;
    if (!id || items.some((i) => i.id === id)) return;
    loadExtra(id);
  });
  function loadExtra(id: string) {
    extraFailed = false;
    void quiet(
      gql(SHOP_ITEM, { id }).then((r) => {
        if (page.url.searchParams.get('item') === id) extra = r.shopItem ?? null;
      }),
      'shop item',
      () => (extraFailed = true),
    );
  }

  // Wide windows show the item in the right panel; narrow ones in a dialog.
  let wide = $state(false);
  $effect(() => {
    const mq = matchMedia('(min-width: 1100px)');
    wide = mq.matches;
    const on = () => (wide = mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });

  let buyPanel = $state<ItemBuy>();
  let panel = $state<HTMLElement>();
  // Where focus goes back to when the panel closes: the card the item was opened from.
  let from: string | null = null;

  function itemHref(id: string | null) {
    const u = new URL(page.url);
    if (id) u.searchParams.set('item', id);
    else u.searchParams.delete('item');
    return u.pathname + u.search;
  }

  async function show(item: ShopItemFieldsFragment, gift: boolean) {
    giftFirst = gift;
    from = item.id;
    await goto(itemHref(item.id), { noScroll: true, keepFocus: true });
    if (!wide) return;
    await tick();
    panel?.scrollTo({ top: 0 });
    buyPanel?.focusStart();
  }
  function closed() {
    if (page.url.searchParams.has('item')) goto(itemHref(null), { noScroll: true, keepFocus: true, replaceState: true });
  }
  async function closePanel() {
    const back = from ?? wanted;
    closed();
    await tick();
    if (back) document.querySelector<HTMLElement>(`a[data-item="${CSS.escape(back)}"]`)?.focus();
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !wide || !open || e.defaultPrevented) return;
    // Escape inside a field of the panel or anywhere on the page closes the panel (no menu is open here).
    e.preventDefault();
    closePanel();
  }

  const failed = $derived(data.error && !items.length);
  const panelItem = $derived(wide ? open : null);
</script>

<svelte:window {onkeydown} />

<svelte:head>
  <title>{open ? `${clean(open.name)} · Shop` : 'Shop'} - cyex.tv</title>
  <meta name="description" content="Chat badges and name paints for Twitch and Kick, seen by everyone with the cyex.tv extension." />
  <meta property="og:title" content="Shop" />
</svelte:head>

{#snippet about()}
  <PanelBlock title="How it works">
    <FactList
      items={[
        { label: 'Shown', value: 'In Twitch and Kick chat, to everyone with the extension' },
        { label: 'Paid with', value: 'Paddle' },
        { label: 'Arrives', value: 'As soon as the payment is confirmed' },
        { label: 'Gifts', value: 'To any Twitch or Kick login that has signed in once' },
      ]}
    />
  </PanelBlock>
  <PanelBlock title="More">
    <LinkRows
      items={[
        { href: '/redeem', label: 'Redeem a code', icon: 'ticket' },
        me
          ? { href: '/me/settings/billing', label: 'Your orders', icon: 'list' }
          : { href: `/signin?back=${encodeURIComponent('/shop')}`, label: 'Sign in to see items on your name', icon: 'enter' },
        { href: '/help#what-are-cosmetics', label: 'What badges and paints are', icon: 'scroll' },
      ]}
    />
  </PanelBlock>
{/snippet}

<div class="wrap shop">
  <div class="main">
    <header class="page-head head">
      <div>
        <h1 class="page-title">Shop</h1>
        <p class="line">Badges and name paints for your chat name on Twitch and Kick. Buy one for yourself or give it to someone.</p>
      </div>
      <a class="btn btn-s" href="/redeem"><Icon name="ticket" size={15} />Redeem a code</a>
    </header>

    <CheckoutStatus />

    {#if extraFailed && wanted}
      <RetryNote onretry={() => loadExtra(wanted)}>Couldn’t open the item from that link.</RetryNote>
    {/if}

    {#if failed}
      <Problem cause={data.error === 'offline' ? 'offline' : 'server'} onretry={() => invalidateAll()} />
    {:else if !items.length}
      <State title="Nothing is on sale right now.">
        New items show up here when they’re ready. Got a code?
        {#snippet actions()}<a class="btn btn-s" href="/redeem">Redeem a code</a>{/snippet}
      </State>
    {:else}
      {#if filterable}
        <nav class="kinds" aria-label="Kinds">
          {#each kinds as x (x.k)}
            {#if x.k === 'all' || x.n}
              <a href={kindHref(x.k)} aria-current={kind === x.k ? 'page' : undefined} data-sveltekit-noscroll data-sveltekit-keepfocus>
                <span class="tl" data-t={KIND_LABEL[x.k]}>{KIND_LABEL[x.k]}</span><span class="n">{count(x.n)}</span>
              </a>
            {/if}
          {/each}
        </nav>
      {/if}
      {#if shown.length}
        <ul class="grid" aria-label="Items on sale">
          {#each shown as item (item.id)}
            <ItemCard {item} {name} current={!!panelItem && panelItem.id === item.id} onopen={show} />
          {/each}
        </ul>
      {:else}
        <State title="No {KIND_LABEL[kind].toLowerCase()} on sale right now.">
          {#snippet actions()}<a class="btn btn-s" href={kindHref('all')}>Show everything</a>{/snippet}
        </State>
      {/if}
    {/if}

    <div class="about-narrow">{@render about()}</div>
  </div>

  <aside class="panel" bind:this={panel} aria-label={panelItem ? clean(panelItem.name) : 'About the shop'} data-testid="shop-panel">
    {#if panelItem}
      {#key `${panelItem.id}:${giftFirst}`}
        <ItemBuy bind:this={buyPanel} item={panelItem} gift={giftFirst} {name} signedIn={!!me} variant="panel" headingId="shop-panel-title"
          onclose={closePanel} onstarted={closed} showing={() => !!wide && page.url.searchParams.get('item') === panelItem?.id} />
      {/key}
    {:else}
      <div class="ip-head"><h2 class="label">About the shop</h2></div>
      <div class="ip-body">
        {@render about()}
        <p class="pick">Pick an item to see it on your name at every size.</p>
      </div>
    {/if}
  </aside>
</div>

<ItemDialog item={wide ? null : open} gift={giftFirst} {name} signedIn={!!me} onclose={closed} />

<style>
  p { margin: 0; }
  /* The Inspector shape: the shop on the left, the panel on the right (360, full height, sticky). */
  .shop { display: grid; grid-template-columns: minmax(0, 1fr) 360px; column-gap: 24px; align-items: start; }
  .main { min-width: 0; padding-bottom: 48px; }
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .head > div { min-width: 0; }
  .head .btn { flex: none; }
  @media (max-width: 560px) { .head { flex-direction: column; gap: 12px; } }
  .line { margin-top: 6px; max-width: 62ch; font-size: var(--t-ui); line-height: 1.55; color: var(--muted); }
  .head .btn :global(svg) { color: var(--muted); }

  /* The kinds: words on a hairline; the current one bold (no bar or fill), like the profile's tabs. */
  .kinds { display: flex; gap: 4px; overflow-x: auto; scrollbar-width: none; margin: 0 0 16px; border-bottom: 1px solid var(--border); }
  .kinds a:first-child { padding-left: 0; }
  .kinds a { flex: none; display: inline-flex; align-items: center; gap: 6px; height: 44px; padding: 0 10px; color: var(--muted);
    font-size: var(--t-ui); font-weight: 500; white-space: nowrap; transition: color var(--fast); }
  .kinds a[aria-current='page'] { color: var(--text); font-weight: 700; }
  .kinds a:focus-visible { outline-offset: -2px; }
  .tl { display: inline-grid; }
  .tl::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; }
  .n { font-size: var(--t-small); color: var(--dim); font-weight: 500; font-variant-numeric: tabular-nums; }

  /* The grid: cells sharing hairlines in one frame. */
  .grid { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 1fr));
    border: 1px solid var(--border); border-radius: var(--radius-card); overflow: hidden; }

  .panel { position: sticky; top: var(--nav-h); height: calc(100svh - var(--nav-h)); overflow-y: auto; overscroll-behavior: contain;
    margin-right: calc(-1 * var(--page-pad)); border-left: 1px solid var(--border); background: var(--surface);
    box-shadow: 0 0 0 100vw var(--surface); clip-path: inset(0 -100vw 0 0); /* on to the window's edge past the wrap, like every right panel */ }
  .ip-head { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; height: 48px; padding: 0 16px;
    border-bottom: 1px solid var(--border); background: var(--surface); }
  .ip-body { display: grid; gap: 4px; padding: 8px 16px 16px; }
  .label { margin: 0; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .pick { padding-top: 12px; border-top: 1px solid var(--border); font-size: var(--t-small); color: var(--muted); }
  .about-narrow { display: none; }

  @media (hover: hover) and (pointer: fine) {
    .kinds a:hover { color: var(--text); }
  }

  /* Narrower: one column; no panel (items open in a dialog), "how it works" follows the grid. */
  @media (max-width: 1099.98px) {
    .shop { grid-template-columns: minmax(0, 1fr); }
    .panel { display: none; }
    .about-narrow { display: grid; gap: 4px; margin-top: 24px; padding: 8px 16px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  }
</style>
