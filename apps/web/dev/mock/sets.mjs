// Mock API: sets. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY } from './config.mjs';
import { ALL, MINE } from './emotes.mjs';
import { CHANNELS } from './account.mjs';
import { SETS } from './set-data.mjs';
import { PENDING } from './review.mjs';
import { limitOf } from './roles.mjs';
import { statusOf } from './admin.mjs';
import { EDITOR_CHANNELS, EDITOR_OF, PUBLIC_SETS, SHARED_SETS, coversSet, flagsOn } from './editors.mjs';

// Editors of other accounts, as staff with editors.manage_any manage them (the mock admin).
export const STAFF_EDITORS = {};
export const findEmote = (id) => MINE.find((e) => e.id === id) ?? ALL.find((e) => e.id === id);
export const findSet = (id) => SETS.find((s) => s.id === id) ?? SHARED_SETS.find((s) => s.id === id) ?? PUBLIC_SETS.find((s) => s.id === id);
export const ALL_SETS = () => [...SETS, ...SHARED_SETS, ...PUBLIC_SETS];
// Every set is public (ADR 0017, amendment 2026/11/22): the migration listed every unlisted and
// private set. Only staff can take one off the listings now.
for (const s of ALL_SETS()) s.visibility = 'LISTED';
export const PUBLIC_ONLY = () => err('Every set is public. Only staff can take a set off the listings.', 'bad_visibility');
// A personal set's slots: the staff override, else its owner's personal_set_slots role limit
// (3, or 10 for a subscriber role; ADR 0038).
export const personalSlots = (s) => s.capacityOverride ?? limitOf(s.owner?.id ?? ME_ID, 'personal_set_slots');
// setCapacityRange: from what the set holds (at least 1) to the owner's set_capacity_max role
// limit (never below its capacity now), 1.000.000 for staff; null when you can't change it. A
// personal set's owner can't change its slots: only staff, as an override.
export const capRange = (s, admin) => {
  if (!s || !mayView(s, true) || !setRights(s, admin).rename) return null;
  if (s.kind === 'PERSONAL' && !admin) return null;
  const used = s.emotes.length;
  const ownerMax = s.kind === 'PERSONAL' ? s.capacity : Math.max(limitOf(s.owner?.id ?? ME_ID, 'set_capacity_max'), s.capacity);
  return { min: Math.max(1, used), max: admin ? (s.kind === 'PERSONAL' ? Math.max(1000, ownerMax) : 1_000_000) : ownerMax, ownerMax, used, staff: !!admin };
};
const ACTIVE_SET_IDS = () => new Set([...CHANNELS, ...EDITOR_CHANNELS].map((c) => c.activeSetId).filter(Boolean));
// The API's rule: anyone opens listed and unlisted sets; a private one only its owner, editors and
// account editors, or anyone while a channel shows it.
export const mayView = (s, session) => s.visibility !== 'PRIVATE' || ACTIVE_SET_IDS().has(s.id)
  || (session && (s.owner?.id === ME_ID || s.editors.some((e) => e.id === ME_ID) || EDITOR_OF.some((g) => g.account.id === s.owner?.id)));
