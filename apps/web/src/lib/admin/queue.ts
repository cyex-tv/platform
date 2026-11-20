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

/** One history line: what happened, and an optional detail (a reason, a note, what changed). */
export function describe(e: HistoryEntry): { text: string; detail?: string } {
  const before = parse(e.before);
  const after = parse(e.after);
  const vis = typeof after.visibility === 'string' ? after.visibility : '';
  const reason = typeof after.reason === 'string' ? after.reason : undefined;
  switch (e.action) {
    case 'emote.approved':
      return { text: vis && vis !== 'public' ? `Approved as ${vis}` : 'Approved' };
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

type Held = { id: string; visibility: string };
type QueueView<T extends Held> = { items: T[]; total: number; counts: { public: number; limited: number } };

/**
 * The queue as the page shows it while approvals wait out their undo time: the server still has
 * those uploads pending (nothing was sent yet), so every load brings them back. They are left out
 * of the cards, and taken off the tab counts (by the visibility they had) and off the total when
 * they belong to the tab shown. `tab` is the API's (PENDING, PUBLIC, LIMITED or RESOLVED).
 */
export function withheld<T extends Held>(data: QueueView<T> | null | undefined, held: readonly Held[], tab: string): QueueView<T> {
  if (!data) return { items: [], total: 0, counts: { public: 0, limited: 0 } };
  if (!held.length || tab === 'RESOLVED') return data;
  const ids = new Set(held.map((e) => e.id));
  const pub = held.filter((e) => e.visibility === 'PUBLIC').length;
  const lim = held.length - pub;
  const inTab = tab === 'PUBLIC' ? pub : tab === 'LIMITED' ? lim : held.length;
  return {
    items: data.items.filter((e) => !ids.has(e.id)),
    total: Math.max(0, data.total - inTab),
    counts: { public: Math.max(0, data.counts.public - pub), limited: Math.max(0, data.counts.limited - lim) },
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
