// Mock API: signed in/review. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY, OWNERS } from '../config.mjs';
import { GLOBAL } from '../set-data.mjs';
import { CLOSED_REPORTS, FILE_EMOTES, NSFW_REFUSAL, PENDING, REPORTS, REQUEUED, RESOLVED, REVIEW_REASONS, batchIds, decideListing, duplicatesOf, logEmote, rejectOutright, reviewApprovals, reviewQueueAnswer, setReviewReasons } from '../review.mjs';
import { NOTES } from '../account.mjs';
import { noteOff } from '../site.mjs';
import { anyEmote, err } from '../sets.mjs';

// approveEmote and reviewEmotes decide the listing only (ADR 0039): no visibility or PUBLIC approves it,
// UNLISTED denies it, PRIVATE makes the emote private and approves it. Personal use waits in its own queue.
function settle(e, visibility) {
  if (visibility === 'PRIVATE') Object.assign(e, { visibility: 'PRIVATE', flags: { ...e.flags, private: true } });
  decideListing(e, visibility === 'UNLISTED' ? 'DENIED' : 'APPROVED');
}
// The owner's notification, when the owner is the mock account (the others have no inbox here).
function tell(e, kind, payload) {
  if (e.owner?.id !== ME_ID || noteOff(kind)) return;
  NOTES.unshift({ id: `01JD8XREV${String(Date.now()).padStart(17, '0')}`, kind, payload: JSON.stringify(payload), createdAt: new Date().toISOString(), read: false });
}

