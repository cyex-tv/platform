// Mock API: signed in/emotes. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY } from '../config.mjs';
import { ALL, MINE } from '../emotes.mjs';
import { SETS } from '../set-data.mjs';
import { EMOTE_LOG, EMOTE_NOTES, NSFW_REFUSAL, PENDING, RESOLVED, UNLISTED_REFUSAL, decideListing, logEmote } from '../review.mjs';
import { NOTES } from '../account.mjs';
import { publicAccount } from '../directory.mjs';
import { anyEmote, emoteGrant, err, findEmote, managed } from '../sets.mjs';
import { badCursor, pageOf } from '../cursor.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
const STAFF_ONLY = 'Only cyex.tv staff can change who owns an emote. Ownership can’t be requested or given away; contact support if an emote should move.';
const REQUESTS_RETIRED = 'Transfer requests are gone: ownership can’t be requested. Only staff can move an emote to another account.';

export function signedInEmotes(op, body, v, ADMIN) {
  switch (op) {
    case 'MyEmotes': {
      const p = pageOf(MINE, v);
      if (!p) return badCursor();
      return { data: { myEmotes: { total: MINE.length, items: p.items, next: p.next } } };
    }
    case 'UpdateEmote': {
      // Staff with emotes.edit_any (the mock admin) change anyone's.
      const e = MINE.find((x) => x.id === v.id) ?? (ADMIN || managed(anyEmote(v.id)) ? anyEmote(v.id) : null);
      if (!e) return err('Only the emote’s owner can change it.', 'forbidden');
      // As the API: NSFW emotes aren't allowed; unlisted is staff's call, both ways (ADR 0020).
      if (v.input.nsfw === true) return err(NSFW_REFUSAL, 'nsfw_not_allowed');
      const vis = v.input.visibility;
      const priv = v.input.private;
      if (vis && priv !== undefined && priv !== null && priv !== (vis === 'PRIVATE'))
        return err('visibility and private say different things.', 'bad_visibility');
      // PUBLIC (or private: false) now means "not private" (ADR 0039): listed only once its listing is approved.
      const listed = e.approvals?.publicListing === 'APPROVED' ? 'PUBLIC' : 'UNLISTED';
      const notPrivate = e.visibility === 'PRIVATE' ? listed : e.visibility;
      let next = vis ?? (priv === true ? 'PRIVATE' : priv === false ? notPrivate : e.visibility);
      if (next === 'PUBLIC' && !ADMIN) next = notPrivate;
      if (next === 'UNLISTED' && e.visibility !== 'UNLISTED' && !ADMIN) return err(UNLISTED_REFUSAL, 'bad_visibility');
      if (next === 'PUBLIC' && ADMIN && e.approvals?.publicListing !== 'APPROVED') decideListing(e, 'APPROVED');
      if (next === 'UNLISTED' && ADMIN && e.approvals?.publicListing === 'APPROVED') decideListing(e, 'DENIED');
      if (v.input.name !== undefined) e.name = v.input.name;
      if (v.input.tags) e.tags = v.input.tags;
      if (v.input.zeroWidth !== undefined && v.input.zeroWidth !== null) e.flags.zeroWidth = v.input.zeroWidth;
      e.visibility = next;
      e.flags.private = e.visibility === 'PRIVATE';
      return { data: { updateEmote: e } };
    }
    case 'DeleteEmote': {
      const i = MINE.findIndex((x) => x.id === v.id);
      // Staff with emotes.delete_any (the mock admin), and editors with EMOTE_MANAGE, delete others'.
      const j = i < 0 && (ADMIN || managed(anyEmote(v.id))) ? ALL.findIndex((x) => x.id === v.id) : -1;
      const k = i < 0 && j < 0 && managed(anyEmote(v.id)) ? PENDING.findIndex((x) => x.id === v.id) : -1;
      if (k >= 0) {
        PENDING.splice(k, 1);
        return { data: { deleteEmote: true } };
      }
      if (i < 0 && j < 0) return err(findEmote(v.id) ? 'Only the emote’s owner can delete it.' : 'Not found.', findEmote(v.id) ? 'forbidden' : 'not_found');
      if (i >= 0) MINE.splice(i, 1);
      else ALL.splice(j, 1);
      for (const s of SETS) s.emotes = s.emotes.filter((m) => m.emote.id !== v.id);
      return { data: { deleteEmote: true } };
    }
    case 'ReportEmote':
      if (!v.reason?.trim()) return err('Give a reason of 1 to 500 characters; the uploader sees it.', 'bad_reason');
      return { data: { reportEmote: { id: 'R' + Date.now() } } };
    // setEditors: the owner, its editors and staff; anyone else is refused (as the API).
    case 'Emote':
      return { data: { emote: findEmote(v.id) ?? PENDING.find((e) => e.id === v.id) ?? RESOLVED.find((e) => e.id === v.id) ?? null } };
    case 'EmoteHistory': {
      const e = PENDING.find((x) => x.id === v.emoteId) ?? RESOLVED.find((x) => x.id === v.emoteId);
      const tagged = e?.tags.length ? [{ id: 'ELOGTG' + e.id.slice(-4), actorId: e.owner?.id ?? null, action: 'emote.updated', before: JSON.stringify({ tags: [] }), after: JSON.stringify({ tags: e.tags }), createdAt: new Date(Date.parse(e.createdAt) + 90_000).toISOString() }] : [];
      return { data: { emoteHistory: [...(EMOTE_LOG[v.emoteId] ?? []), ...tagged] } };
    }
    case 'EmoteNotes':
      return { data: { emoteNotes: EMOTE_NOTES[v.emoteId] ?? [] } };
    case 'AddEmoteNote': {
      const body = (v.body ?? '').trim();
      if (!body || body.length > 2000) return err('Notes are 1 to 2000 characters.', 'bad_note');
      const n = { id: 'ENOTE' + Date.now(), authorId: ME_ID, body, createdAt: new Date().toISOString() };
      (EMOTE_NOTES[v.emoteId] ??= []).unshift(n);
      logEmote(v.emoteId, 'emote.note_added', { body });
      return { data: { addEmoteNote: n } };
    }
    case 'DeleteEmoteNote': {
      for (const [id, list] of Object.entries(EMOTE_NOTES)) {
        const i = list.findIndex((n) => n.id === v.id);
        if (i >= 0) { list.splice(i, 1); logEmote(id, 'emote.note_deleted'); return { data: { deleteEmoteNote: true } }; }
      }
      return { data: { deleteEmoteNote: false } };
    }
    case 'TransferEmote': {
      // Staff only (ADR 0039): the mock admin holds emotes.transfer. Anyone else, the owner included,
      // gets transfer_staff_only; an emote they can't see is not found. No request, no accept step.
      const e = anyEmote(v.id);
      if (!e?.owner) return err('Not found.', 'not_found');
      if (!ADMIN) {
        const sees = e.state === 'APPROVED' || e.owner.id === ME_ID || managed(e);
        return sees ? err(STAFF_ONLY, 'transfer_staff_only') : err('Not found.', 'not_found');
      }
      if (v.toAccountId === e.owner.id) return err('That account owns it already.', 'same_owner');
      const to = publicAccount(v.toAccountId);
      if (!to) return err('Not found.', 'not_found');
      const from = e.owner;
      for (const list of [MINE, ALL]) if (list.includes(e)) list.splice(list.indexOf(e), 1);
      if (e.state !== 'PENDING' || !PENDING.includes(e)) (to.id === ME_ID ? MINE : ALL).unshift(e);
      e.owner = to;
      logEmote(e.id, 'emote.transferred', { from_id: from.id, to_id: to.id, via: 'emotes.transfer' });
      for (const [role, mine] of [['from', from.id === ME_ID], ['to', to.id === ME_ID]])
        if (mine) NOTES.unshift({ id: `01JD8XTRF${role === 'to' ? 'T' : 'F'}${String(Date.now()).padStart(16, '0')}`, kind: 'emote.transferred', read: false, createdAt: new Date().toISOString(),
          payload: JSON.stringify({ role, emote_id: e.id, name: e.name, from_id: from.id, from_name: from.displayName, to_id: to.id, to_name: to.displayName, actor_id: ME_ID, actor_name: ME_SUMMARY.displayName }) });
      return { data: { transferEmote: e } };
    }
    // Retired with transfer requests (ADR 0039): nothing is open, and every answer is 410.
    case 'EmoteTransferRequests':
      return { data: { emoteTransferRequests: [] } };
    case 'AcceptEmoteTransfer':
    case 'DeclineEmoteTransfer':
    case 'CancelEmoteTransfer':
      return err(REQUESTS_RETIRED, 'transfer_requests_retired');
    case 'AccountEmotes': {
      if (!ADMIN && v.accountId !== ME_ID && !emoteGrant(v.accountId, 'EMOTE_MANAGE'))
        return err('This needs users.view or emotes.review, or the account’s emote manage permission.', 'forbidden');
      const all = [...MINE, ...PENDING, ...ALL].filter((e) => e.owner?.id === v.accountId);
      const p = pageOf(all, v);
      if (!p) return badCursor();
      return { data: { accountEmotes: { total: all.length, items: p.items, next: p.next } } };
    }
    default:
      return null;
  }
}
