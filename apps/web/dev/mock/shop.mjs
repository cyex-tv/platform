// Mock API: shop. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY, OTHERS, OWNERS, PADDLE_DELAY, PADDLE_MOCK } from './config.mjs';
import { BADGES, C_GRANTS, C_OWNED, C_SELECTED, C_TIME, PAINTS } from './cosmetics.mjs';
import { publicAccount } from './directory.mjs';
import { err } from './sets.mjs';

// ---- Shop and redeem codes (ADR 0016) ---------------------------------------------------
// SAMPLE DATA ONLY: four made-up shop items (bundles of the sample badges and paints above, one of
// them hidden), three sample orders and two sample code batches. The fake checkout COMPLETES
// IMMEDIATELY: startCheckout makes the order PAID at once and grants the cosmetics, with no payment
// page (checkoutUrl and clientToken are null). The known sample code is SAMPLE-CODE-0001 (also
// "sample code 0001"; codes are compared normalized) and the disabled one OLD-CODE-0001. Operations
// are recognised by their root field; arguments are read from variables named like the arguments
// ($id, $input, $all, $itemId, $recipient, $code, $reason, $status, $accountId, $page, $perPage).
const shopId = (prefix, n) => prefix + String(n).padStart(26 - prefix.length, '0');
const S_FUTURE = '2027-06-01T00:00:00.000Z';
const mkItem = (n, rest) => ({ id: shopId('01JD8XSHP', n), description: 'Sample item (mock data)', currency: 'EUR', activeFrom: null,
  activeTo: null, stockLimit: null, visible: true, sortOrder: n, deleted: false, createdAt: C_TIME, updatedAt: C_TIME, ...rest });
export const SHOP_ITEMS = [
  mkItem(1, { name: 'Sample: Sunset pack', priceCents: 499, paddlePriceId: 'pri_01mocksample1', badgeIds: [BADGES[0].id], paintIds: [PAINTS[0].id] }),
  mkItem(2, { name: 'Sample: Aurora', priceCents: 299, paddlePriceId: 'pri_01mocksample2', badgeIds: [], paintIds: [PAINTS[1].id] }),
  mkItem(3, { name: 'Sample: Ember collector', priceCents: 799, paddlePriceId: 'pri_01mocksample3', stockLimit: 50, badgeIds: [BADGES[1].id], paintIds: [PAINTS[2].id] }),
  mkItem(4, { name: 'Sample: Coming soon', priceCents: 999, paddlePriceId: 'pri_01mocksample4', visible: false, activeFrom: S_FUTURE, badgeIds: [BADGES[0].id], paintIds: [PAINTS[2].id] }),
];
let orderSeq = 0;
const mkOrder = (buyerId, recipientId, item, status, at, rest = {}) => ({
  id: shopId('01JD8XORD', ++orderSeq), buyerId, recipientId, gift: buyerId !== recipientId, itemId: item.id, itemName: item.name, status,
  amountCents: item.priceCents, currency: item.currency, paidTotalCents: status === 'FAILED' ? null : item.priceCents,
  paidCurrency: status === 'FAILED' ? null : item.currency, provider: 'paddle', providerTransactionId: `txn_mock_${orderSeq}`,
  refundReason: null, createdAt: at, paidAt: status === 'FAILED' ? null : at, refundedAt: null, failedAt: null, refundRejectedAt: null, ...rest });
