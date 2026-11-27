// The review queue's pure parts: what a history entry says, the picture's sizes, and how the
// arrow keys move through a grid of cards. Kept out of the page so they are unit-tested.

export type HistoryEntry = {
  id: string;
  actorId?: string | null;
  action: string;
  before?: string | null;
  after?: string | null;
  createdAt: string;
};

function parse(json: string | null | undefined): Record<string, unknown> {
  if (!json) return {};
  try {
    const v = JSON.parse(json);
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
  } catch {
    return {};
  }
}

const FIELD: Record<string, string> = {
  name: 'name',
  tags: 'tags',
  zero_width: 'zero-width',
  nsfw: 'NSFW',
  private: 'private',
  unlisted: 'unlisted',
};

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

const VERDICT: Record<string, string> = { approved: 'approved', denied: 'denied', pending: 'back to pending' };

/**
 * What an approvals audit row says (`emote.approved` and `emote.approvals_changed`, ADR 0038):
 * "Public listing approved", "Personal use denied: Blurry". Only the parts that changed (a part
 * with no "before" counts as changed); both approved reads as one phrase. Empty when the row has
 * no approvals in it (an older row).
 */
function approvalText(before: Record<string, unknown>, after: Record<string, unknown>): string {
  const listing = typeof after.public_listing === 'string' ? after.public_listing : '';
  const personal = typeof after.personal_use === 'string' ? after.personal_use : '';
  // A part still pending on a first verdict wasn't decided: it isn't mentioned.
  const moved = (was: unknown, now: string) => !!now && was !== now && !(now === 'pending' && was === undefined);
  const l = moved(before.public_listing, listing) ? listing : '';
  const u = moved(before.personal_use, personal) ? personal : '';
  if (!l && !u) return '';
  const reason = typeof after.reason === 'string' && after.reason.trim() ? after.reason.trim() : '';
  const bits: string[] = [];
  if (l === 'approved' && u === 'approved') bits.push('Approved for public listing and personal use');
  else {
    if (l) bits.push(`Public listing ${VERDICT[l] ?? l}`);
    if (u) bits.push(`${bits.length ? 'personal' : 'Personal'} use ${VERDICT[u] ?? u}`);
  }
  const text = bits.join(', ');
  return reason && (l === 'denied' || u === 'denied') ? `${text}: ${reason}` : text;
}

/** One history line: what happened, and an optional detail (a reason, a note, what changed). */
export function describe(e: HistoryEntry): { text: string; detail?: string } {
  const before = parse(e.before);
  const after = parse(e.after);
  const vis = typeof after.visibility === 'string' ? after.visibility : '';
  const reason = typeof after.reason === 'string' ? after.reason : undefined;
  switch (e.action) {
    case 'emote.approved': {
      // ADR 0038: the first listing verdict, with the personal-use state it was decided with.
      const text = approvalText(before, after);
      if (text) return { text };
      return { text: vis && vis !== 'public' ? `Approved as ${vis}` : 'Approved' };
    }
    case 'emote.approvals_changed':
      return { text: approvalText(before, after) || 'Changed the approvals' };
    case 'emote.transferred':
      return { text: after.via === 'emotes.transfer' ? 'Transferred to another account by staff' : 'Transferred to another account' };
    case 'emote.rejected':
      return { text: 'Rejected', detail: reason };
    case 'emote.resubmitted':
      return { text: 'Sent back to review after its owner changed it' };
    case 'emote.taken_down':
      return { text: 'Taken down', detail: reason };
    case 'emote.deleted':
      return { text: 'Deleted' };
    case 'emote.note_added':
      return { text: 'Added a note', detail: typeof after.body === 'string' ? after.body : undefined };
    case 'emote.note_deleted':
      return { text: 'Deleted a note', detail: typeof before.body === 'string' ? before.body : undefined };
    case 'emote.requeued':
      return { text: 'Approval undone: back at the end of the queue' };
    case 'emote.flags_changed': {
      const was = typeof before.visibility === 'string' ? before.visibility : '';
      const bits = [
        vis && vis !== was ? `made it ${vis}` : '',
        'nsfw' in before && 'nsfw' in after && !same(before.nsfw, after.nsfw) ? (after.nsfw ? 'marked it NSFW' : 'took NSFW off') : '',
        'zero_width' in before && 'zero_width' in after && !same(before.zero_width, after.zero_width) ? (after.zero_width ? 'made it zero-width' : 'took zero-width off') : '',
      ].filter(Boolean);
      return { text: bits.length ? `Flags: ${bits.join(', ')}` : 'Changed the flags' };
    }
    case 'emote.updated': {
      const changed = Object.keys(FIELD).filter((k) => k in after && !same(before[k], after[k]));
      if (changed.length === 1 && changed[0] === 'tags') {
        const tags = Array.isArray(after.tags) ? after.tags.join(', ') : '';
        return { text: tags ? 'Set the tags' : 'Removed the tags', detail: tags || undefined };
      }
      if (changed.length === 1 && changed[0] === 'name') return { text: `Renamed to ${String(after.name)}` };
      return { text: changed.length ? `Changed ${changed.map((k) => FIELD[k]).join(', ')}` : 'Edited' };
    }
    default:
      return { text: e.action.replace(/^emote\./, '').replace(/_/g, ' ') };
  }
}

