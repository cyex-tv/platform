<script lang="ts">
  // Admin: every order, newest first (shop.orders), in a dense table: when, buyer, item, amount,
  // state, gift. Narrowed by state, item or account; the filters, the page and the picked order
  // (?status= ?item= ?account= ?page= ?order=) live in the address. The picked order opens in the
  // inspector: who paid, who got it, the amounts, Paddle's transaction, the dates and the refund
  // reason staff gave. A paid order can be refunded with a reason: Paddle is asked for a full
  // refund and the cosmetics are taken back at once. Buyers never see that reason.
  import { goto } from '$app/navigation';
  import { page as route } from '$app/state';
  import { tick } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import { lookUp, who } from '$lib/admin/names.svelte';
  import { orderFilters, ordersHref, pageRange } from '$lib/admin/shop-orders';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys } from '$lib/admin/ui/pick';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, clock, count, date, dateTime, iso } from '$lib/format';
  import type { OrderStatus as Status } from '$lib/gql/graphql';
  import { ADMIN_ORDERS, REFUND_ORDER, SHOP_ITEMS } from '$lib/queries';
  import Gate from '$lib/shop/Gate.svelte';
  import { loadShopCounts } from '$lib/shop/counts.svelte';
  import { orderStatusText, price, staffMessage, type OrderStatus } from '$lib/shop/shop';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { notify } from '$lib/tool/notices.svelte';

  const PER = 25;
  type Row = NonNullable<Awaited<ReturnType<typeof ask>>['items']>[number];
  const ask = async (status: Status | null, itemId: string, accountId: string, page: number, f?: typeof fetch) =>
    (await gql(ADMIN_ORDERS, { status, itemId: itemId || null, accountId: accountId || null, page, perPage: PER }, f)).orders;

  // Filters, page and picked order come from the address; each is its own derived value so
  // picking an order does not load the list again.
  const status = $derived(orderFilters(route.url.searchParams).status);
  const itemId = $derived(orderFilters(route.url.searchParams).item);
  const accountId = $derived(orderFilters(route.url.searchParams).account);
  const pageNo = $derived(orderFilters(route.url.searchParams).page);
  const pickedId = $derived(route.url.searchParams.get('order'));
  function go(patch: Parameters<typeof ordersHref>[1]) {
    void goto(ordersHref(route.url, patch), { replaceState: true, keepFocus: true, noScroll: true });
  }

  // The account filter's name: from a row the staff member clicked, or looked up (a staff account page links here).
  let known = $state<Record<string, string>>({});
  $effect(() => {
    if (!accountId || known[accountId]) return;
    lookUp([accountId]);
  });
  const accountName = $derived(accountId ? known[accountId] ?? who(accountId)?.name ?? '' : '');

  let target = $state<Row | null>(null);
  let refund: ReasonDialog;

  const items = loader(async ({ fetch }) => (await gql(SHOP_ITEMS, { all: true }, fetch)).shopItems);
  const L = loader(async ({ fetch }) => ask(status || null, itemId, accountId, pageNo, fetch));

  const picked = $derived(L.data?.items.find((o) => o.id === pickedId) ?? null);

  async function doRefund(reason: string) {
    if (!target) return;
    const row = target;
    try {
      const r = await gql(REFUND_ORDER, { id: row.id, reason });
      notify(`${iso(row.itemName)} refunded. The cosmetics were taken back.`);
      L.set(L.data ? { ...L.data, items: L.data.items.map((o) => (o.id === r.refundOrder.id ? r.refundOrder : o)) } : null);
      loadShopCounts();
    } catch (e) {
      // The dialog says why. When the order changed under us (refunded already, a refund running), show its real state.
      if (e instanceof ApiError && (e.code === 'already_refunded' || e.code === 'refund_in_progress')) void L.reload();
      throw e;
    }
  }

  const amount = (o: Row) =>
    o.status === 'PAID' || o.status === 'REFUNDED' ? price(o.paidTotalCents ?? o.amountCents, o.paidCurrency ?? o.currency) : price(o.amountCents, o.currency);
  const filtered = $derived(!!status || !!itemId || !!accountId);
  const STATES: { value: Status | ''; label: string }[] = [
    { value: '', label: 'All' },
    { value: 'PAID', label: 'Paid' },
    { value: 'PENDING', label: 'Waiting' },
    { value: 'REFUNDED', label: 'Refunded' },
    { value: 'FAILED', label: 'Not completed' },
  ];
  const gone = 'Deleted account';

  let body = $state<HTMLElement>();
  function pick(id: string | null) {
    go({ order: id });
  }
  // Closing the inspector puts focus back on the order's row.
  function unpick() {
    const id = pickedId;
    pick(null);
    void tick().then(() => body?.querySelector<HTMLElement>(`[data-item="${CSS.escape(id ?? '')}"]`)?.focus());
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !pickedId || e.defaultPrevented || document.querySelector('dialog[open]')) return;
    if ((e.target as HTMLElement | null)?.closest('input, textarea, select')) return;
    e.preventDefault();
    unpick();
  }
  function rowKey(e: KeyboardEvent, id: string) {
    if ((e.key === 'Enter' || e.key === ' ') && e.target === e.currentTarget) {
      e.preventDefault();
      pick(id);
    }
  }
  function byAccount(a: { id: string; displayName: string }) {
    known[a.id] = a.displayName;
    go({ account: a.id });
  }