export const ORDERS = [
  mkOrder(ME_ID, ME_ID, SHOP_ITEMS[0], 'PAID', '2026-11-14T10:00:00.000Z'),
  mkOrder(ME_ID, ME_ID, SHOP_ITEMS[1], 'REFUNDED', '2026-11-12T09:00:00.000Z', { refundReason: 'Sample refund (mock data)', refundedAt: '2026-11-13T09:00:00.000Z' }),
  mkOrder(OTHERS[0].id, ME_ID, SHOP_ITEMS[2], 'PAID', '2026-11-15T18:00:00.000Z'),
  mkOrder(ME_ID, OTHERS[1].id, SHOP_ITEMS[1], 'PAID', '2026-11-13T12:00:00.000Z'), // a gift I sent
  // Paddle rejected this refund after the cosmetics were taken back (the staff flag).
  mkOrder(OTHERS[1].id, OTHERS[1].id, SHOP_ITEMS[0], 'REFUNDED', '2026-11-10T09:00:00.000Z', { refundReason: 'Sample refund (mock data)', refundedAt: '2026-11-11T09:00:00.000Z', refundRejectedAt: '2026-11-11T09:05:00.000Z' }),
  // A refund already running: the first try answers refund_in_progress, the second goes through.
  mkOrder(OTHERS[2].id, OTHERS[2].id, SHOP_ITEMS[1], 'PAID', '2026-11-16T09:00:00.000Z', { refundBusy: true }),
];
const normCode = (c) => String(c ?? '').toUpperCase().replace(/[\s-]/g, '').replace(/O/g, '0').replace(/[IL]/g, '1');
const mkBatch = (n, rest) => ({ id: shopId('01JD8XCBT', n), itemId: null, expiresAt: null, disabledAt: null, createdBy: ME_SUMMARY, createdAt: C_TIME, ...rest });
const CODE_BATCHES = [
  mkBatch(1, { name: 'Sample giveaway', prefix: 'SAMPLE', usesPerCode: 100, badgeIds: [BADGES[1].id], paintIds: [PAINTS[2].id], codes: [normCode('SAMPLE-CODE-0001')] }),
  mkBatch(2, { name: 'Sample: old stream', prefix: 'OLD', usesPerCode: 1, badgeIds: [BADGES[0].id], paintIds: [], disabledAt: '2026-11-10T12:00:00.000Z', codes: [normCode('OLD-CODE-0001')] }),
];
const REDEMPTIONS = []; // { batchId, code, accountId }
const CODE_FAILS = []; // times of wrong codes (the mock has one signed-in account)
const IDEMPOTENT = new Map(); // `${account}:${op}:${key}` → { request, answer } (kept until restart)
const CODE_ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

const itemOf = (id) => SHOP_ITEMS.find((i) => i.id === id);
const stockLeftOf = (item) => (item.stockLimit == null ? null
  : Math.max(0, item.stockLimit - ORDERS.filter((o) => o.itemId === item.id && o.status === 'PAID').length));
const ownsAll = (accountId, badgeIds, paintIds) => {
  const o = C_OWNED[accountId] ?? { BADGE: [], PAINT: [] };
  return badgeIds.length + paintIds.length > 0 && badgeIds.every((id) => o.BADGE.includes(id)) && paintIds.every((id) => o.PAINT.includes(id));
};
const isAvailable = (item) => {
  const now = Date.now();
  return item.visible && !item.deleted && (!item.activeFrom || Date.parse(item.activeFrom) <= now) && (!item.activeTo || Date.parse(item.activeTo) > now)
    && stockLeftOf(item) !== 0 && item.badgeIds.length + item.paintIds.length > 0;
};
const shopItemOut = (item, session) => ({
  ...item, badges: BADGES.filter((b) => item.badgeIds.includes(b.id)), paints: PAINTS.filter((p) => item.paintIds.includes(p.id)),
  stockLeft: stockLeftOf(item), available: isAvailable(item), owned: Boolean(session) && ownsAll(ME_ID, item.badgeIds, item.paintIds),
});
const summaryOf = (id) => (id ? publicAccount(id) ?? { id, displayName: '', avatar: null } : null);
const orderOut = (o) => ({ ...o, buyer: summaryOf(o.buyerId), recipient: summaryOf(o.recipientId) });
// As the API: a gift's recipient sees it once paid, without what the buyer paid or the transaction.
const shownToRecipient = (o) => o.status === 'PAID' || o.status === 'REFUNDED';
const partyOf = (o) => o.buyerId === ME_ID || (o.recipientId === ME_ID && shownToRecipient(o));
const partyOut = (o) => (o.buyerId === ME_ID ? orderOut(o)
  : { ...orderOut(o), amountCents: 0, paidTotalCents: null, paidCurrency: null, providerTransactionId: null });