export type FileLike = { name: string; format: string; scale: number; width: number; height: number; size: number; isStatic: boolean };

/** The moving (or only) WebP at each scale, smallest first: what chat shows. */
export function sizes(files: FileLike[]): FileLike[] {
  return files.filter((f) => f.format === 'WEBP' && !f.isStatic).sort((a, b) => a.scale - b.scale);
}

/** What kind of picture: `Animated WebP` or `WebP`. */
export function kindOf(e: { animated: boolean; files: FileLike[] }): string {
  const fmt = sizes(e.files)[0]?.format ?? 'WEBP';
  const name = fmt === 'WEBP' ? 'WebP' : fmt === 'AVIF' ? 'AVIF' : fmt;
  return e.animated ? `Animated ${name}` : name;
}

/**
 * Where an arrow key goes in a grid of `n` cards laid out `cols` wide, from `at`. Left and right
 * step one (and wrap rows); up and down step a row and stop at the edges; Home and End jump.
 * Null for any other key.
 */
export function step(key: string, at: number, n: number, cols: number): number | null {
  if (n <= 0) return null;
  const c = Math.max(1, cols);
  const clamp = (i: number) => Math.min(n - 1, Math.max(0, i));
  switch (key) {
    case 'ArrowRight':
      return clamp(at + 1);
    case 'ArrowLeft':
      return clamp(at - 1);
    case 'ArrowDown':
      return at + c < n ? at + c : at;
    case 'ArrowUp':
      return at - c >= 0 ? at - c : at;
    case 'Home':
      return 0;
    case 'End':
      return n - 1;
    default:
      return null;
  }
}

/** After the card at `at` leaves a list of `n` (before it left): the one to show next, or null. */
export function after(at: number, n: number): number | null {
  if (n <= 1) return null;
  return Math.min(at, n - 2);
}

/**
 * What a failed approval says. It names the upload, and a call that ran out of time says it may have
 * gone through (the queue is reloaded, so the card shows where it stands).
 */
export function approveFailure(err: unknown, name: string): string {
  const code = (err as { code?: unknown } | null)?.code;
  if (code === 'timeout') return `cyex.tv didn’t answer in time. ${name} may have been approved: check the queue before trying again.`;
  const why = err instanceof Error && err.message ? err.message : 'That didn’t work.';
  return `${name} wasn’t approved. ${why}`;
}

// What a reviewer can decide, and what each button or key sends (ADR 0038). Two questions per
// upload: may it be listed publicly, and may it be a personal emote. "Approve both" is the common
// case; a reviewer who holds only one of the two permissions gets the one they can do.

/** The two review permissions the signed-in reviewer holds (`emotes.review`, `emotes.review_personal`). */
export type Reviewer = { listing: boolean; personal: boolean };
/** The queue the actions are for: the main one (listing first), or the personal-use one. */
export type Mode = 'main' | 'personal';
export type Action = 'both' | 'listing' | 'personal' | 'deny-listing' | 'deny-personal' | 'reject';
export type Verdict = 'APPROVE' | 'DENY';
/** What `reviewApprovals` is told: each part left out stays as it is. */
export type Decision = { listing?: Verdict; personal?: Verdict };

