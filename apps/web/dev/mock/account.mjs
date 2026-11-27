// Mock API: account. See ../mock-api.mjs.
import { randomBytes } from 'node:crypto';
import { ME_ID, ME_SUMMARY, OTHERS } from './config.mjs';
import { ALL, DAY, MINE } from './emotes.mjs';
import { SETS } from './set-data.mjs';
import { ACCOUNT_ROLES, ROLES } from './roles.mjs';
import { ACCOUNT_EDITORS, EDITOR_OF } from './editors.mjs';
import { err, setOut } from './sets.mjs';

// A deletion within its 30-day undo window (ADR 0019): set by deleteAccount, cleared by
// restoreAccount. Signing in while it is set gives the mock_restore cookie, not a session.
export let PENDING_DELETION = null;
export let RESTORE_COOKIE = false;
export const setPendingDeletion = (v) => { PENDING_DELETION = v; };
export const setRestoreCookie = (v) => { RESTORE_COOKIE = v; };
// As the API's me.roles: with every permission (the mock's owner) the top role comes first, hidden.
export const MY_ROLES = (admin) => [
  ...(admin ? [{ id: ROLES.find((r) => r.builtIn === 'ROOT')?.id ?? 'owner', name: 'Owner', public: false, invisible: true }] : []),
  ...(ACCOUNT_ROLES[ME_ID] ?? []).map((id) => ROLES.find((r) => r.id === id)).filter(Boolean)
    .sort((a, b) => b.position - a.position).filter((r) => r.builtIn !== 'ROOT').map((r) => ({ id: r.id, name: r.name, public: r.public, invisible: !!r.invisible })),
];

// Every account has a default set named after it (made at sign-up).
SETS.push({ id: '01JD8XSET000000000000DEFLT', name: ME_SUMMARY.displayName, kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
  emotes: [], editors: [], visibility: 'LISTED', createdAt: '2026-11-10T12:00:00.000Z' });