// What the mock account may do with a set, as the API checks it: the owner and staff everything;
// an account editor with Sets everything on the sets the grant covers, with Emotes the emotes; a
// per-set editor the emotes and the name. Anyone else nothing (the API answers forbidden).
export const setRights = (s, ADMIN) => {
  const none = { emotes: false, rename: false, manage: false };
  if (!s) return none;
  if (ADMIN || s.owner?.id === ME_ID) return { emotes: true, rename: true, manage: true };
  const owner = s.owner?.id;
  const flags = owner && coversSet(owner, s.id) ? flagsOn(owner) : [];
  const manage = flags.includes('SETS');
  const rename = manage || s.editors.some((e) => e.id === ME_ID);
  return { manage, rename, emotes: rename || flags.includes('EMOTES') };
};
export const forbidden = () => err('Only the set’s owner and editors can change it.', 'forbidden');
export const err = (message, code) => ({ data: null, errors: [{ message, extensions: { code } }] });
// The API's rule for the emote permissions: held on a grant that covers every set.
export const emoteGrant = (ownerId, flag) => EDITOR_OF.some((g) => g.account.id === ownerId && g.allSets && g.permissions.includes(flag));
export const managed = (e) => !!e && !!e.owner && e.owner.id !== ME_ID && emoteGrant(e.owner.id, 'EMOTE_MANAGE');
export const anyEmote = (id) => findEmote(id) ?? PENDING.find((e) => e.id === id);
export const setOut = (s) => ({ ...s, editors: undefined, visibility: s.visibility ?? 'LISTED', listed: (s.visibility ?? 'LISTED') === 'LISTED',
  slotsUsed: s.emotes.length, capacityOverride: s.capacityOverride ?? null, createdAt: s.createdAt ?? '2026-11-10T12:00:00.000Z', version: s.version ?? 0 });
const setCard = (s, preview = 12) => {
  const o = setOut(s);
  return { id: o.id, name: o.name, kind: o.kind, owner: o.owner, visibility: o.visibility, listed: o.listed, capacity: o.capacity,
    slotsUsed: o.slotsUsed, active: ACTIVE_SET_IDS().has(o.id), createdAt: o.createdAt,
    preview: [...o.emotes].reverse().slice(0, Math.max(0, Math.min(50, preview))) };
};
// accountSets(accountId, preview): an account's listed sets as cards, signed in or not.
export const accountSets = (accountId, preview) =>
  ALL_SETS().filter((s) => s.owner?.id === accountId && s.kind !== 'GLOBAL' && (s.visibility ?? 'LISTED') === 'LISTED'
    && !(statusOf(accountId).blocked)).slice(0, 50).map((s) => setCard(s, preview ?? 12));
const VISIBILITIES = ['LISTED', 'UNLISTED', 'PRIVATE'];
let clones = 0;
// cloneSet(id, name, ownerId, visibility): the API's rules in small: approved emotes only,
// others' private emotes skipped, capacity 1000.
export function cloneSet(v, session, admin = false) {
  const src = findSet(v.id);
  if (!src || !mayView(src, session)) return err('Not found.', 'not_found');
  const ownerId = v.ownerId || ME_ID;
  if (!flagsOn(ownerId).includes('SETS'))
    return err('Only the account’s owner and its editors with the sets permission can make sets for it.', 'forbidden');
  if (++clones > 30) return err('That’s a lot of copied sets. Try again in a while.', 'rate_limited');
  const name = (v.name ?? src.name).trim();
  if (!name || name.length > 100) return err('Set names are 1 to 100 characters.', 'bad_name');
  if (v.visibility && !VISIBILITIES.includes(v.visibility)) return err('Unknown visibility.', 'bad_visibility');
  if (v.visibility && v.visibility !== 'LISTED' && !admin) return PUBLIC_ONLY();
  const owner = ownerId === ME_ID ? ME_SUMMARY : EDITOR_OF.find((g) => g.account.id === ownerId).account;
  const keep = src.emotes.filter((m) => m.emote.state === 'APPROVED' && (!m.emote.flags.private || m.emote.owner?.id === ownerId));
  const copied = keep.slice(0, 1000);
  const s = { id: `01JD8XSETCPY${String(clones).padStart(14, '0')}`, name, kind: 'CHANNEL', capacity: 1000, owner,
    emotes: copied.map((m) => ({ ...m, addedAt: new Date().toISOString() })), editors: [], visibility: v.visibility ?? 'LISTED',
    createdAt: new Date().toISOString() };
  (ownerId === ME_ID ? SETS : SHARED_SETS).push(s);
  return { data: { cloneSet: { set: setOut(s), copied: copied.length, skipped: src.emotes.length - copied.length } } };
}