const batchCosmetics = (b) => {
  const item = b.itemId ? itemOf(b.itemId) : null;
  return { badgeIds: [...new Set([...(item?.badgeIds ?? []), ...b.badgeIds])], paintIds: [...new Set([...(item?.paintIds ?? []), ...b.paintIds])] };
};
const batchOut = (b) => ({ id: b.id, name: b.name, prefix: b.prefix, itemId: b.itemId, itemName: b.itemId ? itemOf(b.itemId)?.name ?? null : null,
  badgeIds: b.badgeIds, paintIds: b.paintIds, codeCount: b.codes.length, usesPerCode: b.usesPerCode,
  redemptions: REDEMPTIONS.filter((r) => r.batchId === b.id).length, expiresAt: b.expiresAt, disabledAt: b.disabledAt, createdBy: b.createdBy, createdAt: b.createdAt });
// Gives the account what it does not own yet; returns the new cosmetics.
function grantAll(accountId, badgeIds, paintIds, source, reason) {
  const owned = (C_OWNED[accountId] ??= { BADGE: [], PAINT: [] });
  const fresh = { BADGE: badgeIds.filter((id) => !owned.BADGE.includes(id)), PAINT: paintIds.filter((id) => !owned.PAINT.includes(id)) };
  for (const kind of ['BADGE', 'PAINT']) for (const cosmeticId of fresh[kind]) {
    owned[kind].push(cosmeticId);
    C_GRANTS.unshift({ accountId, kind, cosmeticId, source, reason, grantedBy: ME_SUMMARY, grantedAt: new Date().toISOString() });
  }
  return { badges: BADGES.filter((b) => fresh.BADGE.includes(b.id)), paints: PAINTS.filter((p) => fresh.PAINT.includes(p.id)) };
}
function checkShopItem(input) {
  const name = input?.name;
  if (typeof name !== 'string' || !name.trim() || name.trim().length > 64) return ['Names are 1 to 64 characters.', 'bad_name'];
  const badgeIds = input.badgeIds ?? [];
  const paintIds = input.paintIds ?? [];
  if (badgeIds.length + paintIds.length < 1 || badgeIds.length + paintIds.length > 10 || !badgeIds.every((id) => BADGES.some((b) => b.id === id))
    || !paintIds.every((id) => PAINTS.some((p) => p.id === id))) return ['Pick 1 to 10 existing badges and paints.', 'bad_cosmetics'];
  if (typeof input.paddlePriceId !== 'string' || !/^pri_[a-z0-9]{1,64}$/.test(input.paddlePriceId)) return ['Use a Paddle price ID like pri_01abc.', 'bad_price_id'];
  if (!Number.isInteger(input.priceCents) || input.priceCents < 0 || input.priceCents > 100000) return ['The price is 0 to 100000 cents.', 'bad_price'];
  return null;
}
const shopItemFrom = (id, input, before) => ({
  ...(before ?? { sortOrder: 0, deleted: false, createdAt: new Date().toISOString() }), id, name: input.name.trim(), description: input.description ?? '',
  priceCents: input.priceCents, currency: input.currency ?? 'EUR', paddlePriceId: input.paddlePriceId, badgeIds: input.badgeIds ?? [], paintIds: input.paintIds ?? [],
  activeFrom: input.activeFrom ?? null, activeTo: input.activeTo ?? null, stockLimit: input.stockLimit ?? null, visible: input.visible ?? true,
  sortOrder: input.sortOrder ?? 0, updatedAt: new Date().toISOString(),
});
const pageOf = (rows, v, map) => {
  const per = Math.min(100, Math.max(1, v.perPage ?? 50));
  const page = Math.max(1, v.page ?? 1);
  return { total: rows.length, items: rows.slice((page - 1) * per, page * per).map(map) };
};

