<script lang="ts">
  // Admin: every order, newest first (shop.orders), narrowed by state, item or account. A row opens
  // to its detail: who paid, who got it, the amounts, Paddle's transaction, the dates and the refund
  // reason staff gave. A paid order can be refunded with a reason: Paddle is asked for a full
  // refund and the cosmetics are taken back at once. Buyers never see that reason.
  import { page as route } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import { lookUp, who } from '$lib/admin/names.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count, dateTime, iso } from '$lib/format';
  import type { OrderStatus as Status } from '$lib/gql/graphql';
  import { ADMIN_ORDERS, REFUND_ORDER, SHOP_ITEMS } from '$lib/queries';
  import Gate from '$lib/shop/Gate.svelte';
  import { loadShopCounts } from '$lib/shop/counts.svelte';
  import { orderStatusText, price, staffMessage, type OrderStatus } from '$lib/shop/shop';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { appendUnique, pageToken } from '$lib/tool/paging';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const PER = 25;
  type Row = NonNullable<Awaited<ReturnType<typeof ask>>['items']>[number];
  const ask = async (status: Status | null, itemId: string, accountId: string, page: number, f?: typeof fetch) =>
    (await gql(ADMIN_ORDERS, { status, itemId: itemId || null, accountId: accountId || null, page, perPage: PER }, f)).orders;

  let status = $state<Status | ''>('');
  let itemId = $state('');
  // ?account=<id> (from a staff account page) starts narrowed to that account.
  const fromLink = route.url.searchParams.get('account');
  let account = $state<{ id: string; name: string } | null>(fromLink ? { id: fromLink, name: '' } : null);
  $effect(() => {
    if (!account || account.name) return;
    const id = account.id;
    lookUp([id]);
    const known = who(id);
    if (known) account = { id, name: known.name };
  });
  let more = $state<Row[]>([]);
  let page = $state(1);
  let loadingMore = $state(false);
  let open = $state<string | null>(null);
  let target = $state<Row | null>(null);
  let refund: ReasonDialog;

  const token = pageToken();
  const items = loader(async ({ fetch }) => (await gql(SHOP_ITEMS, { all: true }, fetch)).shopItems);
  const L = loader(async ({ fetch }) => {
    const s = status || null;
    const i = itemId;
    const a = account?.id ?? '';
    token.reset();
    more = [];
    page = 1;
    loadingMore = false;
    return ask(s, i, a, 1, fetch);
  });

  async function next() {
    if (loadingMore) return;
    const live = token.begin();
    loadingMore = true;
    try {
      const r = await ask(status || null, itemId, account?.id ?? '', page + 1);
      if (!live()) return; // the filter changed while this loaded
      page += 1;
      more = appendUnique(more, r.items);
    } catch (e) {
      if (live()) fail(e);
    } finally {
      if (live()) loadingMore = false;
    }
  }

  async function doRefund(reason: string) {
    if (!target) return;
    const row = target;
    try {
      const r = await gql(REFUND_ORDER, { id: row.id, reason });
      notify(`${iso(row.itemName)} refunded. The cosmetics were taken back.`);
      replace(r.refundOrder);
      loadShopCounts();
    } catch (e) {
      // The dialog says why. When the order changed under us (refunded already, a refund running), show its real state.
      if (e instanceof ApiError && (e.code === 'already_refunded' || e.code === 'refund_in_progress')) void L.reload();
      throw e;
    }
  }
  function replace(next: Row) {
    L.set(L.data ? { ...L.data, items: L.data.items.map((o) => (o.id === next.id ? next : o)) } : null);
    more = more.map((o) => (o.id === next.id ? next : o));
  }

  const amount = (o: Row) =>
    o.status === 'PAID' || o.status === 'REFUNDED' ? price(o.paidTotalCents ?? o.amountCents, o.paidCurrency ?? o.currency) : price(o.amountCents, o.currency);
  const filtered = $derived(!!status || !!itemId || !!account);
  const STATES: { value: Status | ''; label: string }[] = [
    { value: '', label: 'Any state' },
    { value: 'PAID', label: 'Paid' },
    { value: 'PENDING', label: 'Waiting for payment' },
    { value: 'REFUNDED', label: 'Refunded' },
    { value: 'FAILED', label: 'Not completed' },
  ];
