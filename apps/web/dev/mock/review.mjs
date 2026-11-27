// Mock API: review. See ../mock-api.mjs.
import { ME_ID, OTHERS } from './config.mjs';
import { ALL, MINE, NOW, emote } from './emotes.mjs';
import { findEmote } from './sets.mjs';
import { badCursor, pageOf } from './cursor.mjs';

// The review queue: MOCK_PENDING uploads (default 60; try 3000 for a busy day), every 5th unlisted
// and every 7th private, each a few minutes after the last, all mixed across uploaders.
export const PENDING = Array.from({ length: Number(process.env.MOCK_PENDING ?? 60) }, (_, i) => {
  const e = emote(800 + i, OTHERS[i % OTHERS.length], 'PENDING');
  e.createdAt = new Date(NOW - (i + 1) * 7 * 60_000).toISOString();
  // Every upload is unlisted until its listing is approved (ADR 0039); the owner only makes it private.
  if (i % 7 === 3) Object.assign(e, { visibility: 'PRIVATE', flags: { ...e.flags, private: true } });
  else e.visibility = 'UNLISTED';
  // Every 6th is a re-upload of an approved sample (the same picture, a near name), so the detail
  // dialog's "Possible duplicates" has something to show.
  if (i % 6 === 0) {
    const twin = ALL[(i * 7) % ALL.length];
    Object.assign(e, { name: `${twin.name}2`, host: twin.host, files: twin.files, animated: twin.animated });
  }
  return e;
});
// Reviewed this session (the Resolved tab), the latest first, with when.
export const RESOLVED = [];
export const EMOTE_NOTES = { [PENDING[0]?.id]: [{ id: 'ENOTE1', authorId: OTHERS[0].id, body: 'Same uploader sent a near copy last week; compare before approving.', createdAt: new Date(NOW - 3600_000).toISOString() }] };
export const EMOTE_LOG = {};
export const logEmote = (id, action, after = null) => (EMOTE_LOG[id] ??= []).unshift({ id: 'ELOG' + Math.random().toString(36).slice(2, 10), actorId: ME_ID, action, before: null, after: after && JSON.stringify(after), createdAt: new Date().toISOString() });
// Emotes by the SHA-256 of their file: what the mock's uploads were made from. MOCK_SAME_FILE=<sha>
// makes that file "the same" as one public sample emote, to see the uploader's warning at once.
export const FILE_EMOTES = {};
if (process.env.MOCK_SAME_FILE) FILE_EMOTES[process.env.MOCK_SAME_FILE.toLowerCase()] = [ALL[3]];
// The review queue's ready reject reasons (the API seeds the same six).
// The API's words for the two refusals (apps/api/src/upload.rs).
export const NSFW_REFUSAL = 'NSFW emotes aren’t allowed on cyex.tv, so there is no NSFW flag. Emotes that break the guidelines are removed.';
export const UNLISTED_REFUSAL = 'You can’t choose listed or unlisted: every emote is unlisted until a moderator approves its public listing. You can make it private.';
export let REVIEW_REASONS = ['Low quality or blurry', 'Already uploaded (a duplicate)', 'Not an emote', 'NSFW or sexual content (not allowed)', 'Hateful or harassing', 'The name breaks the rules'];
export const setReviewReasons = (v) => { REVIEW_REASONS = v; };
// Trigram similarity, as pg_trgm's similarity(): lower-cased words padded with two spaces in front, one behind.
function trigrams(s) {
  const out = new Set();
  for (const w of s.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean)) {
    const p = `  ${w} `;
    for (let i = 0; i + 3 <= p.length; i++) out.add(p.slice(i, i + 3));
  }
  return out;
}
function similarity(a, b) {
  const x = trigrams(a), y = trigrams(b);
  if (!x.size || !y.size) return 0;
  let both = 0;
  for (const t of x) if (y.has(t)) both++;
  return both / (x.size + y.size - both);
}
// Possible duplicates of an emote, as emoteDuplicates answers: the same file first (in the mock, the
// same uploaded bytes, or a pending sample drawn with an approved emote's picture), then names at
// least half alike.
export function duplicatesOf(id) {
  const me = PENDING.find((e) => e.id === id) ?? RESOLVED.find((e) => e.id === id) ?? findEmote(id);
  if (!me) return null;
  const sameFile = new Set(Object.values(FILE_EMOTES).find((list) => list.some((e) => e.id === id)) ?? []);
  const seen = new Set();
  return [...ALL, ...PENDING, ...RESOLVED, ...MINE]
    .filter((e) => e.id !== id)
    .map((e) => ({ emote: e, sameFile: sameFile.has(e) || Boolean(e.host && e.host === me.host && e.state === 'APPROVED'),
      similarity: e.name.toLowerCase() === me.name.toLowerCase() ? 1 : Math.round(similarity(e.name, me.name) * 100) / 100 }))
    .filter((d) => (d.sameFile || d.similarity >= 0.5) && !seen.has(d.emote.id) && seen.add(d.emote.id))
    .sort((a, b) => Number(b.sameFile) - Number(a.sameFile) || b.similarity - a.similarity)
    .slice(0, 12);
}
/** Approved emotes whose personal use waits for a reviewer (the personal queue, ADR 0039), upload order. */
export const personalPending = () =>
  [...new Map([...MINE, ...ALL, ...RESOLVED, ...PENDING].map((e) => [e.id, e])).values()]
    .filter((e) => e.state === 'APPROVED' && e.approvals?.personalUse === 'PENDING')
    .sort((a, b) => a.createdAt.localeCompare(b.createdAt));
