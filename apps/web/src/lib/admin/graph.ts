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

/** How an account came to hold something, in a few words. */
export const SOURCE_LABEL: Record<Grant['source'], string> = {
  STAFF: 'Given by staff',
  PURCHASE: 'Bought',
  CODE: 'Redeemed a code',
  EVENT: 'Event',
  DEFAULT: 'Every account',
  OWNER: 'Owner list',
};

/** The source with its order or code batch: `Bought · Supporter pack`. */
export function how(g: Grant): string {
  const extra = g.orderName ?? g.codeBatchName ?? null;
  return extra ? `${SOURCE_LABEL[g.source]} · ${extra}` : SOURCE_LABEL[g.source];
}

export type HolderSort = 'account' | 'how' | 'given' | 'ended';

/**
 * The holders table of one item: held now only, or everyone who ever held it; narrowed by a
 * search over the account's name (from `nameOf`) and ID; sorted by a column. Ties go newest given
 * first. Pure, so the owners lookup and its CSV agree.
 */
export function holderRows(
  grants: Grant[],
  opts: { show: 'held' | 'ever'; sort: HolderSort; desc: boolean; search?: string; nameOf?: (id: string) => string | undefined },
): Grant[] {
  const name = (g: Grant) => (opts.nameOf?.(g.accountId) ?? '').toLowerCase();
  const q = (opts.search ?? '').trim().toLowerCase();
  const rows = grants.filter(
    (g) => (opts.show === 'ever' || g.active) && (!q || name(g).includes(q) || g.accountId.toLowerCase().includes(q)),
  );
  const key = (g: Grant): string => {
    switch (opts.sort) {
      case 'account':
        return name(g) || g.accountId.toLowerCase();
      case 'how':
        return how(g).toLowerCase();
      case 'ended':
        // Still held counts as ending last: after the ended ones going up, first going down.
        return g.revokedAt ?? '\uffff';
      default:
        return g.grantedAt ?? '';
    }
  };
  const dir = opts.desc ? -1 : 1;
  return rows.sort((a, b) => {
    const x = key(a);
    const y = key(b);
    if (x !== y) return x < y ? -dir : dir;
    return (b.grantedAt ?? '').localeCompare(a.grantedAt ?? '');
  });
}

/** A CSV field: quoted when it needs to be; a leading = + - @ is defused for spreadsheets. */
function field(v: string | null | undefined): string {
  let s = v ?? '';
  if (/^[=+\-@\t\r]/.test(s)) s = `'${s}`;
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/** The holders as CSV (with a header row), for a spreadsheet. */
export function holdersCsv(rows: Grant[], nameOf: (id: string) => string | undefined = () => undefined): string {
  const head = ['account_id', 'account', 'held_now', 'how', 'given_by', 'given_at', 'reason', 'ended_at', 'ended_by', 'end_reason'];
  const lines = rows.map((g) =>
    [
      g.accountId,
      nameOf(g.accountId) ?? '',
      g.active ? 'yes' : 'no',
      how(g),
      g.grantedBy ?? '',
      g.grantedAt ?? '',
      g.reason ?? '',
      g.revokedAt ?? '',
      g.revokedBy ?? '',
      g.revokeReason ?? '',
    ]
      .map(field)
      .join(','),
  );
  return [head.join(','), ...lines].join('\r\n') + '\r\n';
}
