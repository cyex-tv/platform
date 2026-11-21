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
      <ul class="ui-list">
        {#each subs as s (s.id)}
          <li>
            <span class="what"><strong>{s.plan}</strong><span class="ui-hint"><span class="ui-num">{date(s.startsAt)}</span>{s.endsAt ? ` to ${date(s.endsAt)}` : ', renews'} · {s.source}</span></span>
            <span class="ui-chip {s.state === 'CURRENT' ? 'approved' : ''}">{STATE[s.state]}</span>
          </li>
        {/each}
      </ul>
    {/if}
    <h3 class="box-sub">
      Orders
      {#if orders.total}<span class="sum"><span class="ui-num">{count(orders.total)}</span> in all{#if spent}{' · paid '}<span class="ui-num">{price(spent, currency)}</span> in these{/if}</span>{/if}
    </h3>
    {#if orders.items.length === 0}
      <p class="box-none">No orders.</p>
    {:else}
      <ul class="ui-list">
        {#each orders.items as o (o.id)}
          {@const r = orderRow(o, accountId, iso)}
          <li>
            <span class="what"><strong><bdi>{iso(o.itemName)}</bdi></strong><span class="ui-hint"><span class="ui-num">{dateTime(o.createdAt)}</span>{#if r.note}{' · '}{r.note}{/if}</span></span>
            <span class="amount"><span class="ui-num">{r.received ? 'Gift' : price(o.paidTotalCents ?? o.amountCents, o.paidCurrency ?? o.currency)}</span><span class="ui-hint">{orderStatusText(o.status)}</span></span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</Box>

<style>
  .sum { margin-left: auto; font-weight: 500; letter-spacing: 0; text-transform: none; color: var(--dim); }
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-size: var(--t-small); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .amount { display: grid; flex: none; text-align: right; font-size: var(--t-small); font-weight: 600; }
  .amount .ui-hint { font-weight: 400; font-size: var(--t-fine); }
</style>