/** Which actions the reviewer has in a queue, primary first. Empty when they can decide nothing there. */
export function actionsFor(mode: Mode, r: Reviewer): Action[] {
  if (mode === 'personal') return r.personal ? ['personal', 'deny-personal'] : [];
  if (!r.listing) return [];
  return r.personal ? ['both', 'listing', 'personal', 'deny-listing', 'deny-personal', 'reject'] : ['listing', 'deny-listing', 'reject'];
}

/** The name on a button. Without the personal permission "Approve" is the listing alone. */
export function actionLabel(a: Action, r: Reviewer, mode: Mode = 'main'): string {
  switch (a) {
    case 'both':
      return 'Approve both';
    case 'listing':
      return r.personal ? 'Approve public listing' : 'Approve';
    case 'personal':
      return mode === 'personal' ? 'Approve personal use' : 'Approve personal use only';
    case 'deny-listing':
      return 'Deny public listing';
    case 'deny-personal':
      return 'Deny personal use';
    case 'reject':
      return 'Reject';
  }
}

/** What an action decides; null for Reject (the emote is refused outright, with its own call). */
export function decisionOf(a: Action): Decision | null {
  switch (a) {
    case 'both':
      return { listing: 'APPROVE', personal: 'APPROVE' };
    case 'listing':
      return { listing: 'APPROVE' };
    case 'personal':
      return { personal: 'APPROVE' };
    case 'deny-listing':
      return { listing: 'DENY' };
    case 'deny-personal':
      return { personal: 'DENY' };
    case 'reject':
      return null;
  }
}

/** A denial and a rejection need a reason the owner sees; an approval is sent after its undo time. */
export const needsReason = (a: Action) => a === 'deny-listing' || a === 'deny-personal' || a === 'reject';

/** The `ApprovalsInput` for a decision. `onlyPending` stays at the API's default (true) in queues. */
export function approvalsInput(d: Decision, reason?: string) {
  return {
    ...(d.listing ? { publicListing: d.listing } : {}),
    ...(d.personal ? { personalUse: d.personal } : {}),
    ...(reason ? { reason } : {}),
  };
}

/** The action a key stands for in a queue (A, L, P, D, N, R); null for any other, or one the reviewer lacks. */
export function actionForKey(key: string, mode: Mode, r: Reviewer): Action | null {
  const k = key.toLowerCase();
  let want: Action | null = null;
  if (mode === 'personal') {
    if (k === 'a' || k === 'p') want = 'personal';
    else if (k === 'd' || k === 'n') want = 'deny-personal';
  } else if (k === 'a') want = r.personal ? 'both' : 'listing';
  else if (k === 'l') want = 'listing';
  else if (k === 'p') want = 'personal';
  else if (k === 'd') want = 'deny-listing';
  else if (k === 'n') want = 'deny-personal';
  else if (k === 'r') want = 'reject';
  return want && actionsFor(mode, r).includes(want) ? want : null;
}

/** What a toast says after the action, in the past tense: "Wave approved for public listing and personal use." */
export function actionDone(a: Action): string {
  switch (a) {
    case 'both':
      return 'approved for public listing and personal use';
    case 'listing':
      return 'approved for public listing';
    case 'personal':
      return 'approved for personal use';
    case 'deny-listing':
      return 'denied public listing';
    case 'deny-personal':
      return 'denied personal use';
    case 'reject':
      return 'rejected';
  }
}

export type ApprovalState = 'PENDING' | 'APPROVED' | 'DENIED';
export type Approvals = { publicListing: ApprovalState; personalUse: ApprovalState };

/** Small chips for a card: the approvals that are decided (a pending part says nothing in its own queue). */
export function approvalChips(e: { approvals?: Approvals | null }, resolved: boolean): { text: string; tone: 'ok' | 'warn' }[] {
  const a = e.approvals;
  if (!a) return [];
  const out: { text: string; tone: 'ok' | 'warn' }[] = [];
  if (resolved && a.publicListing === 'APPROVED') out.push({ text: 'Listed', tone: 'ok' });
  else if (resolved && a.publicListing === 'DENIED') out.push({ text: 'Not listed', tone: 'warn' });
  if (a.personalUse === 'APPROVED') out.push({ text: 'Personal', tone: 'ok' });
  else if (a.personalUse === 'DENIED') out.push({ text: 'No personal', tone: 'warn' });
  return out;
}

