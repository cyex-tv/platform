// How the profile keeps long lists bounded (owner, 2026/11/20: built for accounts at their
// editor limit, with hundreds of sets and thousands of emotes). Every list shows a page, says how
// many there are, and grows only on purpose ("Show more"); nothing grows the band at the top.
// Pure, so it is tested.
import { count } from '$lib/format';

/** Editors per step in the band's Editors panel (an account holds 50 at most). */
export const EDITORS_STEP = 10;
/** Set cards per step on the Emote sets tab. */
export const SETS_STEP = 24;
/** Emotes per step on the Active emotes tab (a set can hold 1.000). */
export const EMOTES_STEP = 120;
/** Role flags shown beside the name before the rest fold into "+N". */
export const ROLES_SHOWN = 3;
/** Above this many sets, the tab gets a name filter. */
export const SETS_FILTER_FROM = 12;

/** How many of `total` show after `steps` steps of `step` (at least one step). */
export function shown(total: number, steps: number, step: number): number {
  return Math.max(0, Math.min(total, Math.max(1, steps) * step));
}

/** "Showing 10 of 50", or "" when everything shows. */
export function showing(visible: number, total: number): string {
  return visible < total ? `Showing ${count(visible)} of ${count(total)}` : '';
}

/** The next step's button text: "Show 10 more" (or fewer when fewer are left). */
export function moreText(visible: number, total: number, step: number): string {
  return `Show ${count(Math.min(step, total - visible))} more`;
}

/** The first `max` items, and how many are folded away. */
export function fold<T>(items: readonly T[], max: number): { head: T[]; rest: number } {
  return { head: items.slice(0, max), rest: Math.max(0, items.length - max) };
}

/** Items whose name holds `q` (any case); all of them for a blank `q`. */
export function byName<T extends { name: string }>(items: readonly T[], q: string): T[] {
  const t = q.trim().toLowerCase();
  return t ? items.filter((i) => i.name.toLowerCase().includes(t)) : [...items];
}
