// The role editor's Limits section (setRoleLimits): plain names and tooltips for the API's limit
// keys, grouped as the API groups them, and the checks on what is typed. An empty box keeps the
// default; a number overrides it for everyone who holds the role. An account counts under the
// most generous override among its roles. File sizes (the Media group) are typed in MB.
import type { LimitGroup, RoleLimitFieldsFragment } from '$lib/gql/graphql';
import { count } from '$lib/format';

export type LimitInfo = { label: string; hint: string; /** What the number counts, for a cap that is neither per time nor at once (set capacity: slots a set). */ unit?: string };

export const LIMIT_INFO: Record<string, LimitInfo> = {
  gql_account: { label: 'API requests', hint: 'Calls to the API a signed-in account may make. The site and the extension make several per page.' },
  account_search: { label: 'Account searches', hint: 'Searches for accounts by name, and login checks in staff tools.' },
  quick_search: { label: 'Quick searches', hint: 'The search box in the header. It searches as you type, so keep it generous.' },
  heavy_reads: { label: 'Activity reads', hint: 'Activity lists and set lists, which each look up many things at once.' },
  uploads_per_hour: { label: 'Emote uploads', hint: 'Emotes an account may upload, and receive from its editors, in an hour.' },
  pending_emotes: { label: 'Waiting for review', hint: 'Emotes of one account that may wait for review at once. An upload past this is refused until some are reviewed.' },
  editors: { label: 'Editor changes', hint: 'Adding, changing and removing the editors of your account.' },
  cosmetics: { label: 'Cosmetics changes', hint: 'Picking badges and paints, and staff changes to cosmetics, the shop and codes.' },
  set_clones: { label: 'Set copies', hint: 'Copying a whole emote set at once.' },
  personal_lists: { label: 'Hides and favourites', hint: 'Hiding, unhiding, favouriting and unfavouriting emotes and creators.' },
  emote_transfers: { label: 'Emote transfers', hint: 'Giving emotes to another account, refused tries included.' },
  prefs: { label: 'Settings changes', hint: 'Changes to your own settings, such as notifications.' },
  checkout: { label: 'Checkouts', hint: 'Starting a checkout in the shop.' },
  redeem: { label: 'Code tries', hint: 'Redeeming codes, right or wrong. Wrong codes also lock out for a while on their own.' },
  data_export: { label: 'Data downloads', hint: 'Downloading a copy of everything about your account.' },
  badge_max_bytes: { label: 'Badge file size', hint: 'The biggest badge picture an account may upload (staff with cosmetics rights).', unit: 'MB' },
  badge_max_side: { label: 'Badge width and height', hint: 'The longest side of a badge picture. A role can raise it, up to the most shown.', unit: 'px' },
  badge_max_frames: { label: 'Badge frames', hint: 'The most frames an animated badge may have.', unit: 'frames' },
  paint_max_bytes: { label: 'Paint file size', hint: 'The biggest paint picture an account may upload (staff with cosmetics rights).', unit: 'MB' },
  paint_max_side: { label: 'Paint width and height', hint: 'The longest side of a paint picture. A role can raise it, up to the most shown.', unit: 'px' },
  personal_set_slots: { label: 'Personal set slots', hint: 'The emotes an account’s personal set holds: 3 by default. A subscriber role sets 10. Staff can override one set by hand (up to 1.000).', unit: 'slots' },
  set_capacity_max: { label: 'Set capacity', hint: 'The most emotes one channel set can hold, picked by its owner (a subscriber role could raise it). Staff can set any set up to 1.000.000.', unit: 'slots a set' },
};

export const GROUP_LABEL: Record<LimitGroup, string> = { REQUESTS: 'Requests', UPLOADS: 'Uploads', ACCOUNT: 'Account actions', MEDIA: 'File sizes' };
const GROUP_ORDER: LimitGroup[] = ['REQUESTS', 'UPLOADS', 'MEDIA', 'ACCOUNT'];

/** The most a count may be set to (the API's MAX_LIMIT); a limit's own `max` says its most. */
export const LIMIT_MAX = 1_000_000;

