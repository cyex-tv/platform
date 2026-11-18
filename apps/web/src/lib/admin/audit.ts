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

/** A `YYYY-MM-DD` day as the start of that day in the viewer's time zone, as an ISO instant. */
export function dayStart(day: string): string | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) return null;
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])).toISOString();
}

/** The instant after the end of a `YYYY-MM-DD` day (the start of the next day), for `until`. */
export function dayAfter(day: string): string | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) return null;
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]) + 1).toISOString();
}

/** Accepts an account's ID: 26 characters of Crockford base32. */
export const looksLikeId = (s: string) => /^[0-9A-HJKMNP-TV-Z]{26}$/i.test(s.trim());
