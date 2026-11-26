<script lang="ts">
  // Your orders: what you bought and what you were given, newest first, with the state, the amount
  // paid and who a gift came from or went to. It lives in Account (one place). The reason staff
  // give for a refund is theirs: it is never shown here.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import Section from '$lib/components/Section.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count, dateTime, iso } from '$lib/format';
  import type { OrderFieldsFragment } from '$lib/gql/graphql';
  import { MY_ORDERS } from '$lib/queries';
  import { fail } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { appendUnique, pageToken } from '$lib/tool/paging';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { orderRow, orderStatusText, type OrderStatus } from './shop';

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
  const row = (o: OrderFieldsFragment) => orderRow(o, mine, iso);
</script>

<Section title="Orders" id="orders-h">
  {#snippet actions()}
    <a class="ui-btn ghost small" href="/shop">Shop<Icon name="arrow-right" size={14} /></a>
  {/snippet}
  <LoadGate source={L} label="Loading your orders…">
    {#snippet children(r)}
      {@const rows = appendUnique(r.items, more)}
      {#if !rows.length}
        <State bare title="No orders yet.">What you buy or are given in the shop is listed here.</State>
      {:else}
        <ul class="orders" aria-label="Orders">
          {#each rows as o (o.id)}
            <li>
              <span class="what">
                <strong><bdi>{clean(o.itemName)}</bdi></strong>
                <span class="meta"><time class="ui-mono" datetime={o.createdAt}>{dateTime(o.createdAt)}</time>{#if row(o).note}{' · '}{row(o).note}{/if}</span>
              </span>
              <span class="amt ui-mono">{row(o).amount}</span>
              <span class="st" class:dim={o.status !== 'PAID'}>{orderStatusText(o.status as OrderStatus)}</span>
            </li>
          {/each}
        </ul>
        {#if rows.length < r.total}
          <div class="foot">
            <button class="ui-btn secondary small" type="button" disabled={busy} onclick={next}>{busy ? 'Loading…' : 'Show more'}</button>
            <span class="ui-hint ui-mono">{count(rows.length)} of {count(r.total)}</span>
          </div>
        {/if}
      {/if}
    {/snippet}
  </LoadGate>
</Section>

<style>
  .orders { margin: 0; padding: 0; list-style: none; }
  .orders > li { display: flex; align-items: center; gap: 16px; min-height: 52px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .what { display: grid; gap: 2px; flex: 1; min-width: 0; }
  .what strong { font-size: var(--t-ui); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .amt { flex: none; font-size: var(--t-small); font-weight: 600; text-align: right; }
  .st { flex: none; min-width: 64px; text-align: right; font-size: var(--t-fine); color: var(--muted); }
  .dim { color: var(--dim); }
  .foot { display: flex; align-items: center; gap: 12px; padding: 12px 0; }
  @media (max-width: 639.98px) {
    .orders > li { flex-wrap: wrap; row-gap: 4px; }
    .what { flex-basis: 100%; }
    .amt { margin-right: auto; text-align: left; }
  }
</style>
