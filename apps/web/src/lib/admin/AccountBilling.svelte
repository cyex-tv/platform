<script lang="ts">
  // Billing on the staff account page: the account's latest orders (bought or received, from the
  // shop) and its subscriptions, past, current and to come. There are no subscriptions yet; the
  // panel says so until the API returns some.
  import { gql } from '$lib/api';
  import type { AccountSubscriptionsQuery, AdminOrdersQuery } from '$lib/gql/graphql';
  import { ACCOUNT_SUBSCRIPTIONS, ADMIN_ORDERS } from '$lib/queries';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { count, date, dateTime, iso } from '$lib/format';
  import Box from './ui/Box.svelte';
  import { orderRow, orderStatusText, price } from '$lib/shop/shop';

  type Orders = AdminOrdersQuery['orders'];
  type Sub = AccountSubscriptionsQuery['accountSubscriptions'][number];
  let { accountId }: { accountId: string } = $props();
  const PER = 8;

  let orders = $state<Orders | null>(null);
  let subs = $state<Sub[] | null>(null);
  let problem = $state(false);
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = false;
    try {
      const [o, s] = await Promise.all([
        gql(ADMIN_ORDERS, { accountId, page: 1, perPage: PER }),
        gql(ACCOUNT_SUBSCRIPTIONS, { accountId }),
      ]);
      if (mine !== seq) return;
      orders = o.orders;
      subs = s.accountSubscriptions;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void accountId;
    orders = null;
    subs = null;
    load();
  });

  const STATE: Record<string, string> = { PAST: 'Ended', CURRENT: 'Current', FUTURE: 'Starts later' };
  const spent = $derived(
    (orders?.items ?? []).filter((o) => o.status === 'PAID' && o.buyer?.id === accountId).reduce((n, o) => n + (o.paidTotalCents ?? o.amountCents), 0),
  );
  const currency = $derived(orders?.items.find((o) => o.status === 'PAID')?.paidCurrency ?? orders?.items[0]?.currency ?? 'EUR');
</script>

<Box title="Subscriptions and orders">
  {#snippet actions()}
    <a class="ui-btn ghost small" href="/admin/shop/orders?account={encodeURIComponent(accountId)}">All orders<Icon name="arrow-right" size={14} /></a>
  {/snippet}
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !orders || !subs}
    <State bare kind="loading" title="Loading billing…" />
  {:else}
    <h3 class="box-sub">Subscriptions</h3>
    {#if subs.length === 0}
      <p class="box-none">No subscriptions. cyex.tv doesn’t sell any yet.</p>
    {:else}
      <table class="adm-table stack">
        <thead><tr><th>Plan</th><th>From</th><th>To</th><th>Source</th><th>State</th></tr></thead>
        <tbody>
          {#each subs as s (s.id)}
            <tr>
              <td class="pk"><strong>{s.plan}</strong></td>
              <td data-l="From" class="mono">{date(s.startsAt)}</td>
              <td data-l="To" class="mono">{s.endsAt ? date(s.endsAt) : 'Renews'}</td>
              <td data-l="Source">{s.source}</td>
              <td data-l="State"><span class="adm-flags"><span class:ok={s.state === 'CURRENT'}>{STATE[s.state]}</span></span></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    <h3 class="box-sub">
      Orders
      {#if orders.total}<span class="sum"><span class="ui-num">{count(orders.total)}</span> in all{#if spent}{' · paid '}<span class="ui-num">{price(spent, currency)}</span> in these{/if}</span>{/if}
    </h3>
    {#if orders.items.length === 0}
      <p class="box-none">No orders.</p>
    {:else}
      <table class="adm-table stack">
        <thead><tr><th>Item</th><th>When</th><th>Note</th><th class="num">Amount</th><th>Status</th></tr></thead>
        <tbody>
          {#each orders.items as o (o.id)}
            {@const r = orderRow(o, accountId, iso)}
            <tr>
              <td class="pk"><strong><bdi>{iso(o.itemName)}</bdi></strong></td>
              <td data-l="When" class="mono">{dateTime(o.createdAt)}</td>
              <td data-l="Note" class="dim">{r.note ?? ''}</td>
              <td data-l="Amount" class="num">{r.received ? 'Gift' : price(o.paidTotalCents ?? o.amountCents, o.paidCurrency ?? o.currency)}</td>
              <td data-l="Status">{orderStatusText(o.status)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</Box>

<style>
  .sum { margin-left: auto; font-weight: 500; letter-spacing: 0; text-transform: none; color: var(--dim); }
  td strong { font-size: var(--t-small); }
  .adm-table { border-top: 0; }
</style>
