// Mock API: activity. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY, OTHERS } from './config.mjs';
import { ALL, MINE, NOW } from './emotes.mjs';
import { SETS } from './set-data.mjs';
import { statusOf } from './admin.mjs';
import { SHARED_SETS } from './editors.mjs';
import { BADGES, PAINTS } from './cosmetics.mjs';
import { err, findEmote } from './sets.mjs';

// ---- Account activity (accountActivity) -------------------------------------------------
// Sample public activity, newest first, for lumen_fox (the mock signed-in account), quietoak and
// sunnyfern. Same shape as the real API: pending/private emotes, moderation and blocked accounts
// never appear, so none are here. Everyone else has none; blocked accounts answer an empty list.
const act = (n, minsAgo, kind, actor, rest) => ({
  id: `01JD8XACT${String(9000 - n).padStart(17, '0')}`, kind, actor, set: null, emote: null, fromName: null, toName: null,
  badge: null, paint: null, editor: null, createdAt: new Date(NOW - minsAgo * 60_000).toISOString(), ...rest,
});
const aSet = (s) => ({ id: s.id, name: s.name });
const aEmote = (e, name = e.name) => ({ id: e.id, name, host: e.host, files: e.files });
const aChange = (from, to) => ({ fromId: from?.id ?? null, fromName: from?.name ?? null, toId: to?.id ?? null, toName: to?.name ?? null });
const [FOX_MAIN, FOX_SUB, OAK_MAIN] = SETS;
const [OAK_COZY, FERN] = SHARED_SETS;
const [NORA, PIXEL] = [OTHERS[1], OTHERS[2]];
export const ACTIVITY = {
  [ME_ID]: [
    act(1, 6, 'SET_EMOTE_ADDED', ME_SUMMARY, { set: aSet(FOX_MAIN), emote: aEmote(ALL[12]) }),
    act(2, 41, 'SET_EMOTE_RENAMED', NORA, { set: aSet(FOX_MAIN), emote: aEmote(ALL[14], 'KEKwait'), fromName: ALL[14].name, toName: 'KEKwait' }),
    act(3, 95, 'COSMETICS_SELECTED', ME_SUMMARY, { paint: aChange(PAINTS[0], PAINTS[1]) }),
    act(4, 180, 'SET_EMOTE_REMOVED', PIXEL, { set: aSet(FOX_MAIN), emote: aEmote(ALL[21]) }),
    act(5, 300, 'EMOTE_UPLOADED', ME_SUMMARY, { emote: aEmote(MINE[3]) }),
    act(6, 22 * 60, 'ACTIVE_SET_CHANGED', PIXEL, { set: aSet(FOX_MAIN), fromName: FOX_SUB.name, toName: FOX_MAIN.name }),
    act(7, 26 * 60, 'SET_RENAMED', ME_SUMMARY, { set: aSet(FOX_SUB), fromName: 'Subs', toName: FOX_SUB.name }),
    act(8, 2 * 1440, 'COSMETICS_SELECTED', ME_SUMMARY, { badge: aChange(null, BADGES[0]), paint: aChange(null, PAINTS[0]) }),
    act(9, 2 * 1440 + 30, 'SET_CREATED', ME_SUMMARY, { set: aSet(FOX_SUB) }),
    act(10, 4 * 1440, 'SET_EMOTE_ADDED', ME_SUMMARY, { set: aSet(FOX_MAIN), emote: aEmote(MINE[4]) }),
    act(11, 6 * 1440, 'SET_CREATED', ME_SUMMARY, { set: aSet(FOX_MAIN) }),
    act(12, 6 * 1440 - 20, 'EDITOR_ADDED', ME_SUMMARY, { editor: NORA }),
    act(13, 6 * 1440 - 40, 'EDITOR_ADDED', ME_SUMMARY, { editor: PIXEL }),
    act(14, 40 * 1440, 'EDITOR_REMOVED', OTHERS[5], { editor: OTHERS[5] }),
  ],
  [OTHERS[0].id]: [
    act(21, 12, 'SET_EMOTE_ADDED', OTHERS[0], { set: aSet(OAK_MAIN), emote: aEmote(ALL[62]) }),
    act(22, 130, 'SET_EMOTE_ADDED', ME_SUMMARY, { set: aSet(OAK_COZY), emote: aEmote(ALL[72]) }),
    act(23, 1500, 'COSMETICS_SELECTED', OTHERS[0], { badge: aChange(BADGES[0], BADGES[1]), paint: aChange(null, PAINTS[2]) }),
    act(24, 3 * 1440, 'SET_CREATED', OTHERS[0], { set: aSet(OAK_COZY) }),
  ],
  [OTHERS[4].id]: [
    act(31, 55, 'ACTIVE_SET_CHANGED', ME_SUMMARY, { set: aSet(FERN), fromName: null, toName: FERN.name }),
    act(32, 70, 'SET_EMOTE_REMOVED', OTHERS[4], { set: aSet(FERN), emote: aEmote(ALL[48]) }),
    act(33, 2 * 1440, 'COSMETICS_SELECTED', OTHERS[4], { paint: aChange(null, PAINTS[1]) }),
  ],
};
// Long feeds, to see the visitor's cap (the newest 100 lines) and the owner's date range: 140
// older adds for lumen_fox and quietoak, one every 9 hours going back from 4 days ago.
for (const [who, set, base] of [[ME_SUMMARY, FOX_MAIN, 100], [OTHERS[0], OAK_MAIN, 300]]) {
  for (let i = 0; i < 140; i++) ACTIVITY[who.id].push(act(base + i, 4 * 1440 + i * 540, 'SET_EMOTE_ADDED', who, { set: aSet(set), emote: aEmote(ALL[(i * 7) % 60]) }));
}
// `accountActivity(accountId, before, limit)`: newest first, `limit` 1 to 100 (default 30).
export function activityAnswer(v) {
  if (statusOf(v.accountId).blocked) return { data: { accountActivity: [] } };
  const limit = Math.min(100, Math.max(1, v.limit ?? 30));
  let rows = ACTIVITY[v.accountId] ?? [];
  if (v.before) rows = rows.filter((a) => a.id < v.before);
  return { data: { accountActivity: rows.slice(0, limit) } };
}

