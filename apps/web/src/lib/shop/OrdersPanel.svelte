<script lang="ts">
  // Your orders: what you bought and what you were given, newest first, with the state, the amount
  // paid and who a gift came from or went to. It lives in Account (one place). The reason staff
  // give for a refund is theirs: it is never shown here.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count, date, iso } from '$lib/format';
  import type { OrderFieldsFragment } from '$lib/gql/graphql';
  import { MY_ORDERS } from '$lib/queries';
  import { fail } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { appendUnique, pageToken } from '$lib/tool/paging';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { orderStatusText, price, type OrderStatus } from './shop';

  const PER = 20;
  let more = $state<OrderFieldsFragment[]>([]);
  let page = $state(1);
  let busy = $state(false);

  const token = pageToken();
  const L = loader(async ({ fetch }) => {
    token.reset();
    more = [];
    page = 1;
    busy = false;
    return (await gql(MY_ORDERS, { page: 1, perPage: PER }, fetch)).myOrders;
  });

  async function next() {
    if (busy) return;
    const live = token.begin();
    busy = true;
    try {
      const r = (await gql(MY_ORDERS, { page: page + 1, perPage: PER })).myOrders;
      if (!live()) return;
      page += 1;
      more = appendUnique(more, r.items);
    } catch (e) {
      if (live()) fail(e);
    } finally {
      if (live()) busy = false;
    }
  }

  const mine = $derived(session.me?.id);
  const sub = (o: OrderFieldsFragment) =>
    [date(o.createdAt), !o.gift ? '' : o.buyer?.id === mine ? `Gift to ${iso(o.recipient?.displayName) || 'someone'}` : `Gift from ${iso(o.buyer?.displayName) || 'someone'}`]
      .filter(Boolean)
      .join(' · ');
  const amount = (o: OrderFieldsFragment) =>
    o.status === 'PAID' || o.status === 'REFUNDED' ? price(o.paidTotalCents ?? o.amountCents, o.paidCurrency ?? o.currency) : price(o.amountCents, o.currency);
</script>

<section class="ui-panel" aria-labelledby="orders-h">
  <header>
    <h2 id="orders-h">Orders</h2>
    <a class="ui-btn ghost small" href="/shop">Shop<Icon name="arrow-right" size={14} /></a>
  </header>
  <LoadGate source={L} label="Loading your orders…">
    {#snippet children(r)}
      {@const rows = appendUnique(r.items, more)}
      {#if !rows.length}
        <State bare title="No orders yet.">What you buy or are given in the shop is listed here.</State>
      {:else}
        <ul class="ui-list" aria-label="Orders">
          {#each rows as o (o.id)}
            <li>
              <span class="what">
                <strong><bdi>{clean(o.itemName)}</bdi></strong>
                <span class="ui-hint">{sub(o)}</span>
              </span>
              <span class="right">
                <span class="amt ui-num">{amount(o)}</span>
                <span class="st" class:dim={o.status !== 'PAID'}>{orderStatusText(o.status as OrderStatus)}</span>
              </span>
            </li>
          {/each}
        </ul>
        {#if rows.length < r.total}
          <div class="foot">
            <button class="ui-btn secondary small" type="button" disabled={busy} onclick={next}>{busy ? 'Loading…' : 'Show more'}</button>
            <span class="ui-hint">{count(rows.length)} of {count(r.total)}</span>
          </div>
        {/if}
      {/if}
    {/snippet}
  </LoadGate>
</section>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .right { display: grid; justify-items: end; text-align: right; flex: none; }
  .amt { font-weight: 600; }
  .st { font-size: var(--t-fine); color: var(--muted); }
  .dim { color: var(--dim); }
  .foot { display: flex; align-items: center; gap: 12px; padding: 10px 14px; border-top: 1px solid var(--border); }
</style>
