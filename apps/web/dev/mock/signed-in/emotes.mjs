// Mock API: signed in/emotes. See ../mock-api.mjs.
import { ME_ID } from '../config.mjs';
import { ALL, MINE } from '../emotes.mjs';
import { SETS } from '../set-data.mjs';
import { EMOTE_LOG, EMOTE_NOTES, NSFW_REFUSAL, PENDING, RESOLVED, UNLISTED_REFUSAL, logEmote } from '../review.mjs';
import { EDITOR_OF } from '../editors.mjs';
import { anyEmote, emoteGrant, err, findEmote, managed } from '../sets.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
export function signedInEmotes(op, body, v, ADMIN) {
  switch (op) {
    case 'MyEmotes': {
      const per = v.perPage ?? 50;
      const page = v.page ?? 1;
      return { data: { myEmotes: { total: MINE.length, items: MINE.slice((page - 1) * per, page * per) } } };
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
      const next = vis ?? (priv === true ? 'PRIVATE' : priv === false && e.visibility === 'PRIVATE' ? 'PUBLIC' : e.visibility);
      if (next !== e.visibility && (next === 'UNLISTED' || e.visibility === 'UNLISTED') && !ADMIN)
        return err(e.visibility === 'UNLISTED' ? 'A moderator took this emote off the listings. Only staff can change that.' : UNLISTED_REFUSAL, 'bad_visibility');
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
      return { data: { emote: findEmote(v.id) ?? PENDING.find((e) => e.id === v.id) ?? null } };
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
      // The owner only (ADR 0014, amendment 2026/11/22); the receiver is an account where you
      // hold EMOTE_UPLOAD on every set.
      const e = anyEmote(v.id);
      if (!e?.owner) return err('Not found.', 'not_found');
      if (e.owner.id !== ME_ID) return err('Only an emote’s owner can give it away. Nobody can ask for someone else’s emote to be transferred.', 'forbidden');
      if (v.toAccountId === e.owner.id) return err('That account owns it already.', 'same_owner');
      if (!emoteGrant(v.toAccountId, 'EMOTE_UPLOAD'))
        return err('It can go to the one giving it, or to an account they edit with the upload permission on every set.', 'forbidden');
      const to = EDITOR_OF.find((g) => g.account.id === v.toAccountId).account;
      MINE.splice(MINE.indexOf(e), 1);
      ALL.unshift(e);
      e.owner = to;
      return { data: { transferEmote: e } };
    }
    // Retired with editor transfer requests: nothing is open, nothing can be accepted.
    case 'EmoteTransferRequests':
      return { data: { emoteTransferRequests: [] } };
    case 'AcceptEmoteTransfer':
      return err('Only an emote’s owner can give it away. Nobody can ask for someone else’s emote to be transferred.', 'forbidden');
    case 'DeclineEmoteTransfer':
    case 'CancelEmoteTransfer':
      return v.id === '01JD8XTRANSFER000000000001' ? err('That transfer request isn’t open any more.', 'not_pending') : err('Not found.', 'not_found');
    case 'AccountEmotes': {
      if (!ADMIN && v.accountId !== ME_ID && !emoteGrant(v.accountId, 'EMOTE_MANAGE'))
        return err('This needs users.view or emotes.review, or the account’s emote manage permission.', 'forbidden');
      const all = [...MINE, ...PENDING, ...ALL].filter((e) => e.owner?.id === v.accountId);
      const per = v.perPage ?? 50;
      const from = ((v.page ?? 1) - 1) * per;
      return { data: { accountEmotes: { total: all.length, items: all.slice(from, from + per) } } };
    }
    default:
      return null;
  }
}
