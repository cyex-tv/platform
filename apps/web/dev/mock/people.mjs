// Mock API: people. See ../mock-api.mjs.
import { quickTier, similar, personMatch } from '../search.mjs';
import { ME_ID, ME_SUMMARY, OTHERS } from './config.mjs';
import { ALL } from './emotes.mjs';
import { CHANNELS, IDENTITIES, myIdentities } from './account.mjs';
import { ACCOUNT_ROLES, ROLES } from './roles.mjs';
import { statusOf } from './admin.mjs';
import { ACCOUNT_EDITORS, EDITOR_CHANNELS } from './editors.mjs';
import { C_SELECTED, cosmeticCatalog } from './cosmetics.mjs';
import { err, findSet } from './sets.mjs';
import { COLLECTION_PUBLIC } from './site.mjs';

// searchAccounts(query, page, perPage): the mock's people by login or display name, exact login
// first, then prefixes, then anything containing the text. Blocked accounts are never listed.
let accountSearches = 0;
// Every login the account has linked: the signed-in account's own identities, the channels the mock
// knows for others (EDITOR_CHANNELS), and a Twitch login made from the display name when none is
// known. The people search matches any of them, not only the display name.
export const searchPeople = () => [ME_SUMMARY, ...OTHERS].map((a) => {
  const list = a.id === ME_ID
    ? IDENTITIES.map((i) => ({ platform: i.platform, login: i.login, displayName: i.displayName }))
    : EDITOR_CHANNELS.filter((c) => c.account.id === a.id).map((c) => ({ platform: c.platform, login: c.login, displayName: a.displayName }));
  if (!list.some((i) => i.platform === 'TWITCH')) list.unshift({ platform: 'TWITCH', login: a.displayName.toLowerCase(), displayName: a.displayName });
  if (a.id === OTHERS[0].id) list.push({ platform: 'KICK', login: 'quietoak_live', displayName: 'QuietOak' });
  const seen = new Set();
  return { account: a, identities: list.filter((i) => !seen.has(`${i.platform}/${i.login}`) && seen.add(`${i.platform}/${i.login}`)) };
});
export function searchAccounts(v) {
  if (++accountSearches > 600) return err('That’s a lot of searches. Try again in a minute.', 'rate_limited');
  const text = String(v.query ?? '').trim().replace(/^@/, '').trim().toLowerCase();
  const per = Math.min(50, Math.max(1, v.perPage ?? 20));
  const page = Math.min(Math.ceil(1000 / per), Math.max(1, v.page ?? 1));
  const hits = [...text].length < 2 ? [] : searchPeople()
    .filter((h) => !statusOf(h.account.id).blocked)
    .map((h) => ({ h, m: personMatch(h.identities, text) }))
    .filter((x) => x.m.tier >= 0)
    .sort((a, b) => b.m.tier - a.m.tier || b.m.score - a.m.score || a.h.account.id.localeCompare(b.h.account.id))
    .map((x) => x.h);
  return { data: { searchAccounts: { total: hits.length, totalCapped: false, page, perPage: per, items: hits.slice((page - 1) * per, page * per) } } };
}

// accountProfile(accountId): public logins (no IDs), roles marked public, each channel's active set.
export function accountProfile(id) {
  const account = [ME_SUMMARY, ...OTHERS].find((a) => a.id === id);
  if (!account || statusOf(id).blocked) return null;
  const identities = id === ME_ID
    ? myIdentities().map((i) => ({ platform: i.platform, login: i.login, displayName: i.displayName }))
    : searchPeople().find((h) => h.account.id === id)?.identities ?? [];
  const roles = (ACCOUNT_ROLES[id] ?? []).map((r) => ROLES.find((x) => x.id === r)).filter((r) => r?.public && !r.invisible)
    .sort((a, b) => b.position - a.position).map((r) => ({ id: r.id, name: r.name }));
  const activeSets = [...CHANNELS, ...EDITOR_CHANNELS].filter((c) => c.account.id === id && c.activeSetId).map((c) => {
    const set = findSet(c.activeSetId);
    return set ? { platform: c.platform, login: c.login, setId: set.id, setName: set.name } : null;
  }).filter(Boolean);
  // Editors, names only (never their permissions).
  const editors = id === ME_ID ? ACCOUNT_EDITORS.map((e) => e.account) : id === OTHERS[0].id ? [ME_SUMMARY, OTHERS[3]] : [];
  return { account, identities, roles, activeSets, editors, createdAt: id === ME_ID ? '2026-11-10T12:00:00.000Z' : '2026-11-08T09:00:00.000Z', collectionPublic: !!COLLECTION_PUBLIC[id] };
}
// accountCosmetics(accountIds): what each account shows, with the badge and paint themselves.
export function accountCosmetics(ids) {
  if (!Array.isArray(ids)) return [];
  if (ids.length > 100) return null;
  const catalog = cosmeticCatalog();
  const out = [];
  for (const id of [...new Set(ids)]) {
    if (statusOf(id).blocked) continue;
    const pick = C_SELECTED[id];
    if (!pick) continue;
    const badge = catalog.badges.find((b) => b.id === pick.badgeId) ?? null;
    const paint = catalog.paints.find((x) => x.id === pick.paintId) ?? null;
    if (badge || paint) out.push({ accountId: id, badge, paint });
  }
  return out;
}

// quickSearch(query): the header search. The same tiers as the API's matcher (crates/db
// search::quick): exact, exact ignoring case, prefix, word or initials prefix, substring, then a
// near miss (shared trigrams, 3+ letters); most used first within a tier. 5 emotes, 3 people.
export function quickSearch(v) {
  const q = String(v.query ?? '').trim().slice(0, 100);
  if (!q) return { data: { quickSearch: { emotes: [], users: [] } } };
  // The same listed-emote rule as search and trending (crates/db search::LISTED_EMOTE): public only.
  const emotes = ALL.filter((e) => e.state === 'APPROVED' && e.visibility === 'PUBLIC' && !e.flags.nsfw)
    .map((e) => ({ e, t: quickTier(e.name, q), s: similar(e.name, q) }))
    .filter((h) => h.t > 0)
    .sort((a, b) => a.t - b.t || (a.t === 6 ? b.s - a.s : 0) || b.e.channelCount - a.e.channelCount || a.e.name.length - b.e.name.length || a.e.name.localeCompare(b.e.name))
    .slice(0, 5)
    .map((h) => h.e);
  const text = q.replace(/^@/, '').toLowerCase();
  const users = [...text].length < 2 ? [] : searchPeople()
    .map((h) => ({ h, m: personMatch(h.identities, text) }))
    .filter((x) => x.m.tier >= 0)
    .sort((a, b) => b.m.tier - a.m.tier || b.m.score - a.m.score || a.h.account.id.localeCompare(b.h.account.id))
    .slice(0, 3)
    .map((x) => x.h);
  return { data: { quickSearch: { emotes, users } } };
}