// A page with its cursor, as the API answers `accountActivityPage` and `emoteActivityPage`: `next`
// is the entry to carry on after, or null at the end. `shortFirst` makes the first page come back
// short with a `next`, like a real feed that skipped rows nobody may see.
function activityPageOf(rows, v, shortFirst = 0) {
  const limit = Math.min(100, Math.max(1, v.limit ?? 30));
  let from = rows;
  if (v.before) from = from.filter((a) => a.id < v.before);
  const take = !v.before && shortFirst ? Math.min(limit, shortFirst) : limit;
  const entries = from.slice(0, take);
  const more = from.length > entries.length;
  return { entries, next: more ? entries.at(-1)?.id ?? v.before ?? null : null };
}
// The `filter` as the API reads it: groups (editor lines only when EDITORS is asked for), the
// actor (an upload counts as the owner's), and since (inclusive) / until (exclusive), RFC 3339.
const GROUP_OF = { SET_EMOTE_ADDED: 'EMOTES', SET_EMOTE_REMOVED: 'EMOTES', SET_EMOTE_RENAMED: 'EMOTES', EMOTE_UPLOADED: 'EMOTES',
  SET_CREATED: 'SETS', SET_RENAMED: 'SETS', ACTIVE_SET_CHANGED: 'SETS', COSMETICS_SELECTED: 'COSMETICS', EDITOR_ADDED: 'EDITORS', EDITOR_REMOVED: 'EDITORS' };
