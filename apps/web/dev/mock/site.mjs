// Mock API: site. See ../mock-api.mjs.
import { ME_ID, OTHERS } from './config.mjs';
import { DAY, NOW, emote } from './emotes.mjs';
import { statusOf } from './admin.mjs';
import { BADGES, C_OWNED, PAINTS } from './cosmetics.mjs';
import { err, findEmote } from './sets.mjs';

// The site announcement, notification preferences and the public collection (apps/api/src/site.rs).
// MOCK_ANNOUNCE=0 starts without an announcement. As the API: activeAnnouncement is public, the
// list and writes need site.announce, a kind turned off is never added to NOTES, a collection is
// only shown to others when its owner shows it.
const ANNOUNCEMENTS = process.env.MOCK_ANNOUNCE === '0' ? [] : [{
  id: '01JD8XANN000000000000000001', kind: 'CAMPAIGN', text: 'Winter paints are in the shop until the end of the month.', linkUrl: '/shop', linkLabel: 'See them',
  badgeId: null, emoteId: null, startsAt: null, endsAt: new Date(NOW + 20 * DAY).toISOString(), endedAt: null,
  createdAt: new Date(NOW - DAY).toISOString(), updatedAt: new Date(NOW - DAY).toISOString(),
}];
let announceSeq = 1;
const annLive = (a, now = Date.now()) => !a.endedAt && (!a.startsAt || Date.parse(a.startsAt) <= now) && (!a.endsAt || Date.parse(a.endsAt) > now);
const annOut = (a) => {
  const { emoteId, ...rest } = a;
  return { ...rest, emote: emoteId ? (findEmote(emoteId) ?? null) : null, live: annLive(a) };
};
const activeAnnouncement = () => {
  const live = ANNOUNCEMENTS.filter((a) => annLive(a)).sort((a, b) => Date.parse(b.startsAt ?? b.createdAt) - Date.parse(a.startsAt ?? a.createdAt));
  return live[0] ? annOut(live[0]) : null;
};
const NOTE_KINDS = [['emote.approved', false], ['emote.rejected', false], ['emote.approvals_changed', false], ['emote.failed', false], ['emote.taken_down', true], ['emote.visibility_changed', true], ['emote.transfer_requested', true], ['emote.transferred', false],
  ['emote.transfer_declined', false], ['emote.transfer_cancelled', false], ['editor.added', false], ['editor.changed', false], ['editor.removed', false],
  ['cosmetic.granted', false], ['cosmetic.revoked', false], ['webhook.disabled', true]];