// The shop and redeem-code root fields. Returns null when the operation has none.
export function shopAnswer(q, v, session, admin) {
  const has = (field) => new RegExp(`\\b${field}\\s*[({]`).test(q) || new RegExp(`\\b${field}\\s*$`, 'm').test(q);
  const mutation = /^\s*mutation/.test(q);
  const staff = (perm) => (!session ? err('Sign in to use this.', 'signed_out') : admin ? null : err(`This needs ${perm}.`, 'forbidden'));
  const signedOut = () => (session ? null : err('Sign in to use this.', 'signed_out'));
  if (!mutation) {
    const fields = ['shopItems', 'shopItem', 'myOrders', 'order', 'orders', 'codeBatches'].filter(has);
    if (!fields.length) return null;
    const data = {};
    for (const f of fields) {
      if (f === 'shopItems') {
        if (v.all) {
          const denied = staff('shop.manage');
          if (denied) return denied;
        }
        const rows = SHOP_ITEMS.filter((i) => (v.all ? true : i.visible && !i.deleted)).sort((a, b) => a.sortOrder - b.sortOrder);
        data.shopItems = rows.map((i) => shopItemOut(i, session));
      }
      if (f === 'shopItem') {
        const item = itemOf(v.id);
        data.shopItem = item && (isAvailable(item) || (session && admin)) ? shopItemOut(item, session) : null;
      }
      if (f === 'myOrders') {
        const denied = signedOut();
        if (denied) return denied;
        const rows = ORDERS.filter(partyOf).sort((a, b) => b.createdAt.localeCompare(a.createdAt));
        data.myOrders = pageOf(rows, v, partyOut);
      }
      if (f === 'order') {
        const o = ORDERS.find((x) => x.id === v.id);
        if (o && o.status === 'PENDING' && PADDLE_MOCK && Date.now() - Date.parse(o.createdAt) > PADDLE_DELAY) {
          Object.assign(o, { status: 'PAID', paidAt: new Date().toISOString(), paidTotalCents: o.amountCents, paidCurrency: o.currency });
          const it = itemOf(o.itemId);
          if (it) grantAll(o.recipientId, it.badgeIds, it.paintIds, 'PURCHASE', `Order ${o.id} (mock data)`);
        }
        data.order = !o || !session ? null : admin ? orderOut(o) : partyOf(o) ? partyOut(o) : null;
      }
      if (f === 'orders') {
        const denied = staff('shop.orders');
        if (denied) return denied;
        const rows = ORDERS.filter((o) => (!v.status || o.status === v.status) && (!v.itemId || o.itemId === v.itemId)
          && (!v.accountId || o.buyerId === v.accountId || o.recipientId === v.accountId)).sort((a, b) => b.createdAt.localeCompare(a.createdAt));
        data.orders = pageOf(rows, v, orderOut);
      }
      if (f === 'codeBatches') {
        const denied = staff('codes.manage');
        if (denied) return denied;
        data.codeBatches = pageOf([...CODE_BATCHES].reverse(), v, batchOut);
      }
    }
    return { data };
  }
  for (const [field, perm] of [['createShopItem', 'shop.manage'], ['updateShopItem', 'shop.manage'], ['deleteShopItem', 'shop.manage'],
    ['refundOrder', 'shop.orders'], ['createCodeBatch', 'codes.manage'], ['disableCodeBatch', 'codes.manage']]) {
    if (!has(field)) continue;
    const denied = staff(perm);
    if (denied) return denied;
    const input = v.input ?? {};
    switch (field) {
      case 'createShopItem':
      case 'updateShopItem': {
        const before = field === 'updateShopItem' ? itemOf(v.id) : null;
        if (field === 'updateShopItem' && (!before || before.deleted)) return err('Not found.', 'not_found');
        const bad = checkShopItem(input);
        if (bad) return err(...bad);
        const item = shopItemFrom(before?.id ?? shopId('01JD8XSHP', 100 + SHOP_ITEMS.length), input, before);
        if (before) SHOP_ITEMS[SHOP_ITEMS.indexOf(before)] = item;
        else SHOP_ITEMS.push(item);
        return { data: { [field]: shopItemOut(item, session) } };
      }
      case 'deleteShopItem': {
        const item = itemOf(v.id);
        if (!item || item.deleted) return { data: { deleteShopItem: false } };
        item.deleted = true;
        item.updatedAt = new Date().toISOString();
        return { data: { deleteShopItem: true } };
      }
      case 'refundOrder': {
        const o = ORDERS.find((x) => x.id === v.id);
        if (!o) return err('Not found.', 'not_found');
        if (typeof v.reason !== 'string' || !v.reason.trim()) return err('Give a reason of 1 to 500 characters.', 'bad_reason');
        if (v.reason.trim().length > 500) return err('Give a reason of 1 to 500 characters.', 'bad_reason');
        if (o.status === 'REFUNDED') return err('That order was refunded already.', 'already_refunded');
        if (o.refundBusy) { o.refundBusy = false; return err('A refund for that order is already in progress.', 'refund_in_progress'); }
        if (o.status !== 'PAID') return err('Only paid orders can be refunded.', 'not_refundable');
        o.status = 'REFUNDED';
        o.refundReason = v.reason.trim();
        o.refundedAt = new Date().toISOString();
        const item = itemOf(o.itemId);
        const owned = C_OWNED[o.recipientId];
        for (const [kind, key, ids] of [['BADGE', 'badgeId', item?.badgeIds ?? []], ['PAINT', 'paintId', item?.paintIds ?? []]]) {
          if (!owned) break;
          owned[kind] = owned[kind].filter((id) => !ids.includes(id));
          const sel = C_SELECTED[o.recipientId];
          if (sel && ids.includes(sel[key])) sel[key] = null;
          for (let i = C_GRANTS.length - 1; i >= 0; i--) {
            const g = C_GRANTS[i];
            if (g.accountId === o.recipientId && g.kind === kind && ids.includes(g.cosmeticId)) C_GRANTS.splice(i, 1);
          }
        }
        return { data: { refundOrder: orderOut(o) } };
      }
      case 'createCodeBatch': {
        const prefix = String(input.prefix ?? '').toUpperCase();
        if (typeof input.name !== 'string' || !input.name.trim() || input.name.trim().length > 64) return err('Names are 1 to 64 characters.', 'bad_name');
        if (!/^[A-Z0-9]{1,12}$/.test(prefix)) return err('Use 1 to 12 letters and digits.', 'bad_prefix');
        if (!Number.isInteger(input.count) || input.count < 1 || input.count > 1000) return err('Make 1 to 1000 codes.', 'bad_count');
        if (!Number.isInteger(input.usesPerCode) || input.usesPerCode < 1 || input.usesPerCode > 10000) return err('A code can be used 1 to 10000 times.', 'bad_uses');
        const badgeIds = input.badgeIds ?? [];
        const paintIds = input.paintIds ?? [];
        if (input.itemId && !itemOf(input.itemId)) return err('Not found.', 'not_found');
        if ((!input.itemId && !badgeIds.length && !paintIds.length) || !badgeIds.every((id) => BADGES.some((b) => b.id === id))
          || !paintIds.every((id) => PAINTS.some((p) => p.id === id))) return err('Pick an item or 1 or more existing badges and paints.', 'bad_cosmetics');
        const group = () => Array.from({ length: 4 }, () => CODE_ALPHABET[Math.floor(Math.random() * 32)]).join('');
        const taken = new Set(CODE_BATCHES.flatMap((b) => b.codes));
        const shown = [];
        while (shown.length < input.count) {
          const code = `${prefix}-${group()}-${group()}-${group()}-${group()}`;
          if (!taken.has(normCode(code))) { taken.add(normCode(code)); shown.push(code); }
        }
        const batch = mkBatch(100 + CODE_BATCHES.length, { name: input.name.trim(), prefix, usesPerCode: input.usesPerCode, itemId: input.itemId ?? null,
          expiresAt: input.expiresAt ?? null, badgeIds, paintIds, codes: shown.map(normCode), createdAt: new Date().toISOString() });
        CODE_BATCHES.push(batch);
        return { data: { createCodeBatch: { batch: batchOut(batch), codes: shown } } };
      }
      case 'disableCodeBatch': {
        const b = CODE_BATCHES.find((x) => x.id === v.id);
        if (!b) return err('Not found.', 'not_found');
        b.disabledAt ??= new Date().toISOString();
        return { data: { disableCodeBatch: batchOut(b) } };
      }
    }
  }
  // idempotencyKey, as the real API: a repeat with the same key answers the first result; the same
  // key with another request is refused; a failed request gives its key back.
  const keyed = (op, request, run) => {
    if (!v.idempotencyKey) return run();
    const at = `${ME_ID}:${op}:${v.idempotencyKey}`;
    const seen = IDEMPOTENT.get(at);
    const same = JSON.stringify(request);
    if (seen) return seen.request === same ? seen.answer : err('That idempotency key was used for a different request. Make a new one for each action.', 'idempotency_key_reused');
    const answer = run();
    if (!answer?.errors) IDEMPOTENT.set(at, { request: same, answer });
    return answer;
  };
  if (has('startCheckout')) {
    const denied = signedOut();
    if (denied) return denied;
    return keyed('startCheckout', [v.itemId, v.recipient ?? null], () => startCheckout(v));
  }
  if (has('redeemCode')) {
    const denied = signedOut();
    if (denied) return denied;
    return keyed('redeemCode', [normCode(v.code)], () => redeemCode(v));
  }
  return null;
}