/** How an approval state reads on a fact line. */
export const APPROVAL_LABEL: Record<string, string> = { PENDING: 'Waiting', APPROVED: 'Approved', DENIED: 'Denied' };

/** An approval waiting out its undo time: what it takes out of the tab and the counts. */
type Held = {
  id: string;
  /** Private uploads count in the "unlisted and private" tab. */
  private: boolean;
  /** It leaves the main queue (a listing verdict). */
  main: boolean;
  /** It leaves the personal queue (a personal verdict on an upload whose personal use was pending). */
  personal: boolean;
};
type Counts = { public: number; limited: number; personal: number };
type QueueView<T extends { id: string }> = { items: T[]; total: number; counts: Counts };

/**
 * The queue as the page shows it while approvals wait out their undo time: the server still has
 * those uploads pending (nothing was sent yet), so every load brings them back. They are left out
 * of the cards when the verdict takes them out of the tab shown, and taken off the tab counts
 * and the total. `tab` is the API's (PENDING, PUBLIC, LIMITED, PERSONAL or RESOLVED).
 */
export function withheld<T extends { id: string }>(data: QueueView<T> | null | undefined, held: readonly Held[], tab: string): QueueView<T> {
  if (!data) return { items: [], total: 0, counts: { public: 0, limited: 0, personal: 0 } };
  if (!held.length || tab === 'RESOLVED') return data;
  const leaves = (h: Held) => (tab === 'PERSONAL' ? h.personal : h.main && (tab === 'PUBLIC' ? !h.private : tab === 'LIMITED' ? h.private : true));
  const out = new Set(held.filter(leaves).map((h) => h.id));
  const main = held.filter((h) => h.main);
  const pub = main.filter((h) => !h.private).length;
  return {
    items: data.items.filter((e) => !out.has(e.id)),
    total: Math.max(0, data.total - out.size),
    counts: {
      public: Math.max(0, data.counts.public - pub),
      limited: Math.max(0, data.counts.limited - (main.length - pub)),
      personal: Math.max(0, data.counts.personal - held.filter((h) => h.personal).length),
    },
  };
}

/**
 * The loaded queue with `ids` moved to the end, in the order given: an approval taken back within
 * its undo time puts the upload back last, not where it was (ADR 0024). The server is told too
 * (`requeueEmotes`), so the next load and every other reviewer see the same order.
 */
export function toEnd<T extends { id: string }, Q extends { items: T[] }>(data: Q, ids: readonly string[]): Q {
  const at = new Map(data.items.map((e) => [e.id, e]));
  const moved = ids.map((id) => at.get(id)).filter((e): e is T => !!e);
  if (!moved.length) return data;
  const out = new Set(moved.map((e) => e.id));
  return { ...data, items: [...data.items.filter((e) => !out.has(e.id)), ...moved] };
}

/** Which queues an action takes an upload out of: the main one (a listing verdict or a rejection) and the personal one. */
export function partsOf(a: Action): { main: boolean; personal: boolean } {
  const d = decisionOf(a);
  return d ? { main: !!d.listing, personal: !!d.personal } : { main: true, personal: true };
}

const STATE: Record<Verdict, ApprovalState> = { APPROVE: 'APPROVED', DENY: 'DENIED' };

/** An upload's approvals after a decision (null: rejected, both denied); what the cards show before the next load. */
export function settle<T extends Approvals | null | undefined>(a: T, d: Decision | null): T | Approvals {
  if (!a) return a;
  if (!d) return { publicListing: 'DENIED', personalUse: 'DENIED' };
  return { publicListing: d.listing ? STATE[d.listing] : a.publicListing, personalUse: d.personal ? STATE[d.personal] : a.personalUse };
}

/** The account page's filter on personal use (the rows already loaded). */
export type PersonalFilter = 'all' | 'APPROVED' | 'PENDING' | 'DENIED';
export function byPersonal<T extends { approvals?: Approvals | null }>(rows: readonly T[], f: PersonalFilter): T[] {
  return f === 'all' ? [...rows] : rows.filter((e) => e.approvals?.personalUse === f);
}