</script>

<svelte:head><title>Orders - Admin - cyex.tv</title></svelte:head>

<Gate permission="shop.orders">
  <PageHead title="Orders" line="Every purchase and gift, newest first. A paid order can be refunded here." />

  <div class="filters">
    <label class="ui-field">
      <span class="ui-label">State</span>
      <select class="ui-input" bind:value={status}>
        {#each STATES as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
      </select>
    </label>
    <label class="ui-field">
      <span class="ui-label">Item</span>
      <select class="ui-input" bind:value={itemId}>
        <option value="">Any item</option>
        {#each items.data ?? [] as i (i.id)}<option value={i.id}>{clean(i.name)}</option>{/each}
      </select>
    </label>
    {#if account}
      <div class="ui-field">
        <span class="ui-label">Account</span>
        <button class="ui-btn secondary clear" type="button" onclick={() => (account = null)} aria-label="Stop filtering by {iso(account.name)}"><bdi>{clean(account.name) || account.id.slice(-8)}</bdi><Icon name="x" size={14} /></button>
      </div>
    {/if}
  </div>

  <LoadGate source={L} label="Loading orders…">
    {#snippet children(r)}
      {@const rows = appendUnique(r.items, more)}
      {#if !rows.length}
        <State title={filtered ? 'No orders match.' : 'No orders yet.'}>
          {filtered ? 'Try another state, item or account.' : 'Orders show up here when someone starts a checkout.'}
        </State>
      {:else}
        <p class="ui-hint total" aria-live="polite">{count(r.total)} {r.total === 1 ? 'order' : 'orders'}</p>
        <ul class="list">
          {#each rows as o (o.id)}
            <li class="item">
              <button class="row" type="button" aria-expanded={open === o.id} onclick={() => (open = open === o.id ? null : o.id)}>
                <span class="what">
                  <strong><bdi>{clean(o.itemName)}</bdi></strong>
                  <span class="ui-hint">{dateTime(o.createdAt)} · {iso(o.buyer?.displayName) || 'Deleted account'}{o.gift ? ` → ${iso(o.recipient?.displayName) || 'Deleted account'}` : ''}</span>
                </span>
                <span class="st" class:live={o.status === 'PAID'}>{orderStatusText(o.status as OrderStatus)}</span>
                <span class="amt ui-num">{amount(o)}</span>
                <Icon name={open === o.id ? 'chevron-down' : 'chevron-right'} size={16} />
              </button>
              {#if open === o.id}
                <div class="detail">
                  <dl>
                    <div><dt>Buyer</dt><dd>
                      {#if o.buyer}<button class="link" type="button" onclick={() => (account = { id: o.buyer!.id, name: o.buyer!.displayName })}><bdi>{clean(o.buyer.displayName)}</bdi></button>{:else}Deleted account{/if}
                    </dd></div>
                    <div><dt>Got it</dt><dd>
                      {#if o.recipient}<button class="link" type="button" onclick={() => (account = { id: o.recipient!.id, name: o.recipient!.displayName })}><bdi>{clean(o.recipient.displayName)}</bdi></button>{:else}Deleted account{/if}{#if o.gift} (a gift){/if}
                    </dd></div>
                    <div><dt>Price when started</dt><dd class="ui-num">{price(o.amountCents, o.currency)}</dd></div>
                    {#if o.paidTotalCents != null}<div><dt>Paid, tax included</dt><dd class="ui-num">{price(o.paidTotalCents, o.paidCurrency ?? o.currency)}</dd></div>{/if}
                    {#if o.providerTransactionId}<div><dt>Paddle transaction</dt><dd class="ui-mono">{o.providerTransactionId}</dd></div>{/if}
                    <div><dt>Order</dt><dd class="ui-mono">{o.id}</dd></div>
                    <div><dt>Started</dt><dd class="ui-num">{dateTime(o.createdAt)}</dd></div>
                    {#if o.paidAt}<div><dt>Paid</dt><dd class="ui-num">{dateTime(o.paidAt)}</dd></div>{/if}
                    {#if o.refundedAt}<div><dt>Refunded</dt><dd class="ui-num">{dateTime(o.refundedAt)}</dd></div>{/if}
                    {#if o.failedAt}<div><dt>Not completed</dt><dd class="ui-num">{dateTime(o.failedAt)}</dd></div>{/if}
                    {#if o.refundRejectedAt}<div class="wide"><dt>Refund rejected</dt><dd class="flag" role="note"><Icon name="flag" size={14} />Paddle rejected this refund on {dateTime(o.refundRejectedAt)}; the cosmetics stay revoked. Look at it in Paddle.</dd></div>{/if}
                    {#if o.refundReason}<div class="wide"><dt>Refund reason (staff only)</dt><dd><bdi>{clean(o.refundReason)}</bdi></dd></div>{/if}
                  </dl>
                  {#if o.status === 'PAID'}
                    <div class="ui-actions">
                      <button class="ui-btn danger small" type="button" onclick={() => { target = o; refund.open(); }}>Refund…</button>
                      <span class="ui-hint">Refunds the full amount in Paddle and takes the cosmetics back.</span>
                    </div>
                  {/if}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
        {#if rows.length < r.total}
          <div class="foot">
            <button class="ui-btn secondary" type="button" disabled={loadingMore} onclick={next}>{loadingMore ? 'Loading…' : 'Show more'}</button>
            <span class="ui-hint">{count(rows.length)} of {count(r.total)}</span>
          </div>
        {/if}
      {/if}
    {/snippet}
  </LoadGate>

  <ReasonDialog bind:this={refund} title={target ? `Refund ${iso(target.itemName)}` : 'Refund'} hint="Staff only. The buyer is not shown it. 1 to 500 characters." action="Refund" danger max={500} describe={staffMessage} onsubmit={doRefund} />
</Gate>

<style>
  .filters { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 12px 16px; margin-bottom: 16px; }
  .filters .ui-field { min-width: min(200px, 100%); }
  .clear { justify-content: space-between; }
  .total { margin: 0 0 8px; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .item { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .row { display: flex; align-items: center; gap: 10px 16px; width: 100%; min-height: 56px; padding: 8px 14px; border: 0; background: none; font: inherit; color: inherit; text-align: left; cursor: pointer; border-radius: var(--radius-card); }
  .row > :global(svg) { flex: none; color: var(--dim); }
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st { flex: none; min-width: 120px; font-size: var(--t-small); color: var(--muted); }
  .st.live { color: var(--text); font-weight: 700; }
  .amt { flex: none; min-width: 72px; text-align: right; font-weight: 600; }
  .detail { display: grid; gap: 12px; padding: 4px 14px 14px; border-top: 1px solid var(--border); }
  dl { margin: 12px 0 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(220px, 100%), 1fr)); gap: 10px 16px; }
  dl > div { min-width: 0; }
  dl .wide { grid-column: 1 / -1; }
  dt { font-size: var(--t-fine); color: var(--dim); }
  dd { margin: 2px 0 0; font-size: var(--t-small); overflow-wrap: anywhere; }
  .flag { display: flex; align-items: flex-start; gap: 8px; font-weight: 600; color: var(--text); }
  .flag :global(svg) { flex: none; margin-top: 2px; color: var(--red); }
  .link { padding: 0; border: 0; background: none; font: inherit; color: var(--accent-text); cursor: pointer; text-align: left; }
  .foot { display: flex; align-items: center; gap: 12px; margin-top: 12px; }
  @media (max-width: 640px) { .st { min-width: 0; } .row { flex-wrap: wrap; } .what { flex: 1 1 100%; } .amt { margin-left: auto; } }
</style>
