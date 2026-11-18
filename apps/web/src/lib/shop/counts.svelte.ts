// The numbers on the admin rail's Shop rows and the overview cards: items not deleted, all orders,
// code batches. Each is asked for only when the account holds the permission behind it.
import { gql } from '$lib/api';
import { SHOP_COUNTS } from '$lib/queries';
import { can } from '$lib/tool/session.svelte';

export const shopCounts = $state<{ items?: number; orders?: number; codes?: number }>({});

/** Asks again (after a create, a refund or a new batch the page already knows the result of, call with `fresh`). */
export function loadShopCounts(): Promise<void> {
  const items = can('shop.manage');
  const orders = can('shop.orders');
  const codes = can('codes.manage');
  if (!items && !orders && !codes) return Promise.resolve();
  return gql(SHOP_COUNTS, { items, orders, codes })
    .then((r) => {
      if (r.shopItems) shopCounts.items = r.shopItems.length;
      if (r.orders) shopCounts.orders = r.orders.total;
      if (r.codeBatches) shopCounts.codes = r.codeBatches.total;
    })
    .catch(() => {
      // The rows just have no number.
    });
}
