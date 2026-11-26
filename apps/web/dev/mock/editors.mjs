// Mock API: editors. See ../mock-api.mjs.
import { BASE, ME_ID, ME_SUMMARY, OTHERS } from './config.mjs';
import { ALL, member } from './emotes.mjs';
import { CHANNELS } from './account.mjs';
import { SETS } from './set-data.mjs';
import { err } from './sets.mjs';

// Account editors (ADR 0014): two people work on the mock account, and it works on two others'.
// Flags in the API's order. SETS and CHANNELS are separate (ADR 0014, amendment 2026/11/22): SETS
// makes, renames, deletes and resizes sets and picks the active one; CHANNELS changes channel
// settings (and may pick the active set too, as before). A grant covers every set (allSets) or setIds.
// EMOTE_TRANSFER is retired (ADR 0014, amendment 2026/11/22): dropped when sent with others,
// refused alone (retired_permission), never listed.
const EDITOR_FLAGS = ['EMOTES', 'SETS', 'EDITORS', 'PROFILE', 'CHANNELS', 'EMOTE_UPLOAD', 'EMOTE_MANAGE', 'BILLING_PAYMENT', 'BILLING_SUBSCRIPTION'];
export const RETIRED_ONLY = (flags) => flags?.length > 0 && flags.every((f) => f === 'EMOTE_TRANSFER');
export const RETIRED_ERR = () => err('Editors can’t be given the transfer permission any more: only an emote’s owner can give it away.', 'retired_permission');
export const MAX_EDITORS = 50;
export const withImplied = (flags) => EDITOR_FLAGS.filter((f) => flags.includes(f));
export const ACCOUNT_EDITORS = [
  { account: OTHERS[1], permissions: ['EMOTES'], allSets: false, setIds: ['01JD8XSET0000000000000MAIN'], grantedBy: ME_SUMMARY, grantedAt: '2026-11-12T10:00:00.000Z', updatedAt: '2026-11-12T10:00:00.000Z' },
  { account: OTHERS[2], permissions: ['EMOTES', 'SETS', 'EDITORS', 'CHANNELS', 'EMOTE_MANAGE'], allSets: true, setIds: [], grantedBy: ME_SUMMARY, grantedAt: '2026-11-14T18:30:00.000Z', updatedAt: '2026-11-14T18:30:00.000Z' },
];
export const EDITOR_OF = [
  // The emote permissions (ADR 0014): upload as and manage OTHERS[0]'s emotes (never transfer them).
  { account: OTHERS[0], permissions: ['EMOTES', 'SETS', 'CHANNELS', 'EMOTE_UPLOAD', 'EMOTE_MANAGE'], allSets: true, setIds: [], grantedAt: '2026-11-13T09:00:00.000Z' },
  { account: OTHERS[4], permissions: ['EMOTES', 'PROFILE'], allSets: true, setIds: [], grantedAt: '2026-11-15T20:00:00.000Z' },
];