function startCheckout(v) {
  {
    const item = itemOf(v.itemId);
    if (!item || item.deleted) return err('Not found.', 'not_found');
    if (stockLeftOf(item) === 0) return err('This item is sold out.', 'sold_out');
    if (!isAvailable(item)) return err('This item is not on sale.', 'item_unavailable');
    const login = String(v.recipient?.login ?? '').toLowerCase();
    // No recipient: for the buyer themselves, as the real API does.
    const recipient = v.recipient ? (login ? OWNERS.find((a) => a.displayName.toLowerCase() === login) : null) : ME_SUMMARY;
    if (!recipient) return err('No cyex.tv account with that name.', 'recipient_not_found');
    if (ownsAll(recipient.id, item.badgeIds, item.paintIds)) return err('They own all of that already.', 'already_owned');
    const now = new Date().toISOString();
    if (PADDLE_MOCK) {
      // MOCK_PADDLE=1: like the real thing, the order waits for payment (PENDING) and a client token
      // is given; it turns PAID PADDLE_DELAY ms later, when `order` is asked for (the webhook).
      const pending = mkOrder(ME_ID, recipient.id, item, 'PENDING', now, { paidTotalCents: null, paidCurrency: null, paidAt: null });
      ORDERS.push(pending);
      return { data: { startCheckout: { order: orderOut(pending), transactionId: pending.providerTransactionId, checkoutUrl: null, clientToken: 'test_mockmockmockmock', environment: 'sandbox' } } };
    }
    const order = mkOrder(ME_ID, recipient.id, item, 'PAID', now);
    ORDERS.push(order);
    grantAll(recipient.id, item.badgeIds, item.paintIds, 'PURCHASE', `Order ${order.id} (mock data)`);
    return { data: { startCheckout: { order: orderOut(order), transactionId: order.providerTransactionId, checkoutUrl: null, clientToken: null, environment: 'sandbox' } } };
  }
}

