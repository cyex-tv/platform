// Mock API: entitlements. See ../mock-api.mjs.
import { ME_ID, OTHERS } from './config.mjs';
import { ACCOUNT_ROLES, ROLES } from './roles.mjs';

// Staff notes, newest first, per account.
export const STAFF_NOTES = {
  '01JD8X0000000000000000NORA': [
    { id: 'NOTE2', authorId: '01JD8X0000000000000000OAK1', body: 'Asked about the paint refund; told them refunds go through the order page.', createdAt: '2026-11-16T15:20:00.000Z' },
    { id: 'NOTE1', authorId: ME_ID, body: 'Warned once for report spam (three reports on the same emote in an hour).', createdAt: '2026-11-12T09:05:00.000Z' },
  ],
};
// The entitlement graph: where each cosmetic and role came from, per account.
export const grant = (o) => ({ reason: null, grantedBy: null, grantedAt: null, orderId: null, orderName: null, orderStatus: null, codeBatchId: null, codeBatchName: null,
  revokedAt: null, revokedBy: null, revokeReason: null, active: true, ...o });
export const graphOf = (id) => {
  const base = [grant({ kind: 'ROLE', itemId: 'ROLEUSER', itemName: 'User', accountId: id, source: 'DEFAULT' })];
  if (id === ME_ID) return [grant({ kind: 'ROLE', itemId: 'ROLEROOT', itemName: 'Owner', accountId: id, source: 'OWNER' }), ...base];
  const roles = (ACCOUNT_ROLES[id] ?? []).map((r) => ROLES.find((x) => x.id === r)).filter(Boolean)
    .map((r, i) => grant({ kind: 'ROLE', itemId: r.id, itemName: r.name, accountId: id, source: 'STAFF', grantedBy: ME_ID, grantedAt: `2026-11-${String((i % 8) + 2).padStart(2, '0')}T10:00:00.000Z` }));
  return [
    ...roles,
    grant({ kind: 'BADGE', itemId: 'BADGE1', itemName: 'Early supporter', accountId: id, source: 'STAFF', reason: 'Beta tester', grantedBy: ME_ID, grantedAt: '2026-11-05T12:00:00.000Z' }),
    grant({ kind: 'PAINT', itemId: 'PAINT1', itemName: 'Sunset', accountId: id, source: 'PURCHASE', orderId: 'ORDER1', orderName: 'Sunset pack', orderStatus: 'paid', grantedAt: '2026-11-14T18:00:00.000Z' }),
    grant({ kind: 'PAINT', itemId: 'PAINT2', itemName: 'Aurora', accountId: id, source: 'CODE', codeBatchId: 'BATCH1', codeBatchName: 'Launch week', grantedAt: '2026-11-10T08:00:00.000Z' }),
    grant({ kind: 'BADGE', itemId: 'BADGE2', itemName: 'Event host', accountId: id, source: 'STAFF', grantedBy: '01JD8X0000000000000000OAK1', grantedAt: '2026-11-03T12:00:00.000Z', revokedAt: '2026-11-09T12:00:00.000Z', revokedBy: ME_ID, revokeReason: 'Event over', active: false }),
    grant({ kind: 'PAINT', itemId: 'PAINT3', itemName: 'Ember', accountId: id, source: 'PURCHASE', orderId: 'ORDER2', orderName: 'Ember collector', orderStatus: 'refunded', grantedAt: '2026-11-11T10:00:00.000Z', revokedAt: '2026-11-12T10:00:00.000Z', revokeReason: 'Order refunded', active: false }),
    ...richGraph(id),
    ...base,
  ];
};
// A real fan-out for the graph: nora_byte has bought the monthly supporter pack for a year (each
// month's order a bundle of 5 to 8 paints), a founders bundle, redeemed several code batches, was
// in two events and was given things by two staff members; a few other accounts bought some of the
// same months, so a monthly paint's holders graph has several holders. About 150 nodes for nora.
const MONTH_WORDS = ['Tide', 'Frost', 'Ember', 'Moss', 'Dune', 'Comet', 'Lumen', 'Cinder', 'Fern', 'Halo', 'Quartz', 'Drift', 'Petal', 'Storm', 'Saffron', 'Indigo', 'Velvet', 'Rust', 'Glacier', 'Nova'];
const MONTH_NAMES = ['December', 'January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November'];
const monthPaints = (m) => Array.from({ length: 5 + ((m * 3) % 4) }, (_, i) => ({ id: `MPNT${String(m).padStart(2, '0')}${i}`, name: `${MONTH_WORDS[(m * 7 + i * 3) % MONTH_WORDS.length]} ${MONTH_NAMES[m].slice(0, 3).toLowerCase()} ${i + 1}` }));
const monthStart = (m) => new Date(Date.UTC(2025, 11 + m, 1, 9, 0)).toISOString();
const RICH_MONTHS = { '01JD8X0000000000000000NORA': [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] };
OTHERS.slice(2, 9).forEach((o, i) => (RICH_MONTHS[o.id] = [6, 7, 8, 9, 10, 11].filter((m) => m !== 6 + (i % 6))));
function richGraph(id) {
  const months = RICH_MONTHS[id];
  if (!months) return [];
  const out = months.flatMap((m) => monthPaints(m).map((p) => grant({ kind: 'PAINT', itemId: p.id, itemName: p.name, accountId: id, source: 'PURCHASE',
    orderId: `ORDM${id.slice(-4)}${m}`, orderName: 'Supporter monthly', orderStatus: m === 4 && id.endsWith('NORA') ? 'refunded' : 'paid', grantedAt: monthStart(m),
    ...(m === 4 && id.endsWith('NORA') ? { active: false, revokedAt: '2026-04-03T10:00:00.000Z', revokeReason: 'Order refunded' } : {}) })));
  if (!id.endsWith('NORA')) return out;
  const at = (d) => `2026-${d}T12:00:00.000Z`;
  const item = (kind, n, name, o) => grant({ kind, itemId: `${kind[0]}X${n}`, itemName: name, accountId: id, ...o });
  return [
    ...out,
    ...['Founder', 'First hundred'].map((n, i) => item('BADGE', 10 + i, n, { source: 'PURCHASE', orderId: 'ORDFOUND', orderName: 'Founders bundle', orderStatus: 'paid', grantedAt: at('01-15') })),
    ...['Gilded', 'Obsidian', 'Opal', 'Brass'].map((n, i) => item('PAINT', 10 + i, n, { source: 'PURCHASE', orderId: 'ORDFOUND', orderName: 'Founders bundle', orderStatus: 'paid', grantedAt: at('01-15') })),
    ...['Snowfall', 'Pine', 'Aurora night', 'Hearth', 'Icicle'].map((n, i) => item('PAINT', 20 + i, n, { source: 'CODE', codeBatchId: 'BATCHWINTER', codeBatchName: 'Twitch drop: Winter', grantedAt: at('01-08') })),
    ...['Creator', 'Mossgate crew'].map((n, i) => item(i ? 'PAINT' : 'BADGE', 30 + i, n, { source: 'CODE', codeBatchId: 'BATCHMOSS', codeBatchName: 'Creator code: mossgate', grantedAt: at('06-20') })),
    item('PAINT', 40, 'Pumpkin', { source: 'EVENT', reason: 'Halloween 2026', grantedAt: at('10-31') }),
    item('PAINT', 41, 'Witchlight', { source: 'EVENT', reason: 'Halloween 2026', grantedAt: at('10-31') }),
    item('BADGE', 42, 'Spooky', { source: 'EVENT', reason: 'Halloween 2026', grantedAt: at('10-31') }),
    item('PAINT', 43, 'Bonfire', { source: 'EVENT', reason: 'Halloween 2026', grantedAt: at('10-31') }),
    item('BADGE', 44, 'Jam winner', { source: 'EVENT', reason: 'Emote jam: spring', grantedAt: at('04-12') }),
    item('BADGE', 45, 'Jam entrant', { source: 'EVENT', reason: 'Emote jam: spring', grantedAt: at('04-12') }),
    item('PAINT', 46, 'Confetti', { source: 'EVENT', reason: 'Emote jam: spring', grantedAt: at('04-12') }),
    // The same paint reached two ways: bought in the founders bundle and given again by staff.
    item('PAINT', 10, 'Gilded', { source: 'STAFF', reason: 'Lost it in the refund mix-up', grantedBy: ME_ID, grantedAt: at('05-02') }),
    ...['Translator', 'Bug hunter', 'Helper'].map((n, i) => item('BADGE', 50 + i, n, { source: 'STAFF', reason: 'Community work', grantedBy: ME_ID, grantedAt: at(`0${7 + i}-01`) })),
    item('PAINT', 54, 'Staff pick', { source: 'STAFF', reason: 'Featured emote artist', grantedBy: '01JD8X0000000000000000OAK1', grantedAt: at('09-14') }),
    item('BADGE', 55, 'Moderator alumni', { source: 'STAFF', grantedBy: '01JD8X0000000000000000OAK1', grantedAt: at('03-01'), revokedAt: at('08-01'), revokedBy: ME_ID, revokeReason: 'Stepped down', active: false }),
  ];
}
