// The role editor's Limits section (setRoleLimits): plain names and tooltips for the API's limit
// keys, grouped as the API groups them, and the checks on what is typed. An empty box keeps the
// default; a number overrides it for everyone who holds the role. An account counts under the
// most generous override among its roles.
import type { LimitGroup, RoleLimitFieldsFragment } from '$lib/gql/graphql';
import { count } from '$lib/format';

export type LimitInfo = { label: string; hint: string };

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
};

export const GROUP_LABEL: Record<LimitGroup, string> = { REQUESTS: 'Requests', UPLOADS: 'Uploads', ACCOUNT: 'Account actions' };
const GROUP_ORDER: LimitGroup[] = ['REQUESTS', 'UPLOADS', 'ACCOUNT'];

/** The most a limit may be set to (the API's MAX_LIMIT). */
export const LIMIT_MAX = 1_000_000;

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

/** What a limit allows, in words: "600 a minute", "20 at once". */
export function amount(n: number, seconds: number | null | undefined): string {
  return `${count(n)} ${per(seconds) ?? 'at once'}`;
}

export type LimitGroupView = { group: LimitGroup; label: string; items: RoleLimitFieldsFragment[] };

export function grouped(limits: readonly RoleLimitFieldsFragment[]): LimitGroupView[] {
  return GROUP_ORDER.map((group) => ({ group, label: GROUP_LABEL[group], items: limits.filter((l) => l.group === group) })).filter(
    (g) => g.items.length > 0,
  );
}

/** The boxes' text from what the API says: the override, or empty for the default. */
export function toValues(limits: readonly RoleLimitFieldsFragment[]): Record<string, string> {
  return Object.fromEntries(limits.map((l) => [l.key, l.value == null ? '' : String(l.value)]));
}

/** What is wrong with a box's text, or null. Empty is fine: it keeps the default. */
export function valueProblem(raw: string | undefined): string | null {
  const t = (raw ?? '').trim();
  if (!t) return null;
  if (!/^\d+$/.test(t)) return 'A whole number, or empty for the default.';
  const n = Number(t);
  if (n < 1 || n > LIMIT_MAX) return `1 to ${count(LIMIT_MAX)}.`;
  return null;
}

/** The setRoleLimits input: every box that holds a number. */
export function toInput(values: Record<string, string>): { key: string; value: number }[] {
  return Object.entries(values)
    .filter(([, raw]) => raw.trim() && !valueProblem(raw))
    .map(([key, raw]) => ({ key, value: Number(raw.trim()) }))
    .sort((a, b) => a.key.localeCompare(b.key));
}

/** Whether the boxes say something other than what was loaded. */
export function changed(before: Record<string, string>, after: Record<string, string>): boolean {
  const a = JSON.stringify(toInput(before));
  const b = JSON.stringify(toInput(after));
  return a !== b;
}