function redeemCode(v) {
  {
    const now = Date.now();
    while (CODE_FAILS.length && now - CODE_FAILS[0] > 15 * 60 * 1000) CODE_FAILS.shift();
    if (CODE_FAILS.length >= 5) return err('Too many wrong codes. Try again in 15 min.', 'locked_out');
    const code = normCode(v.code);
    const batch = code ? CODE_BATCHES.find((b) => b.codes.includes(code)) : null;
    if (!batch) {
      CODE_FAILS.push(now);
      return err('That code is not valid.', 'bad_code');
    }
    if (batch.disabledAt) return err('That code has been turned off.', 'code_disabled');
    if (batch.expiresAt && Date.parse(batch.expiresAt) <= now) return err('That code has expired.', 'code_expired');
    const uses = REDEMPTIONS.filter((r) => r.code === code);
    if (uses.some((r) => r.accountId === ME_ID)) return err('You used that code already.', 'already_redeemed');
    if (uses.length >= batch.usesPerCode) return err('That code has been used up.', 'code_used_up');
    const { badgeIds, paintIds } = batchCosmetics(batch);
    if (ownsAll(ME_ID, badgeIds, paintIds)) return err('You own all of that already.', 'already_owned');
    const fresh = grantAll(ME_ID, badgeIds, paintIds, 'CODE', `Code from ${batch.name} (mock data)`);
    REDEMPTIONS.push({ batchId: batch.id, code, accountId: ME_ID });
    return { data: { redeemCode: { name: batch.name, ...fresh } } };
  }
}
