// The shop admin's orders list: the filters and the page live in the address (?status=&item=&account=&page=),
// so a reload, Back and a shared link show the same list. Pure, so it is tested.
import type { OrderStatus } from '$lib/gql/graphql';

const STATUSES: readonly OrderStatus[] = ['PAID', 'PENDING', 'REFUNDED', 'FAILED'];

export type OrderFilters = { status: OrderStatus | ''; item: string; account: string; page: number };

/** The filters an address holds; an unknown state or a page that is not a whole number from 1 is dropped. */
export function orderFilters(sp: URLSearchParams): OrderFilters {
  const s = (sp.get('status') ?? '').toUpperCase();
  const p = Number(sp.get('page'));
  return {
    status: (STATUSES as readonly string[]).includes(s) ? (s as OrderStatus) : '',
    item: sp.get('item') ?? '',
    account: sp.get('account') ?? '',
    page: Number.isInteger(p) && p >= 1 && p <= 100000 ? p : 1,
  };
}

/** The address with `patch` applied (null or '' removes a key). A change to a filter goes back to page 1 and closes the picked order. */
export function ordersHref(url: URL, patch: Partial<Record<'status' | 'item' | 'account' | 'page' | 'order', string | number | null>>): string {
  const q = new URLSearchParams(url.searchParams);
  const filterChanged = 'status' in patch || 'item' in patch || 'account' in patch;
  for (const [k, v] of Object.entries(patch)) {
    if (v === null || v === '' || v === undefined || (k === 'page' && Number(v) === 1)) q.delete(k);
    else q.set(k, String(v));
  }
  if (filterChanged && !('page' in patch)) q.delete('page');
  if (filterChanged && !('order' in patch)) q.delete('order');
  const s = q.toString();
  return s ? `?${s}` : url.pathname;
}

/** `{ from: 26, to: 50, pages: 4 }` for page 2 of 4 at 25 a page; from 0 when empty. */
export function pageRange(page: number, per: number, total: number, shown: number): { from: number; to: number; pages: number } {
  const pages = Math.max(1, Math.ceil(total / per));
  if (!shown) return { from: 0, to: 0, pages };
  const from = (page - 1) * per + 1;
  return { from, to: from + shown - 1, pages };
}
