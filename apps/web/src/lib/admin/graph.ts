// The entitlement graph as a tree: the account (or the item) at the root, then one branch per
// source (a staff member, an order, a code batch, the User role, the owner list), then what came
// from it. Pure, so the shape is unit-tested; the page draws it and offers a list beside it.
import type { GrantKind, GrantSource } from '$lib/gql/graphql';

export type Grant = {
  kind: `${GrantKind}`;
  itemId: string;
  itemName: string;
  accountId: string;
  source: `${GrantSource}`;
  reason?: string | null;
  grantedBy?: string | null;
  grantedAt?: string | null;
  orderId?: string | null;
  orderName?: string | null;
  orderStatus?: string | null;
  codeBatchId?: string | null;
  codeBatchName?: string | null;
  revokedAt?: string | null;
  revokedBy?: string | null;
  revokeReason?: string | null;
  active: boolean;
};

/** A branch: where things came from. `staffId` is set for staff grants (the name is looked up). */
export type Branch = {
  key: string;
  source: Grant['source'];
  label: string;
  staffId?: string | null;
  href?: string | null;
  grants: Grant[];
};

const KIND_ORDER: Grant['kind'][] = ['ROLE', 'BADGE', 'PAINT'];
const SOURCE_ORDER: Grant['source'][] = ['OWNER', 'DEFAULT', 'STAFF', 'PURCHASE', 'CODE', 'EVENT'];

export const KIND_LABEL: Record<Grant['kind'], string> = { ROLE: 'Role', BADGE: 'Badge', PAINT: 'Paint' };

function branchOf(g: Grant, by: 'source' | 'account'): Omit<Branch, 'grants'> {
  if (by === 'account') return { key: `a:${g.accountId}`, source: g.source, label: '', staffId: g.accountId, href: `/admin/users/${encodeURIComponent(g.accountId)}` };
  switch (g.source) {
    case 'OWNER':
      return { key: 'owner', source: g.source, label: 'The owner list' };
    case 'DEFAULT':
      return { key: 'default', source: g.source, label: 'The User role' };
    case 'PURCHASE':
      return g.orderId
        ? { key: `o:${g.orderId}`, source: g.source, label: `Order: ${g.orderName ?? 'shop item'}`, href: `/admin/shop/orders?order=${encodeURIComponent(g.orderId)}` }
        : { key: 'purchase', source: g.source, label: 'Shop purchase' };
    case 'CODE':
      return g.codeBatchId
        ? { key: `c:${g.codeBatchId}`, source: g.source, label: `Code batch: ${g.codeBatchName ?? 'redeem code'}`, href: '/admin/shop/codes' }
        : { key: 'code', source: g.source, label: 'Redeem code' };
    case 'EVENT':
      return { key: 'event', source: g.source, label: 'Event' };
    default:
      return g.grantedBy
        ? { key: `s:${g.grantedBy}`, source: g.source, label: '', staffId: g.grantedBy, href: `/admin/users/${encodeURIComponent(g.grantedBy)}` }
        : { key: 'staff', source: g.source, label: 'Staff (unknown)' };
  }
}

/**
 * Groups grants into branches. `by: 'source'` for one account (where each thing came from);
 * `by: 'account'` for one item (who holds it). Branches with something held come first; inside a
 * branch, held before taken away, then roles, badges, paints, then newest first.
 */
export function branches(grants: Grant[], by: 'source' | 'account' = 'source', show: 'all' | 'held' = 'all'): Branch[] {
  const map = new Map<string, Branch>();
  for (const g of grants) {
    if (show === 'held' && !g.active) continue;
    const b = branchOf(g, by);
    const into = map.get(b.key) ?? { ...b, grants: [] };
    into.grants.push(g);
    map.set(b.key, into);
  }
  const time = (g: Grant) => g.revokedAt ?? g.grantedAt ?? '';
  for (const b of map.values())
    b.grants.sort((x, y) => Number(y.active) - Number(x.active) || KIND_ORDER.indexOf(x.kind) - KIND_ORDER.indexOf(y.kind) || time(y).localeCompare(time(x)));
  return [...map.values()].sort(
    (x, y) =>
      Number(y.grants.some((g) => g.active)) - Number(x.grants.some((g) => g.active)) ||
      SOURCE_ORDER.indexOf(x.source) - SOURCE_ORDER.indexOf(y.source) ||
      x.key.localeCompare(y.key),
  );
}

/** Counts for the summary line: held now, by kind, and how many were taken away. */
export function tally(grants: Grant[]) {
  const held = grants.filter((g) => g.active);
  return {
    roles: held.filter((g) => g.kind === 'ROLE').length,
    badges: held.filter((g) => g.kind === 'BADGE').length,
    paints: held.filter((g) => g.kind === 'PAINT').length,
    gone: grants.length - held.length,
  };
}