export const batchIds = (ids) => [...new Set((ids ?? []).map((x) => String(x).trim()))];
// When an undone approval sent an upload to the end of the queue (requeueEmotes, ADR 0024): the
// upload-time order reads this before createdAt, as the API reads requeued_at.
export const REQUEUED = new Map();
// When each emote was last reviewed, kept apart from the emote so no answer carries it.
export const REVIEWED_AT = new Map();
export function reviewQueueAnswer(v) {
  const f = v.filter ?? {};
  const tab = f.tab ?? 'PENDING';
  const q = (f.search ?? '').trim().toLowerCase();
  // PUBLIC is pending and not private, LIMITED pending and private; PERSONAL (ADR 0039) lists approved
  // emotes whose personal use is still pending.
  let rows = tab === 'RESOLVED' ? RESOLVED.slice()
    : tab === 'PERSONAL' ? personalPending()
    : PENDING.filter((e) => tab === 'PENDING' || (tab === 'PUBLIC') === (e.visibility !== 'PRIVATE'));
  if (f.personalUse) rows = rows.filter((e) => e.approvals?.personalUse === f.personalUse);
  if (q) rows = rows.filter((e) => e.name.toLowerCase().includes(q) || (e.owner?.displayName ?? '').toLowerCase().includes(q));
  if (f.uploaderId) rows = rows.filter((e) => e.owner?.id === f.uploaderId);
  const resolved = tab === 'RESOLVED';
  const sort = f.sort ?? (resolved ? 'REVIEWED' : 'UPLOADED');
  const desc = f.descending ?? resolved;
  const key = (e) => (sort === 'NAME' ? e.name.toLowerCase() : sort === 'UPLOADER' ? (e.owner?.displayName ?? '').toLowerCase() : sort === 'REVIEWED' ? REVIEWED_AT.get(e.id) ?? '' : (REQUEUED.get(e.id) ?? e.createdAt));
  rows.sort((a, b) => (key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : a.createdAt.localeCompare(b.createdAt)) * (desc && sort !== 'UPLOADER' && sort !== 'NAME' ? -1 : 1));
  if (desc && (sort === 'UPLOADER' || sort === 'NAME')) rows.reverse();
  const p = pageOf(rows, v);
  if (!p) return badCursor();
  const counts = { public: PENDING.filter((e) => e.visibility !== 'PRIVATE').length, limited: PENDING.filter((e) => e.visibility === 'PRIVATE').length, personal: personalPending().length };
  return { data: { reviewQueue: { total: rows.length, counts, items: p.items, next: p.next } } };
}
export const REPORTS = [
  { id: 'R1', reason: 'This is a copy of my emote.', reporterId: OTHERS[1].id, state: 'OPEN', createdAt: '2026-11-16T18:00:00.000Z', emote: ALL[5] },
  { id: 'R2', reason: 'Spam', reporterId: OTHERS[0].id, state: 'OPEN', createdAt: '2026-11-16T19:30:00.000Z', emote: ALL[8] },
  { id: 'R3', reason: 'Looks like the one from a well-known streamer.', reporterId: OTHERS[2].id, state: 'OPEN', createdAt: '2026-11-16T20:10:00.000Z', emote: ALL[5] },
  { id: 'R4', reason: 'Not safe for chat.', reporterId: OTHERS[3].id, state: 'OPEN', createdAt: '2026-11-16T20:40:00.000Z', emote: ALL[5] },
];
// Closed reports, for the account page's lists.
export const CLOSED_REPORTS = [
  { id: 'R0', reason: 'Wrong tag.', reporterId: OTHERS[1].id, state: 'DISMISSED', resolution: 'Fine as is.', createdAt: '2026-11-14T10:00:00.000Z', resolvedAt: '2026-11-14T12:00:00.000Z', emote: ALL[5] },
];