const MB = 1024 * 1024;
/** File sizes are stored in bytes and typed in MB. */
export const isBytes = (key: string) => key.endsWith('_max_bytes');
/** The number a limit shows and is typed as: MB for a file size (up to 2 decimals), else itself. */
export const shownNumber = (key: string, n: number) => (isBytes(key) ? Math.round((n / MB) * 100) / 100 : n);
/** A number as text for a box or a sentence: "8", "2,5", "1.000.000". */
export const numberText = (key: string, n: number) => {
  const v = shownNumber(key, n);
  return Number.isInteger(v) ? count(v) : String(v).replace('.', ',');
};

type LimitRef = { key: string; max?: number };

export function info(key: string): LimitInfo {
  return LIMIT_INFO[key] ?? { label: key.replaceAll('_', ' '), hint: '' };
}

/** "a minute", "an hour", "30 seconds"; null for a cap (pending emotes). */
export function per(seconds: number | null | undefined): string | null {
  if (seconds == null) return null;
  if (seconds === 60) return 'a minute';
  if (seconds === 3600) return 'an hour';
  if (seconds === 86_400) return 'a day';
  if (seconds % 86_400 === 0) return `per ${count(seconds / 86_400)} days`;
  if (seconds % 3600 === 0) return `per ${count(seconds / 3600)} hours`;
  if (seconds % 60 === 0) return `per ${count(seconds / 60)} minutes`;
  return `per ${count(seconds)} seconds`;
}

/** What a limit allows, in words: "600 a minute", "20 at once", "8 MB". */
export function amount(l: { key: string; windowSeconds?: number | null }, n: number): string {
  return `${numberText(l.key, n)} ${info(l.key).unit ?? per(l.windowSeconds) ?? 'at once'}`;
}

export type LimitGroupView = { group: LimitGroup; label: string; items: RoleLimitFieldsFragment[] };

export function grouped(limits: readonly RoleLimitFieldsFragment[]): LimitGroupView[] {
  return GROUP_ORDER.map((group) => ({ group, label: GROUP_LABEL[group], items: limits.filter((l) => l.group === group) })).filter(
    (g) => g.items.length > 0,
  );
}

/** The boxes' text from what the API says: the override, or empty for the default. */
export function toValues(limits: readonly RoleLimitFieldsFragment[]): Record<string, string> {
  return Object.fromEntries(limits.map((l) => [l.key, l.value == null ? '' : String(shownNumber(l.key, l.value)).replace('.', ',')]));
}

/** A box's text as the API's number (bytes for a file size), or NaN when it isn't one. */
function parse(key: string, raw: string): number {
  const t = raw.trim().replace(',', '.');
  if (isBytes(key)) return /^\d+(\.\d{1,2})?$/.test(t) ? Math.round(Number(t) * MB) : NaN;
  return /^\d+$/.test(t) ? Number(t) : NaN;
}

/** What is wrong with a box's text, or null. Empty is fine: it keeps the default. */
export function valueProblem(raw: string | undefined, limit?: LimitRef): string | null {
  const t = (raw ?? '').trim();
  if (!t) return null;
  const key = limit?.key ?? '';
  const max = limit?.max ?? LIMIT_MAX;
  const n = parse(key, t);
  if (Number.isNaN(n)) return isBytes(key) ? 'A number of MB, or empty for the default.' : 'A whole number, or empty for the default.';
  if (n < 1 || n > max) return `${isBytes(key) ? '0,01' : '1'} to ${numberText(key, max)}.`;
  return null;
}

/** The setRoleLimits input: every box that holds a number (`limits` give each key's most). */
export function toInput(values: Record<string, string>, limits: readonly LimitRef[] = []): { key: string; value: number }[] {
  const by = new Map(limits.map((l) => [l.key, l]));
  return Object.entries(values)
    .filter(([key, raw]) => raw.trim() && !valueProblem(raw, by.get(key) ?? { key }))
    .map(([key, raw]) => ({ key, value: parse(key, raw) }))
    .sort((a, b) => a.key.localeCompare(b.key));
}

/** Whether the boxes say something other than what was loaded. */
export function changed(before: Record<string, string>, after: Record<string, string>, limits: readonly LimitRef[] = []): boolean {
  const a = JSON.stringify(toInput(before, limits));
  const b = JSON.stringify(toInput(after, limits));
  return a !== b;
}

/** The limits a role overrides (the ones with a number), for the inspector's read-only list. */
export function overrides(limits: readonly RoleLimitFieldsFragment[]): RoleLimitFieldsFragment[] {
  return limits.filter((l) => l.value != null);
}
