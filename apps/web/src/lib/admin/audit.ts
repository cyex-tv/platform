// Reading the audit log: what an entry did in words, where its target lives, and what changed.

export type Change = { path: string; before?: string; after?: string; kind: 'added' | 'removed' | 'changed' };

/** `role.updated` or `emotes.approve` as "Role updated" / "Emotes approve". */
export function actionText(action: string): string {
  const words = action.replace(/[._]/g, ' ').trim();
  return words ? words[0].toUpperCase() + words.slice(1) : action;
}

/** Where a target can be looked at, if it has a page in the admin or the app. */
export function targetHref(kind: string, id: string): string | null {
  const q = encodeURIComponent(id);
  switch (kind) {
    case 'account':
      return `/admin/users/${q}`;
    case 'emote':
      return `/me/emotes/${q}`;
    case 'set':
      return `/me/sets/${q}`;
    case 'role':
      return '/admin/roles';
    case 'badge':
      return '/admin/cosmetics/badges';
    case 'paint':
      return '/admin/cosmetics/paints';
    default:
      return null;
  }
}

function parse(json: string | null | undefined): unknown {
  if (!json) return undefined;
  try {
    return JSON.parse(json);
  } catch {
    return json;
  }
}

const show = (v: unknown) => (typeof v === 'string' ? v : JSON.stringify(v));

function flatten(value: unknown, path: string, out: Map<string, string>) {
  if (value !== null && typeof value === 'object' && !Array.isArray(value) && Object.keys(value).length) {
    for (const [k, v] of Object.entries(value)) flatten(v, path ? `${path}.${k}` : k, out);
  } else if (value !== undefined) {
    out.set(path || '(value)', show(value));
  }
}

/** The fields that differ between an entry's `before` and `after` JSON, one row per leaf. */
export function changes(before: string | null | undefined, after: string | null | undefined): Change[] {
  const a = new Map<string, string>();
  const b = new Map<string, string>();
  flatten(parse(before), '', a);
  flatten(parse(after), '', b);
  const out: Change[] = [];
  for (const [path, was] of a) {
    if (!b.has(path)) out.push({ path, before: was, kind: 'removed' });
    else if (b.get(path) !== was) out.push({ path, before: was, after: b.get(path), kind: 'changed' });
  }
  for (const [path, now] of b) if (!a.has(path)) out.push({ path, after: now, kind: 'added' });
  return out.sort((x, y) => x.path.localeCompare(y.path));
}

/** A `YYYY-MM-DD` day as a local date, or null when it isn't a real one (02-30, month 13, year 0). Years 0 to 99 stay themselves. */
function localDay(day: string, plus = 0): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const at = new Date(2000, 0, 1);
  at.setFullYear(y, mo - 1, d); // not the constructor: it reads years 0 to 99 as 19xx
  if (y < 1 || at.getFullYear() !== y || at.getMonth() !== mo - 1 || at.getDate() !== d) return null;
  if (plus) at.setDate(at.getDate() + plus);
  at.setHours(0, 0, 0, 0);
  return at;
}

/** A `YYYY-MM-DD` day as the start of that day in the viewer's time zone, as an ISO instant. */
export function dayStart(day: string): string | null {
  return localDay(day)?.toISOString() ?? null;
}

/** The instant after the end of a `YYYY-MM-DD` day (the start of the next day), for `until`. */
export function dayAfter(day: string): string | null {
  return localDay(day, 1)?.toISOString() ?? null;
}

/** What is wrong with the day filter, in a sentence, or null: a day that doesn't exist, or From after To. */
export function rangeProblem(from: string, to: string): string | null {
  if (from && !dayStart(from)) return 'The From date doesn’t exist.';
  if (to && !dayStart(to)) return 'The To date doesn’t exist.';
  if (from && to && from > to) return 'From is after To, so nothing can match.';
  return null;
}

/** Accepts an account's ID: 26 characters of Crockford base32. */
export const looksLikeId = (s: string) => /^[0-9A-HJKMNP-TV-Z]{26}$/i.test(s.trim());

/** A `YYYY-MM-DD` day in the cyex format, `2026/11/20`; nothing for anything else. */
export function dayLabel(day: string): string {
  return /^\d{4}-\d{2}-\d{2}$/.test(day) ? day.replaceAll('-', '/') : '';
}