// ---- The two approvals (ADR 0039) -------------------------------------------------------------------
/** Every emote the mock knows, one entry per id. */
export const knownEmote = (id) => PENDING.find((e) => e.id === id) ?? RESOLVED.find((e) => e.id === id) ?? findEmote(id);
const keepVisibility = (e) => {
  // The listing follows the review (the database trigger): unlisted unless private or approved for listing.
  if (e.visibility !== 'PRIVATE') e.visibility = e.approvals.publicListing === 'APPROVED' ? 'PUBLIC' : 'UNLISTED';
  e.flags.private = e.visibility === 'PRIVATE';
};
/** Decides the public listing: the emote can be used either way (`state` approved), listed only on APPROVED. */
export function decideListing(e, to) {
  e.approvals = { ...(e.approvals ?? { personalUse: 'PENDING' }), publicListing: to };
  if (e.state === 'PENDING') {
    e.state = 'APPROVED';
    const at = PENDING.indexOf(e);
    if (at >= 0) PENDING.splice(at, 1);
    REVIEWED_AT.set(e.id, new Date().toISOString());
    if (!RESOLVED.includes(e)) RESOLVED.unshift(e);
  }
  keepVisibility(e);
}
export function decidePersonal(e, to) {
  e.approvals = { ...(e.approvals ?? { publicListing: 'PENDING' }), personalUse: to };
}
/** Refuses an emote outright: rejected, both approvals denied. */
export function rejectOutright(e) {
  const at = PENDING.indexOf(e);
  if (at >= 0) PENDING.splice(at, 1);
  e.state = 'REJECTED';
  e.approvals = { publicListing: 'DENIED', personalUse: 'DENIED' };
  keepVisibility(e);
  REVIEWED_AT.set(e.id, new Date().toISOString());
  if (!RESOLVED.includes(e)) RESOLVED.unshift(e);
}
export const APPROVALS_BAD = 'Pick at least one thing to decide: the public listing or personal use. A denial needs a reason of 1 to 500 characters.';
/**
 * reviewApprovals (apps/api/src/moderation.rs): up to 100 emotes, each part APPROVE or DENY. `onlyPending`
 * (default true) skips a part that is no longer pending; an emote with nothing left to do is `not_pending`.
 * Needs emotes.review for the listing and emotes.review_personal for personal use (the mock admin holds both).
 */
export function reviewApprovals(v, perms, notify) {
  const ids = batchIds(v.ids);
  if (!ids.length || ids.length > 100) return { error: ['Pick 1 to 100 at a time.', 'bad_batch'] };
  const input = v.input ?? {};
  const listing = input.publicListing ?? null;
  const personal = input.personalUse ?? null;
  const reason = (input.reason ?? '').trim();
  if (!listing && !personal) return { error: [APPROVALS_BAD, 'bad_approvals'] };
  if ((listing === 'DENY' || personal === 'DENY') && (!reason || [...reason].length > 500)) return { error: [APPROVALS_BAD, 'bad_approvals'] };
  if (listing && !perms.listing) return { error: ['This needs emotes.review.', 'forbidden'] };
  if (personal && !perms.personal) return { error: ['This needs emotes.review_personal.', 'forbidden'] };
  const onlyPending = input.onlyPending ?? true;
  const done = [], failed = [];
  for (const id of ids) {
    const e = knownEmote(id);
    if (!e) { failed.push({ id, code: 'not_found', message: 'Not found.' }); continue; }
    if (e.state !== 'PENDING' && e.state !== 'APPROVED') { failed.push({ id, code: 'not_pending', message: 'Rejected and taken down emotes can’t be reviewed this way.' }); continue; }
    const a = e.approvals ?? { publicListing: 'PENDING', personalUse: 'PENDING' };
    const doListing = listing && (!onlyPending || a.publicListing === 'PENDING') && a.publicListing !== (listing === 'APPROVE' ? 'APPROVED' : 'DENIED');
    const doPersonal = personal && (!onlyPending || a.personalUse === 'PENDING') && a.personalUse !== (personal === 'APPROVE' ? 'APPROVED' : 'DENIED');
    if (!doListing && !doPersonal) {
      failed.push({ id, code: 'not_pending', message: onlyPending ? 'Nothing is waiting for review on that emote any more.' : 'That emote is already decided that way.' });
      continue;
    }
    const first = doListing && a.publicListing === 'PENDING';
    if (doListing) decideListing(e, listing === 'APPROVE' ? 'APPROVED' : 'DENIED');
    if (doPersonal) decidePersonal(e, personal === 'APPROVE' ? 'APPROVED' : 'DENIED');
    const why = (doListing && listing === 'DENY') || (doPersonal && personal === 'DENY') ? reason : null;
    const after = { state: e.state.toLowerCase(), visibility: e.visibility.toLowerCase(), public_listing: e.approvals.publicListing.toLowerCase(), personal_use: e.approvals.personalUse.toLowerCase(), reason: why };
    logEmote(e.id, first ? 'emote.approved' : 'emote.approvals_changed', after);
    notify?.(e, first ? 'emote.approved' : 'emote.approvals_changed', { emote_id: e.id, name: e.name, visibility: after.visibility, public_listing: after.public_listing, personal_use: after.personal_use, reason: why });
    done.push(id);
  }
  return { done, failed };
}