const NOTE_PREFS = { off: new Set(), quiet: new Set() };
const prefsOut = () => NOTE_KINDS.map(([kind, required]) => ({ kind, notify: required || !NOTE_PREFS.off.has(kind), toast: !NOTE_PREFS.quiet.has(kind), required }));
export const COLLECTION_PUBLIC = { [OTHERS[0].id]: true, [OTHERS[4].id]: true };
const SOURCES = ['PURCHASE', 'EVENT', 'CODE', 'STAFF'];
function collectionOf(id, session) {
  const own = !!session && id === ME_ID;
  if (!own && (!COLLECTION_PUBLIC[id] || statusOf(id).blocked)) return null;
  const owned = C_OWNED[id] ?? { BADGE: [], PAINT: [] };
  const items = [...owned.PAINT.map((c) => ['PAINT', c]), ...owned.BADGE.map((c) => ['BADGE', c])].map(([kind, cosmeticId], n) => ({
    kind, cosmeticId, name: (kind === 'PAINT' ? PAINTS : BADGES).find((x) => x.id === cosmeticId)?.name ?? '',
    ownedSince: new Date(NOW - (n * 9 + 3) * DAY).toISOString(), source: SOURCES[n % SOURCES.length],
  }));
  return { accountId: id, public: !!COLLECTION_PUBLIC[id], items };
}
function annInput(i, now = Date.now()) {
  // The same checks, in the same order, as apps/api/src/site.rs (fields, then check_refs).
  const text = String(i?.text ?? '').trim();
  if (!text || [...text].length > 200 || /[\u0000-\u001f]/.test(text)) return err('The announcement is one line of 1 to 200 characters.', 'bad_text');
  const link = String(i.linkUrl ?? '').trim() || null;
  const linkOk = (u) => [...u].length <= 500 && !/[\s\u0000-\u001f\u007f]/.test(u)
    && ((u.startsWith('/') && !u.startsWith('//') && !u.startsWith('/\\')) || (u.startsWith('https://') && u.length > 'https://'.length));
  if (link && !linkOk(link)) return err('Links start with https:// or are a path on cyex.tv, like /shop.', 'bad_link');
  const label = String(i.linkLabel ?? '').trim() || null;
  if (label && ([...label].length > 40 || /[\u0000-\u001f]/.test(label))) return err('Link words are up to 40 characters.', 'bad_label');
  if (label && !link) return err('Link words need a link.', 'bad_label');
  const emoteId = String(i.emoteId ?? '').trim() || null;
  const badgeId = String(i.badgeId ?? '').trim() || null;
  const time = (raw, field) => {
    const t = String(raw ?? '').trim();
    if (!t) return null;
    // RFC 3339: a date, a time and a zone.
    return /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?(Z|[+-]\d\d:\d\d)$/i.test(t) && !Number.isNaN(Date.parse(t))
      ? new Date(Date.parse(t)).toISOString()
      : err(`${field} isn’t a date and time.`, 'bad_time');
  };
  const startsAt = time(i.startsAt, 'The start');
  if (startsAt?.errors) return startsAt;
  const endsAt = time(i.endsAt, 'The end');
  if (endsAt?.errors) return endsAt;
  if (endsAt && Date.parse(endsAt) <= now) return err('The end has to be in the future.', 'bad_time');
  if (endsAt && startsAt && Date.parse(endsAt) <= Date.parse(startsAt)) return err('The end has to come after the start.', 'bad_time');
  // The emote must be approved and public, as the API's listed-emote read says.
  const emote = emoteId ? findEmote(emoteId) : null;
  if (emoteId && !(emote && emote.state === 'APPROVED' && emote.visibility === 'PUBLIC')) return err('That emote doesn’t exist or isn’t approved.', 'bad_emote');
  if (badgeId && !BADGES.some((b) => b.id === badgeId)) return err('That badge doesn’t exist.', 'bad_badge');
  return { kind: i.kind ?? 'INFO', text, linkUrl: link, linkLabel: label, emoteId, badgeId, startsAt, endsAt };
}
/** Whether a kind is turned off (the API never creates it). */
export const noteOff = (kind) => NOTE_PREFS.off.has(kind) && !NOTE_KINDS.find(([k]) => k === kind)?.[1];
/** GET /v1/meta and the `meta` query (apps/api/src/meta.rs). */
export const META = {
  api_version: '0.1.0', build: 'dev', limits: { rest: '/v1/limits', graphql_field: 'limits' },
  deprecations: [{
    id: 'ulid_id_input', what: "An account, emote or set ID given as a ULID (26 characters) in an input: a GraphQL argument or variable, a REST path or an upload's owner_id.",
    since: '2026-11-26', sunset: '2027-06-01', replacement: 'Send the Snowflake (a decimal string) that every answer already carries (ADR 0027).',
  }, {
    id: 'file_scale_non_emote', what: 'The `scale` of a badge or paint picture file. It is 1, 2 or 4 for the chat sizes but 7, 14 and 28 for the larger badge sizes (128, 256 and 512 px), which is no scale of anything. Emote files keep `scale` (1 to 4) and are not affected.',
    since: '2026-11-26', sunset: '2027-11-26', replacement: "Read the file's `width` (and `height`): it is the contract for every file. `scale` stays in v1 and is only dropped for badge and paint files in v2.",
  }, {
    id: 'emote_transfer_requests', what: 'Emote transfer requests: the `emoteTransferRequests` query (always empty) and the `acceptEmoteTransfer`, `declineEmoteTransfer` and `cancelEmoteTransfer` mutations (always `410 transfer_requests_retired`). Ownership can’t be requested any more.',
    since: '2026-11-27', sunset: '2027-06-01', replacement: 'None: staff with `emotes.transfer` move an emote directly with `transferEmote` (ADR 0039). Past transfers stay in the audit log.',
  }],
};
const metaAnswer = () => ({ apiVersion: META.api_version, build: META.build, limits: { rest: META.limits.rest, graphqlField: META.limits.graphql_field }, deprecations: META.deprecations });
export function siteAnswer(q, v, session, admin) {
  const field = (name) => new RegExp(`\\b${name}\\b`).test(q);
  const data = {};
  const needAdmin = () => (!session ? err('Sign in to use this.', 'signed_out') : !admin ? err('This needs site.announce.', 'forbidden') : null);
  if (/metas*{/.test(q)) data.meta = metaAnswer();
  if (field('activeAnnouncement')) data.activeAnnouncement = activeAnnouncement();
  if (/\{\s*announcements\s*\{/.test(q)) {
    const no = needAdmin();
    if (no) return no;
    data.announcements = [...ANNOUNCEMENTS].sort((a, b) => Date.parse(b.createdAt) - Date.parse(a.createdAt)).slice(0, 50).map(annOut);
  }
  if (field('createAnnouncement') || field('updateAnnouncement')) {
    const no = needAdmin();
    if (no) return no;
    const f = annInput(v.input);
    if (f.errors) return f;
    const at = new Date().toISOString();
    if (field('createAnnouncement')) {
      const a = { id: `01JD8XANN${String(++announceSeq).padStart(18, '0')}`, ...f, endedAt: null, createdAt: at, updatedAt: at };
      ANNOUNCEMENTS.push(a);
      data.createAnnouncement = annOut(a);
    } else {
      const a = ANNOUNCEMENTS.find((x) => x.id === v.id);
      if (!a) return err('Not found.', 'not_found');
      if (a.endedAt) return err('This announcement has ended. Make a new one.', 'ended');
      Object.assign(a, f, { updatedAt: at });
      data.updateAnnouncement = annOut(a);
    }
  }
  if (field('endAnnouncement')) {
    const no = needAdmin();
    if (no) return no;
    const a = ANNOUNCEMENTS.find((x) => x.id === v.id);
    if (!a) return err('Not found.', 'not_found');
    if (!a.endedAt) Object.assign(a, { endedAt: new Date().toISOString(), updatedAt: new Date().toISOString() });
    data.endAnnouncement = annOut(a);
  }
  if (field('myNotificationPrefs')) {
    if (!session) return err('Sign in to use this.', 'signed_out');
    data.myNotificationPrefs = prefsOut();
  }
  if (field('setNotificationPrefs')) {
    if (!session) return err('Sign in to use this.', 'signed_out');
    if ((v.prefs ?? []).length > NOTE_KINDS.length) return err('Too many kinds at once.', 'too_many');
    const seen = new Set();
    for (const p of v.prefs ?? []) {
      const kind = String(p.kind ?? '').trim();
      const known = NOTE_KINDS.find(([k]) => k === kind);
      if (!known) return err('That isn’t a kind of notification.', 'unknown_kind');
      if (seen.has(kind)) return err('Each kind once.', 'unknown_kind');
      seen.add(kind);
      if (known[1] && !p.notify) return err('Hand-over requests and takedowns always notify you.', 'required_kind');
    }
    for (const p of v.prefs ?? []) {
      const kind = String(p.kind).trim();
      NOTE_PREFS.off[p.notify ? 'delete' : 'add'](kind);
      NOTE_PREFS.quiet[p.toast ? 'delete' : 'add'](kind);
    }
    data.setNotificationPrefs = prefsOut();
  }
  if (field('accountCollection')) data.accountCollection = collectionOf(v.accountId ?? '', session);
  if (field('setCollectionPublic')) {
    if (!session) return err('Sign in to use this.', 'signed_out');
    COLLECTION_PUBLIC[ME_ID] = !!v.public;
    data.setCollectionPublic = collectionOf(ME_ID, session);
  }
  return Object.keys(data).length ? { data } : null;
}