// MOCK_EDITORS=50: the account is at the editor limit.
if (process.env.MOCK_EDITORS === '50') {
  for (let i = ACCOUNT_EDITORS.length; i < MAX_EDITORS; i++) {
    ACCOUNT_EDITORS.push({ account: { id: `01JD8X00000000000000ED${String(i).padStart(4, '0')}`, displayName: `editor_${i}`, avatar: null },
      permissions: ['EMOTES'], allSets: true, setIds: [], grantedBy: ME_SUMMARY, grantedAt: '2026-11-16T10:00:00.000Z', updatedAt: '2026-11-16T10:00:00.000Z' });
  }
}
// Which sets the mock account's grant on an owner covers.
export const coversSet = (ownerId, setId) => {
  if (ownerId === ME_ID) return true;
  const g = EDITOR_OF.find((x) => x.account.id === ownerId);
  return !!g && (g.allSets || g.setIds.includes(setId));
};
// The sets of ME_ID's account an add or change names, or an error.
export const scopeFrom = (setIds) => {
  const ids = [...new Set(setIds ?? [])];
  if (!ids.length) return { allSets: true, setIds: [] };
  if (ids.length > 200) return { error: err('Pick at most 200 sets, or let them work on every set.', 'too_many_sets') };
  if (!ids.every((id) => SETS.some((s) => s.id === id && s.owner?.id === ME_ID))) return { error: err('Pick sets of this account.', 'bad_set') };
  return { allSets: false, setIds: ids };
};
export const SHARED_SETS = [
  { id: '01JD8XSET000000000000OAKAL', name: 'quietoak cozy', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[0],
    emotes: ALL.slice(70, 78).map((e) => member(e)), editors: [], visibility: 'PRIVATE', createdAt: '2026-11-13T09:10:00.000Z' },
  { id: '01JD8XSET000000000000FERN1', name: 'fern garden', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[4],
    emotes: ALL.slice(46, 54).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-15T20:30:00.000Z' },
];
// Public sets of other accounts (accountSets, cloneSet): listed, unlisted and private, so a
// profile shows only the listed ones and a private one 404s to everyone else.
export const PUBLIC_SETS = [
  { id: '01JD8XSET000000000000NORA1', name: 'nora picks', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[1],
    emotes: ALL.slice(5, 23).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-09T15:00:00.000Z' },
  { id: '01JD8XSET000000000000NORA2', name: 'cozy nights', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[1],
    emotes: ALL.slice(30, 38).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-14T22:00:00.000Z' },
  { id: '01JD8XSET000000000000NORA3', name: 'drafts', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[1],
    emotes: ALL.slice(80, 84).map((e) => member(e)), editors: [], visibility: 'PRIVATE', createdAt: '2026-11-16T08:00:00.000Z' },
  { id: '01JD8XSET000000000000NORA4', name: 'nora personal', kind: 'PERSONAL', capacity: 5, owner: OTHERS[1],
    emotes: ALL.slice(90, 93).map((e) => member(e)), editors: [], visibility: 'UNLISTED', createdAt: '2026-11-11T11:00:00.000Z' },
  // Edge cases for /sets/<id>: a listed set with no emotes, and an unlisted one with a very long
  // name and a very long owner name.
  { id: '01JD8XSET000000000000EMPTY', name: 'empty shelf', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[1],
    emotes: [], editors: [], visibility: 'LISTED', createdAt: '2026-11-17T08:00:00.000Z' },
  { id: '01JD8XSET000000000000LONGN', kind: 'CHANNEL', capacity: 1000, visibility: 'UNLISTED', editors: [], createdAt: '2026-11-12T08:00:00.000Z',
    name: 'The complete and unabridged collection of every single emote I have ever collected, sorted by mood, season and how loud chat gets',
    owner: { id: '01JD8X0000000000000000LONG', displayName: 'a_streamer_with_an_extremely_long_display_name_that_never_ends', avatar: `${BASE}/avatars/3.svg` },
    emotes: ALL.slice(8, 52).map((e) => member(e)) },
];
export const EDITOR_CHANNELS = [
  { platform: 'TWITCH', platformChannelId: '55500001', login: 'quietoak', activeSetId: SETS[2].id, allowPersonalEmotes: true, account: OTHERS[0] },
  { platform: 'KICK', platformChannelId: '55500002', login: 'quietoak', activeSetId: null, allowPersonalEmotes: false, account: OTHERS[0] },
  { platform: 'TWITCH', platformChannelId: '55500003', login: 'sunnyfern', activeSetId: SHARED_SETS[1].id, allowPersonalEmotes: true, account: OTHERS[4] },
];
// The sets of the accounts the mock account edits: theirs among SETS (per-set grants) and SHARED_SETS.
export const editorSets = () => [...SETS, ...SHARED_SETS].filter((s) => s.owner && EDITOR_OF.some((g) => g.account.id === s.owner.id) && coversSet(s.owner.id, s.id));
export const editorChannels = () => EDITOR_CHANNELS.filter((c) => EDITOR_OF.some((g) => g.account.id === c.account.id));
export const flagsOn = (ownerId) => (ownerId === ME_ID ? EDITOR_FLAGS : withImplied(EDITOR_OF.find((g) => g.account.id === ownerId)?.permissions ?? []));
export const editorOfOut = () => EDITOR_OF.map((g) => ({ ...g }));

export const PERMISSIONS = ['emotes.review', 'emotes.takedown', 'emotes.edit_any', 'sets.edit_any', 'global_set.edit', 'users.view', 'users.restrict', 'roles.manage', 'audit.view', 'cosmetics.manage', 'cosmetics.grant', 'shop.manage', 'shop.orders', 'codes.manage',
  'roles.assign', 'reports.manage', 'users.restrict_uploads', 'users.restrict_tickets', 'users.block', 'users.sessions', 'users.notes', 'entitlements.view', 'billing.view', 'subscriptions.manage',
  'emotes.delete_any', 'editors.manage_any', 'site.announce', 'review.settings', 'emotes.flags', 'emotes.upload', 'reports.file'];

// myEditableChannels: yours, the accounts you edit, then accounts whose sets you edit (here: none
// beyond those), by role then login.
export function myEditableChannels() {
  const rank = { OWNER: 0, ACCOUNT_EDITOR: 1, SET_EDITOR: 2 };
  const list = [
    ...CHANNELS.map((channel) => ({ role: 'OWNER', channel })),
    ...editorChannels().map((channel) => ({ role: 'ACCOUNT_EDITOR', channel })),
    { role: 'SET_EDITOR', channel: { platform: 'TWITCH', platformChannelId: '55500009', login: 'Nora_byte', activeSetId: null, allowPersonalEmotes: true, account: OTHERS[1] } },
  ];
  return list.sort((a, b) => rank[a.role] - rank[b.role] || a.channel.login.localeCompare(b.channel.login, 'en', { sensitivity: 'base' }) || a.channel.platform.localeCompare(b.channel.platform));
}
