// Mock API: top people. See ../mock-api.mjs.
import { ME_ID, OTHERS } from './config.mjs';
import { reviewQueueAnswer } from './review.mjs';
import { ACCOUNT_ROLES } from './roles.mjs';
import { AUDIT, statusOf } from './admin.mjs';
import { ORDERS } from './shop.mjs';
import { ACTIVITY, emoteActivityRows, filtered } from './activity.mjs';
import { publicAccount } from './directory.mjs';
import { err } from './sets.mjs';
import { searchPeople } from './people.mjs';

// Fields added after the site's first queries, answered by field name so any operation name works.
// ---- "Most important people" for the person filters (apps/api/src/top_people.rs) --------------
// Counts per account, most first (ties: the first seen, as the API's newest-first), as people
// with their logins; `limit` 1 to 10, default 5.
function topPeople(ids, v) {
  const n = new Map();
  for (const id of ids) if (id) n.set(id, (n.get(id) ?? 0) + 1);
  const people = searchPeople();
  const limit = Math.min(10, Math.max(1, v.limit ?? 5));
  return [...n].sort((a, b) => b[1] - a[1]).map(([id, count]) => {
    const p = people.find((h) => h.account.id === id) ?? (publicAccount(id) ? { account: publicAccount(id), identities: [] } : null);
    return p && !statusOf(id).blocked ? { ...p, count } : null;
  }).filter(Boolean).slice(0, limit);
}
export function topPeopleAnswer(q, v, session, admin) {
  const has = (name) => new RegExp(`\\b${name}\\b`).test(q);
  const staff = (perm) => (!session ? err('Sign in to use this.', 'signed_out') : admin ? null : err(`This needs ${perm}.`, 'forbidden'));
  const data = {};
  if (has('activityTopActors')) {
    const a = (v.accountId ?? '').trim(), e = (v.emoteId ?? '').trim();
    if (!a === !e) return err('Give either `accountId` or `emoteId`, not both.', 'bad_scope');
    const rows = a ? (statusOf(a).blocked ? [] : filtered(ACTIVITY[a] ?? [], { ...(v.filter ?? {}), actorId: null })) : emoteActivityRows({ emoteId: e });
    data.activityTopActors = topPeople(rows.map((r) => r.actor?.id), v);
  }
  if (has('auditTopPeople')) {
    const no = staff('audit.view');
    if (no) return no;
    const since = v.since ? Date.parse(v.since) : null, until = v.until ? Date.parse(v.until) : null;
    const target = v.by === 'TARGET';
    const rows = AUDIT.filter((r) => (!v.targetKind || r.targetKind === v.targetKind) && (!target || r.targetKind === 'account')
      && (!v.targetId || r.targetId === v.targetId) && (!v.actorId || r.actorId === v.actorId) && (!v.action || r.action === v.action)
      && (since === null || Date.parse(r.createdAt) >= since) && (until === null || Date.parse(r.createdAt) < until));
    data.auditTopPeople = topPeople(rows.map((r) => (target ? r.targetId : r.actorId)), v);
  }
  if (has('orderTopBuyers')) {
    const no = staff('shop.orders');
    if (no) return no;
    const rows = ORDERS.filter((o) => (!v.status || o.status === v.status || (v.status === 'PAID' && o.status === 'REFUND_REQUESTED')) && (!v.itemId || o.itemId === v.itemId));
    data.orderTopBuyers = topPeople(rows.map((o) => o.buyerId), v);
  }
  if (has('reviewTopUploaders')) {
    const no = staff('emotes.review');
    if (no) return no;
    const f = { ...(v.filter ?? {}), uploaderId: null };
    data.reviewTopUploaders = topPeople(reviewQueueAnswer({ filter: f, perPage: 100 }).data.reviewQueue.items.map((e) => e.owner?.id), v);
  }
  if (has('entitlementTopHolders')) {
    const no = staff('entitlements.view');
    if (no) return no;
    const ids = [];
    for (const [id, roles] of Object.entries(ACCOUNT_ROLES)) for (const _ of roles) ids.push(id);
    data.entitlementTopHolders = topPeople([ME_ID, ME_ID, ME_ID, ...ids, ...OTHERS.slice(2, 5).map((o) => o.id)], v);
  }
  return Object.keys(data).length ? { data } : null;
}