// The signed-in operations of this area, by operation name: null when the operation is not here.
export function signedInReview(op, body, v, ADMIN) {
  switch (op) {
    case 'AdminCounts':
      return { data: { reviewQueue: { total: PENDING.length }, reportQueue: { total: REPORTS.length }, globalSet: { id: GLOBAL.id } } };
    case 'ReviewQueue':
      if (!ADMIN) return err('This needs emotes.review.', 'forbidden');
      return reviewQueueAnswer(v);
    case 'Approve':
    case 'Reject': {
      const at = PENDING.findIndex((x) => x.id === v.id);
      if (at < 0) return err('Only pending emotes can be approved or rejected.', 'not_pending');
      const e = PENDING[at];
      if (op === 'Approve') settle(e, v.visibility);
      else rejectOutright(e);
      logEmote(e.id, op === 'Approve' ? 'emote.approved' : 'emote.rejected', { state: e.state.toLowerCase(), reason: v.reason ?? null, visibility: e.visibility.toLowerCase() });
      return { data: { [op === 'Approve' ? 'approveEmote' : 'rejectEmote']: { id: e.id, state: e.state, visibility: e.visibility } } };
    }
    case 'ReviewEmotes': {
      if (!ADMIN) return err('This needs emotes.review.', 'forbidden');
      const ids = batchIds(v.ids);
      if (!ids.length || ids.length > 100) return err('Pick 1 to 100 at a time.', 'bad_batch');
      const reason = (v.reason ?? '').trim();
      if (v.verdict === 'REJECT' && (!reason || reason.length > 500)) return err('Give a reason of 1 to 500 characters; the uploader sees it.', 'bad_reason');
      if (v.verdict === 'REJECT' && v.visibility) return err('A rejection doesn’t take a visibility.', 'bad_visibility');
      const done = [], failed = [];
      for (const id of ids) {
        const at = PENDING.findIndex((x) => x.id === id);
        if (at < 0) {
          failed.push(RESOLVED.some((x) => x.id === id)
            ? { id, code: 'not_pending', message: 'Only pending emotes can be approved or rejected.' }
            : { id, code: 'not_found', message: 'Not found.' });
          continue;
        }
        const e = PENDING[at];
        if (v.verdict === 'APPROVE') settle(e, v.visibility);
        else rejectOutright(e);
        logEmote(e.id, v.verdict === 'APPROVE' ? 'emote.approved' : 'emote.rejected', { state: e.state.toLowerCase(), reason: reason || null, visibility: e.visibility.toLowerCase() });
        done.push(id);
      }
      return { data: { reviewEmotes: { done, failed } } };
    }
    case 'ReviewApprovals':
    case 'EmoteReviewApprovals': {
      // The mock admin holds emotes.review and emotes.review_personal; anyone else holds neither.
      const out = reviewApprovals(v, { listing: !!ADMIN, personal: !!ADMIN }, tell);
      if (out.error) return err(...out.error);
      return { data: { reviewApprovals: out } };
    }
    case 'RequeueEmotes': {
      if (!ADMIN) return err('This needs emotes.review.', 'forbidden');
      const ids = batchIds(v.ids);
      if (!ids.length || ids.length > 100) return err('Pick 1 to 100 at a time.', 'bad_batch');
      const done = [], failed = [];
      for (const id of ids) {
        if (!PENDING.some((x) => x.id === id)) {
          failed.push(RESOLVED.some((x) => x.id === id) || anyEmote(id)
            ? { id, code: 'not_pending', message: 'Only pending emotes go back in the queue.' }
            : { id, code: 'not_found', message: 'Not found.' });
          continue;
        }
        REQUEUED.set(id, new Date().toISOString());
        logEmote(id, 'emote.requeued', {});
        done.push(id);
      }
      return { data: { requeueEmotes: { done, failed } } };
    }
    case 'SetEmoteFlags': {
      // The mock admin holds emotes.flags (and emotes.edit_any).
      if (!ADMIN) return err('This needs emotes.flags.', 'forbidden');
      const e = anyEmote(v.id) ?? PENDING.find((x) => x.id === v.id) ?? RESOLVED.find((x) => x.id === v.id);
      if (!e) return err('Not found.', 'not_found');
      if (v.input.nsfw === true) return err(NSFW_REFUSAL, 'nsfw_not_allowed');
      const before = { visibility: e.visibility.toLowerCase(), nsfw: e.flags.nsfw, zero_width: e.flags.zeroWidth };
      if (v.input.zeroWidth !== undefined && v.input.zeroWidth !== null) e.flags.zeroWidth = v.input.zeroWidth;
      // Saving clears an old NSFW mark (the API's review of it).
      e.flags.nsfw = false;
      // A visibility change moves the listing approval with it (ADR 0039): PUBLIC approves, UNLISTED denies.
      if (v.input.visibility) {
        e.visibility = v.input.visibility;
        if (e.visibility !== 'PRIVATE') decideListing(e, e.visibility === 'PUBLIC' ? 'APPROVED' : 'DENIED');
      }
      e.flags.private = e.visibility === 'PRIVATE';
      const after = { visibility: e.visibility.toLowerCase(), nsfw: e.flags.nsfw, zero_width: e.flags.zeroWidth };
      if (JSON.stringify(before) !== JSON.stringify(after)) logEmote(e.id, 'emote.flags_changed', { ...after, via: 'emotes.flags' });
      return { data: { setEmoteFlags: e } };
    }
    case 'EmoteDuplicates': {
      if (!ADMIN) return err('This needs emotes.review.', 'forbidden');
      const list = duplicatesOf(v.emoteId);
      if (!list) return err('Not found.', 'not_found');
      return { data: { emoteDuplicates: list } };
    }
    case 'ReviewReasons':
      if (!ADMIN) return err('This needs emotes.review or review.settings.', 'forbidden');
      return { data: { reviewReasons: REVIEW_REASONS } };
    case 'SetReviewReasons': {
      if (!ADMIN) return err('This needs review.settings.', 'forbidden');
      const out = [];
      for (const r of v.reasons ?? []) {
        const t = String(r).trim();
        if (!t || t.length > 200 || /[\u0000-\u001f]/.test(t)) return err('Each reason is 1 to 200 characters on one line.', 'bad_reason');
        if (!out.some((o) => o.toLowerCase() === t.toLowerCase())) out.push(t);
      }
      if (out.length > 12) return err('At most 12 ready reasons.', 'too_many_reasons');
      setReviewReasons(out);
      return { data: { setReviewReasons: out } };
    }
    case 'EmotesWithSameFile': {
      if (!/^[0-9a-fA-F]{64}$/.test(v.sha256 ?? '')) return err('Give a sha256 as 64 hex digits.', 'bad_sha256');
      const list = (FILE_EMOTES[v.sha256.toLowerCase()] ?? []).filter((e) => e.state === 'APPROVED' && e.visibility === 'PUBLIC');
      return { data: { emotesWithSameFile: list.slice(0, 3) } };
    }
    case 'AccountsByLogin': {
      if (!ADMIN) return err('This needs cosmetics.grant, roles.assign, roles.manage or users.view.', 'forbidden');
      const seen = new Set();
      const key = (l) => l.replace(/^@/, '').toLowerCase();
      const logins = (v.logins ?? []).map((l) => String(l).trim()).filter((l) => l && !seen.has(key(l)) && seen.add(key(l)));
      if (!logins.length || logins.length > 100) return err('Give 1 to 100 logins at a time.', 'bad_logins');
      const people = [ME_SUMMARY, ...OWNERS.filter((o) => o.id !== ME_ID)];
      return { data: { accountsByLogin: logins.map((login) => {
        const m = login.match(/^(?:(twitch|kick):)?@?([A-Za-z0-9_-]{1,40})$/i);
        if (!m) return { login, account: null, problem: 'bad_login' };
        const hit = people.find((p) => p.displayName.toLowerCase() === m[2].toLowerCase());
        return hit ? { login, account: hit, problem: null } : { login, account: null, problem: 'not_found' };
      }) } };
    }
    case 'DismissReports': {
      if (!ADMIN) return err('This needs emotes.review, emotes.takedown or reports.manage.', 'forbidden');
      const ids = batchIds(v.ids);
      if (!ids.length || ids.length > 100) return err('Pick 1 to 100 at a time.', 'bad_batch');
      const done = [], failed = [];
      for (const id of ids) {
        const at = REPORTS.findIndex((x) => x.id === id);
        if (at < 0) { failed.push({ id, code: 'not_open', message: 'That report was already resolved.' }); continue; }
        REPORTS.splice(at, 1);
        done.push(id);
      }
      return { data: { dismissReports: { done, failed } } };
    }
    case 'ReportQueue':
      return { data: { reportQueue: { total: REPORTS.length, items: REPORTS } } };
    case 'AccountReports': {
      if (!ADMIN) return err('This needs emotes.review or emotes.takedown.', 'forbidden');
      const all = [...REPORTS, ...CLOSED_REPORTS];
      return { data: { accountReports: { about: all.filter((r) => r.emote?.owner?.id === v.accountId), by: all.filter((r) => r.reporterId === v.accountId) } } };
    }
    case 'ResolveReport': {
      const r = REPORTS.splice(REPORTS.findIndex((x) => x.id === v.id), 1)[0];
      if (v.action === 'TAKE_DOWN') for (let k = REPORTS.length - 1; k >= 0; k--) if (REPORTS[k].emote?.id === r.emote?.id) REPORTS.splice(k, 1);
      return { data: { resolveReport: { id: r.id, state: v.action === 'DISMISS' ? 'DISMISSED' : 'ACTIONED' } } };
    }
    default:
      return null;
  }
}