export function filtered(rows, f) {
  const groups = f?.groups?.length ? f.groups : ['EMOTES', 'SETS', 'COSMETICS'];
  return rows.filter((a) => groups.includes(GROUP_OF[a.kind])
    && (!f?.actorId || a.actor?.id === f.actorId)
    && (!f?.since || Date.parse(a.createdAt) >= Date.parse(f.since))
    && (!f?.until || Date.parse(a.createdAt) < Date.parse(f.until)));
}
// Anyone but the owner and staff reads the newest 100 lines, all time: their since/until are
// ignored and paging ends at the 100th line (owner rule, 2026/11/22; as the API).
const PUBLIC_LINES = 100;
export function activityPageAnswer(v, session, admin) {
  if (statusOf(v.accountId).blocked) return { data: { accountActivityPage: { entries: [], next: null } } };
  let f = v.filter ?? null;
  for (const k of ['since', 'until']) if (f?.[k] && Number.isNaN(Date.parse(f[k]))) return err(`${k} must be an RFC 3339 time.`, 'bad_time');
  const all = !!session && (v.accountId === ME_ID || !!admin);
  if (!all && f) f = { ...f, since: null, until: null };
  const short = v.accountId === ME_ID && !f?.actorId && !f?.since ? 4 : 0;
  const rows = filtered(ACTIVITY[v.accountId] ?? [], f);
  return { data: { accountActivityPage: activityPageOf(all ? rows : rows.slice(0, PUBLIC_LINES), v, short) } };
}

// `emoteActivity(emoteId, before, limit)`: the sample lines about this emote from every account's
// feed, plus an upload line and a few adds for any public emote, newest first.
export function emoteActivityAnswer(v) {
  const rows = emoteActivityRows(v);
  return { data: { emoteActivity: rows.slice(0, Math.min(100, Math.max(1, v.limit ?? 30))) } };
}
export function emoteActivityPageAnswer(v) {
  const rows = emoteActivityRows(v).filter((a) => !v.actorId || a.actor?.id === v.actorId);
  return { data: { emoteActivityPage: activityPageOf(rows, v) } };
}
// Every line about the emote (before paging), newest first; none unless it is public.
export function emoteActivityRows(v) {
  const e = findEmote(v.emoteId ?? v.id);
  if (!e || e.state !== 'APPROVED' || e.flags.private) return [];
  const own = Object.values(ACTIVITY).flat().filter((a) => a.emote?.id === e.id);
  const n = Number(e.id.slice(-3)) || 0;
  if (n === 9) return []; // a public emote nobody has touched
  // Emote 8 has 130 lines, to page through.
  const many = n === 8 ? Array.from({ length: 130 }, (_, k) => act(4000 + k, 10 + k * 37, k % 3 === 0 ? 'SET_EMOTE_REMOVED' : 'SET_EMOTE_ADDED', OTHERS[k % OTHERS.length], { set: aSet(SETS[k % SETS.length]), emote: aEmote(e) })) : [];
  const made = [
    ...many,
    act(900 + n * 4, 90 + n, 'SET_EMOTE_ADDED', OTHERS[n % OTHERS.length], { set: aSet(SETS[n % SETS.length]), emote: aEmote(e) }),
    act(901 + n * 4, 600 + n, 'SET_EMOTE_RENAMED', OTHERS[(n + 1) % OTHERS.length], { set: aSet(SETS[(n + 1) % SETS.length]), emote: aEmote(e, e.name + 'x'), fromName: e.name, toName: e.name + 'x' }),
    act(902 + n * 4, 3000 + n, 'SET_EMOTE_ADDED', OTHERS[(n + 2) % OTHERS.length], { set: aSet(SETS[(n + 2) % SETS.length]), emote: aEmote(e) }),
    act(903 + n * 4, 6000 + n, 'EMOTE_UPLOADED', e.owner ?? ME_SUMMARY, { emote: aEmote(e) }),
  ];
  return [...own, ...made].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}