export const CHANNELS = [
  { platform: 'TWITCH', platformChannelId: '141981764', login: 'lumen_fox', activeSetId: SETS[0].id, allowPersonalEmotes: true, account: ME_SUMMARY },
];
export const IDENTITIES = [
  { platform: 'TWITCH', platformUserId: '141981764', login: 'lumen_fox', displayName: 'lumen_fox', avatar: ME_SUMMARY.avatar },
];
// MOCK_KICK=1: the mock account has Kick linked too (to try the display name choice and Disconnect).
if (process.env.MOCK_KICK === '1') IDENTITIES.push({ platform: 'KICK', platformUserId: '4815162', login: 'lumenfox_kick', displayName: 'LumenFoxKick', avatar: null });
// setDisplayPlatform: which identity the account goes by (null: Twitch first). Listed first, and
// the account's name everywhere follows it.
export let DISPLAY_PLATFORM = null;
export const setDisplayPlatform = (v) => { DISPLAY_PLATFORM = v; };
export const myIdentities = () => [...IDENTITIES].sort((a, b) => (b.platform === DISPLAY_PLATFORM) - (a.platform === DISPLAY_PLATFORM) || b.platform.localeCompare(a.platform));
export function goBy() {
  const first = myIdentities()[0];
  if (first) Object.assign(ME_SUMMARY, { displayName: first.displayName, avatar: first.avatar });
}
export const NOTES = [
  { id: '01JD8XNOTE0000000000000006', kind: 'cosmetic.granted', payload: JSON.stringify({ kind: 'paint', cosmetic_id: 'P1', name: 'Sample sunrise', reason: 'Thanks for testing' }), createdAt: '2026-11-17T09:30:00.000Z', read: false },
  { id: '01JD8XNOTE0000000000000005', kind: 'editor.added', payload: JSON.stringify({ role: 'editor', owner_id: OTHERS[4].id, owner_name: OTHERS[4].displayName, editor_id: ME_ID, editor_name: ME_SUMMARY.displayName, actor_id: OTHERS[4].id, actor_name: OTHERS[4].displayName, permissions: ['emotes', 'profile'] }), createdAt: '2026-11-15T20:00:00.000Z', read: false },
  { id: '01JD8XNOTE0000000000000004', kind: 'editor.changed', payload: JSON.stringify({ role: 'owner', owner_id: ME_ID, owner_name: ME_SUMMARY.displayName, editor_id: OTHERS[1].id, editor_name: OTHERS[1].displayName, actor_id: OTHERS[2].id, actor_name: OTHERS[2].displayName, permissions: ['emotes'] }), createdAt: '2026-11-14T19:00:00.000Z', read: true },
  { id: '01JD8XNOTE0000000000000003', kind: 'emote.approved', payload: JSON.stringify({ emote_id: MINE[3].id, name: MINE[3].name, visibility: 'public', public_listing: 'approved', personal_use: 'pending', reason: null }), createdAt: '2026-11-16T22:10:00.000Z', read: false },
  { id: '01JD8XNOTE0000000000000011', kind: 'emote.approvals_changed', payload: JSON.stringify({ emote_id: MINE[4].id, name: MINE[4].name, visibility: 'unlisted', public_listing: 'denied', personal_use: 'approved', reason: 'Looks too much like another emote for the public listing.' }), createdAt: '2026-11-16T22:40:00.000Z', read: false },
  { id: '01JD8XNOTE0000000000000012', kind: 'emote.transferred', payload: JSON.stringify({ role: 'to', emote_id: ALL[30].id, name: ALL[30].name, from_id: OTHERS[1].id, from_name: OTHERS[1].displayName, to_id: ME_ID, to_name: ME_SUMMARY.displayName, actor_id: OTHERS[2].id, actor_name: 'cyex staff' }), createdAt: '2026-11-16T20:00:00.000Z', read: true },
  { id: 'N2', kind: 'emote.failed', payload: JSON.stringify({ emote_id: MINE[9].id, name: MINE[9].name }), createdAt: '2026-11-16T21:30:00.000Z', read: false },
  { id: 'N1', kind: 'emote.rejected', payload: JSON.stringify({ emote_id: MINE[2].id, name: MINE[2].name, reason: 'Looks like a copy of a well-known emote.' }), createdAt: '2026-11-16T21:00:00.000Z', read: false },
];
NOTES.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
// MOCK_NOTES=none: no notifications; MOCK_NOTES=120: that many unread ones (the API lists the latest 100).
if (process.env.MOCK_NOTES === 'none') NOTES.length = 0;
else if (Number(process.env.MOCK_NOTES) > NOTES.length) {
  const kinds = ['emote.approved', 'emote.rejected', 'cosmetic.granted', 'editor.added'];
  for (let i = NOTES.length; i < Number(process.env.MOCK_NOTES); i++)
    NOTES.push({ id: `01JD8XBULK${String(i).padStart(16, '0')}`, kind: kinds[i % 4], payload: JSON.stringify({ emote_id: MINE[i % MINE.length].id, name: `Bulk${i}`, kind: 'paint', role: 'editor', owner_name: 'Ana', reason: 'Sample' }), createdAt: new Date(Date.parse('2026-11-14T00:00:00Z') - i * 3_600_000).toISOString(), read: false });
  NOTES.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

// An editor's transfer request from before requests were retired (ADR 0014, amendment
// 2026/11/22): the old notification, and the withdrawal both sides got at start-up.
{
  const e = MINE[5];
  const by = ACCOUNT_EDITORS[1].account;
  const p = { emote_id: e.id, name: e.name, request_id: '01JD8XTRANSFER000000000001', from_id: ME_ID, from_name: ME_SUMMARY.displayName,
    to_id: by.id, to_name: by.displayName, actor_id: by.id, actor_name: by.displayName };
  NOTES.unshift({ id: '01JD8XNOTE0000000000000009', kind: 'emote.transfer_requested', read: true, createdAt: '2026-11-19T09:00:00.000Z',
    payload: JSON.stringify({ role: 'owner', ...p }) });
  NOTES.unshift({ id: '01JD8XNOTE0000000000000010', kind: 'emote.transfer_cancelled', read: false, createdAt: '2026-11-22T08:00:00.000Z',
    payload: JSON.stringify({ role: 'owner', reason: 'retired', ...p }) });
}

// Sessions (mySessions, revokeSession, revokeOtherSessions): where the mock account is signed in.
const SESSIONS = [
  { id: '01JD8XSESS0000000000000001', kind: 'SITE', device: 'Firefox on Windows', createdAt: '2026-11-10T12:00:00.000Z', lastSeenAt: '2026-11-17T22:40:00.000Z', expiresAt: '2026-12-17T22:40:00.000Z', current: true, appClientId: null },
  { id: '01JD8XSESS0000000000000002', kind: 'SITE', device: 'Safari on iPhone', createdAt: '2026-11-12T08:15:00.000Z', lastSeenAt: '2026-11-16T07:55:00.000Z', expiresAt: '2026-12-16T07:55:00.000Z', current: false, appClientId: null },
  { id: '01JD8XSESS0000000000000003', kind: 'APP', device: 'cyex deck', createdAt: '2026-11-11T18:00:00.000Z', lastSeenAt: '2026-11-17T21:05:00.000Z', expiresAt: '2027-01-16T21:05:00.000Z', current: false, appClientId: 'cyex-deck' },
  { id: '01JD8XSESS0000000000000004', kind: 'SITE', device: 'Chrome on Android', createdAt: '2026-11-02T10:00:00.000Z', lastSeenAt: '2026-11-03T10:00:00.000Z', expiresAt: '2026-12-03T10:00:00.000Z', current: false, appClientId: null },
];
let exports = 0;
// Console tokens (ADR 0028, 0029), as the API makes them: one per account, 3 days, listed in
// mySessions (and myConsoleToken), ended whenever a session signs out, one or all.
// The token is a fake; nothing here accepts it as a Bearer token.
let consoleSeq = 0;
export function mintConsoleToken() {
  const i = SESSIONS.findIndex((x) => x.kind === 'CONSOLE');
  if (i >= 0) SESSIONS.splice(i, 1);
  consoleSeq += 1;
  const now = new Date();
  const id = `01JD8XSESSC${String(consoleSeq).padStart(15, '0')}`;
  const expiresAt = new Date(now.getTime() + 3 * 86400_000).toISOString();
  SESSIONS.push({ id, kind: 'CONSOLE', device: 'Console token', createdAt: now.toISOString(), lastSeenAt: now.toISOString(), expiresAt, current: false, appClientId: null });
  const token = `mockconsole${randomBytes(24).toString('base64url')}`.slice(0, 43);
  return { token, tokenType: 'Bearer', expiresAt, sessionId: id };
}
/** Signing a session out rotates the console token (ADR 0029): the API ends it. */
export function endConsoleToken() {
  const i = SESSIONS.findIndex((x) => x.kind === 'CONSOLE');
  if (i >= 0) SESSIONS.splice(i, 1);
}
if (process.env.MOCK_LONG === '1') SESSIONS[1].device = 'Mozilla Firefox Developer Edition on Windows 11 Enterprise N with a rather long device name that keeps going and going';

// exportMyData: the API's document shape, filled from the mock's data.
const exportDoc = () => JSON.stringify({
  format: 'cyex.tv account export', version: 1, exported_at: new Date().toISOString(),
  account: { id: ME_ID, created_at: '2026-11-10T12:00:00.000Z', uploads_blocked: false },
  identities: IDENTITIES.map((i) => ({ platform: i.platform.toLowerCase(), platform_user_id: i.platformUserId, login: i.login, display_name: i.displayName, avatar: i.avatar, linked_at: '2026-11-10T12:00:00.000Z', refreshed_at: '2026-11-17T08:00:00.000Z' })),
  sessions: SESSIONS,
  channels: CHANNELS.map((c) => ({ platform: c.platform.toLowerCase(), platform_channel_id: c.platformChannelId, login: c.login, active_set_id: c.activeSetId, allow_personal_emotes: c.allowPersonalEmotes, created_at: '2026-11-10T12:00:00.000Z' })),
  emotes: MINE, sets: SETS.filter((x) => x.owner?.id === ME_ID).map(setOut),
  set_editors: SETS.filter((x) => x.owner?.id === ME_ID && x.editors.length).map((x) => ({ set_id: x.id, editor_ids: x.editors.map((e) => e.id) })),
  account_editors: ACCOUNT_EDITORS.map((e) => ({ account_id: ME_ID, editor_id: e.account.id, permissions: e.permissions.map((f) => f.toLowerCase()), granted_at: e.grantedAt })),
  editor_of: EDITOR_OF.map((g) => ({ account_id: g.account.id, editor_id: ME_ID, permissions: g.permissions.map((f) => f.toLowerCase()), granted_at: g.grantedAt })),
  cosmetics: { badges: [], paints: [], badge_id: null, paint_id: null }, roles: [],
  reports_filed: [], notifications: NOTES.map((n) => ({ id: n.id, kind: n.kind, payload: JSON.parse(n.payload), created_at: n.createdAt, read_at: n.read ? n.createdAt : null })),
  orders: [],
});
export function accountHub(q, v) {
  const data = {};
  if (/\bmySessions\b/.test(q)) data.mySessions = SESSIONS;
  if (/\bmyConsoleToken\b/.test(q)) data.myConsoleToken = SESSIONS.find((x) => x.kind === 'CONSOLE') ?? null;
  if (/\bcreateConsoleToken\b/.test(q)) data.createConsoleToken = mintConsoleToken();
  if (/\brevokeSession\s*\(/.test(q)) {
    const i = SESSIONS.findIndex((x) => x.id === v.id);
    if (i >= 0) {
      SESSIONS.splice(i, 1);
      endConsoleToken();
    }
    data.revokeSession = i >= 0;
  }
  if (/\bsignOutEverywhere\b/.test(q)) {
    const n = SESSIONS.length;
    SESSIONS.splice(0, SESSIONS.length);
    return { data: { signOutEverywhere: n }, _cookie: 'mock_session=; Path=/; Max-Age=0' };
  }
  if (/\brevokeOtherSessions\b/.test(q)) {
    data.revokeOtherSessions = SESSIONS.filter((x) => !x.current).length;
    SESSIONS.splice(0, SESSIONS.length, ...SESSIONS.filter((x) => x.current));
  }
  if (/\bexportMyData\b/.test(q)) {
    if (++exports > 5) return err('You asked for a copy of your data a few times already. Try again in an hour.', 'rate_limited');
    data.exportMyData = exportDoc();
  }
  if (/\bdeleteAccount\s*\(/.test(q)) {
    const typed = String(v.confirm ?? '').trim().replace(/^@/, '').toLowerCase();
    if (!IDENTITIES.some((i) => i.login.toLowerCase() === typed))
      return err('Type one of your Twitch or Kick logins to confirm.', 'confirm_mismatch');
    // The mock keeps its data (it would have nothing left to show); it signs you out like the API
    // and remembers the 30-day window, so signing in again lands on /restore.
    const now = Date.now();
    PENDING_DELETION = { requestedAt: new Date(now).toISOString(), eraseAfter: new Date(now + 30 * DAY).toISOString(), displayName: IDENTITIES[0]?.displayName ?? null, avatar: IDENTITIES[0]?.avatar ?? null };
    return { data: { deleteAccount: true }, _cookie: 'mock_session=; Path=/; Max-Age=0' };
  }
  if (/\bmarkNotificationsRead\b/.test(q)) {
    if (Array.isArray(v.ids) && v.ids.length > 100) return err('Mark at most 100 notifications at once.', 'too_many');
    const pick = Array.isArray(v.ids) ? NOTES.filter((n) => v.ids.includes(n.id)) : NOTES;
    data.markNotificationsRead = pick.filter((x) => !x.read).length;
    pick.forEach((x) => (x.read = true));
  }
  if (/\bunreadNotificationCount\b/.test(q)) data.unreadNotificationCount = NOTES.filter((n) => !n.read).length;
  return Object.keys(data).length ? { data } : null;
}