</script>

<svelte:head><title>{picked ? `${clean(picked.itemName)} - Orders` : 'Orders'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Gate permission="shop.orders">
  <Split label="Order" open={!!picked} onclose={() => pick(null)}>
    <Bar title="Orders" count={L.data ? `${count(L.data.total)} ${L.data.total === 1 ? 'order' : 'orders'}` : null}>
      {#snippet tools()}
        <Segmented label="State" size="s" options={STATES} value={status} onchange={(v) => go({ status: v })} />
        <label class="tb">
          <span class="sr-only">Item</span>
          <select class="ui-input" value={itemId} onchange={(e) => go({ item: e.currentTarget.value })}>
            <option value="">Any item</option>
            {#each items.data ?? [] as i (i.id)}<option value={i.id}>{clean(i.name)}</option>{/each}
          </select>
        </label>
        {#if accountId}
          <button class="ui-btn secondary small clear" type="button" onclick={() => go({ account: null })} aria-label="Stop filtering by {accountName ? iso(accountName) : accountId}">
            <Icon name="user" size={14} /><bdi>{clean(accountName) || accountId.slice(-8)}</bdi><Icon name="x" size={14} />
          </button>
        {/if}
      {/snippet}
    </Bar>

    <LoadGate source={L} label="Loading orders…">
      {#snippet children(r)}
        {#if !r.items.length}
          <State title={filtered ? 'No orders match.' : 'No orders yet.'}>
            {filtered ? 'Try another state, item or account.' : 'Orders show up here when someone starts a checkout.'}
            {#snippet actions()}{#if pageNo > 1}<button class="ui-btn secondary" type="button" onclick={() => go({ page: 1 })}>First page</button>{/if}{/snippet}
          </State>
        {:else}
          {@const rg = pageRange(pageNo, PER, r.total, r.items.length)}
          <div class="tablebox">
          <table class="adm-table stack">
            <thead>
              <tr><th class="when">When</th><th class="buyer">Buyer</th><th>Item</th><th class="num" style="width:76px">Amount</th><th style="width:88px">State</th><th class="sec" style="width:96px">Gift</th></tr>
            </thead>
            <tbody bind:this={body} use:gridKeys={'tr[data-item]'}>
              {#each r.items as o, n (o.id)}
                <tr data-row data-item={o.id} aria-selected={o.id === pickedId} tabindex={(picked ? o.id === pickedId : n === 0) ? 0 : -1}
                  onclick={() => pick(o.id)} onkeydown={(e) => rowKey(e, o.id)}>
                  <td class="dim mono" data-l="When">{date(o.createdAt)}<span class="tm"> {clock(o.createdAt)}</span></td>
                  <td class="ell" data-l="Buyer"><bdi>{o.buyer ? clean(o.buyer.displayName) : gone}</bdi></td>
                  <td class="ell pk" data-l="Item"><bdi>{clean(o.itemName)}</bdi></td>
                  <td class="num" data-l="Amount">{amount(o)}</td>
                  <td class="ell" data-l="State"><span class="adm-flags"><span class:ok={o.status === 'PAID'} class:warn={o.status === 'FAILED'}>{orderStatusText(o.status as OrderStatus)}</span></span></td>
                  <td class="ell dim sec" data-l="Gift">{#if o.gift}<bdi>{o.recipient ? clean(o.recipient.displayName) : gone}</bdi>{:else}-{/if}</td>
                </tr>
              {/each}
            </tbody>
          </table>
          </div>
          {#if rg.pages > 1}
            <nav class="adm-pager" aria-label="Pages">
              <button class="ui-btn secondary small" type="button" disabled={pageNo <= 1} onclick={() => go({ page: pageNo - 1 })}><Icon name="chevron-left" size={14} />Previous</button>
              <span>{count(rg.from)}-{count(rg.to)} of {count(r.total)}</span>
              <button class="ui-btn secondary small" type="button" disabled={pageNo >= rg.pages} onclick={() => go({ page: pageNo + 1 })}>Next<Icon name="chevron-right" size={14} /></button>
            </nav>
          {/if}
        {/if}
      {/snippet}
    </LoadGate>

    {#snippet empty()}
      <div class="adm-ip-head"><span class="adm-label">Order</span></div>
      <p class="none">Pick an order to see who paid, who got it, and refund it.</p>
    {/snippet}

    {#snippet panel()}
      {#if picked}
        {@const o = picked}
        <div class="adm-ip-head">
          <span class="adm-label">Order</span>
          <IconButton label="Close" icon="x" size="s" onclick={unpick} />
        </div>
        <div class="adm-ip-body">
          <section class="adm-sec">
            <h2 class="adm-name"><bdi>{clean(o.itemName)}</bdi></h2>
            <p class="adm-flags"><span class:ok={o.status === 'PAID'} class:warn={o.status === 'FAILED'}>{orderStatusText(o.status as OrderStatus)}</span><span class="adm-num">{amount(o)}</span></p>
            {#if o.refundRejectedAt}
              <p class="flag" role="note"><Icon name="flag" size={14} />Paddle rejected this refund on {dateTime(o.refundRejectedAt)}; the cosmetics stay revoked. Look at it in Paddle.</p>
            {/if}
          </section>
          <section class="adm-sec">
            <h3 class="adm-label">People</h3>
            <dl class="adm-kv">
              <dt>Buyer</dt>
              <dd class="txt">{#if o.buyer}<button class="link" type="button" title="Show this account's orders" onclick={() => byAccount(o.buyer!)}><bdi>{clean(o.buyer.displayName)}</bdi></button>{:else}{gone}{/if}</dd>
              <dt>Got it</dt>
              <dd class="txt">{#if o.recipient}<button class="link" type="button" title="Show this account's orders" onclick={() => byAccount(o.recipient!)}><bdi>{clean(o.recipient.displayName)}</bdi></button>{:else}{gone}{/if}{#if o.gift}{" "}(gift){/if}</dd>
            </dl>
          </section>
          <section class="adm-sec">
            <h3 class="adm-label">Payment</h3>
            <dl class="adm-kv">
              <dt>Price when started</dt><dd>{price(o.amountCents, o.currency)}</dd>
              {#if o.paidTotalCents != null}<dt>Paid, tax included</dt><dd>{price(o.paidTotalCents, o.paidCurrency ?? o.currency)}</dd>{/if}
              {#if o.providerTransactionId}<dt>Paddle</dt><dd class="mono">{o.providerTransactionId}</dd>{/if}
              <dt>Order</dt><dd class="mono">{o.id}</dd>
            </dl>
          </section>
          <section class="adm-sec">
            <h3 class="adm-label">Dates</h3>
            <dl class="adm-kv">
              <dt>Started</dt><dd>{dateTime(o.createdAt)}</dd>
              {#if o.paidAt}<dt>Paid</dt><dd>{dateTime(o.paidAt)}</dd>{/if}
              {#if o.refundedAt}<dt>Refunded</dt><dd>{dateTime(o.refundedAt)}</dd>{/if}
              {#if o.failedAt}<dt>Not completed</dt><dd>{dateTime(o.failedAt)}</dd>{/if}
            </dl>
          </section>
          {#if o.refundReason}
            <section class="adm-sec">
              <h3 class="adm-label">Refund reason <span class="n">Staff only</span></h3>
              <p class="reason"><bdi>{clean(o.refundReason)}</bdi></p>
            </section>
          {/if}
          {#if o.status === 'PAID'}
            <section class="adm-sec">
              <div class="adm-acts">
                <button class="ui-btn danger small" type="button" onclick={() => { target = o; refund.open(); }}>Refund…</button>
              </div>
              <p class="ui-hint">Refunds the full amount in Paddle and takes the cosmetics back.</p>
            </section>
          {/if}
        </div>
      {/if}
    {/snippet}
  </Split>

  <ReasonDialog bind:this={refund} title={target ? `Refund ${iso(target.itemName)}` : 'Refund'} hint="Staff only. The buyer is not shown it. 1 to 500 characters." action="Refund" danger max={500} describe={staffMessage} onsubmit={doRefund} />
</Gate>

<style>
  .stack { table-layout: fixed; }
  /* Names first: the buyer gets a share of the row, not a sliver. Where the list is narrow (the
     inspector open beside it, 1280px windows) the gift column and the time go; both are in the
     inspector (Got it … (gift), Dates). */
  .tablebox { container-type: inline-size; }
  .when { width: 136px; }
  .buyer { width: 24%; }
  @media (min-width: 720px) {
    @container (max-width: 759.98px) {
      .sec, .tm { display: none; }
      .when { width: 100px; }
    }
  }
  tr[aria-selected='true'] .pk { font-weight: 700; }
  .tb { display: block; min-width: 0; }
  .tb select { width: auto; min-width: 150px; max-width: 100%; }
  .clear { max-width: 100%; }
  .clear bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .flag { display: flex; align-items: flex-start; gap: 8px; margin: 0; font-size: var(--t-small); font-weight: 600; }
  .flag :global(svg) { flex: none; margin-top: 2px; color: var(--red); }
  .reason { margin: 0; font-size: var(--t-small); overflow-wrap: anywhere; }
  .link { padding: 0; border: 0; background: none; font: inherit; font-weight: 600; color: var(--accent-text); cursor: pointer; text-align: right; }
  .link:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; border-radius: 2px; }
  /* Phones: each row stacks, its cells labelled. */
  @media (max-width: 719.98px) {
    .tb, .tb select { width: 100%; }
  }
</style>
