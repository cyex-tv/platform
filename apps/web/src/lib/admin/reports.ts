// The report queue as the admin reads it: reports grouped per emote, how old, and which row to
// pick once one is resolved.
import { date } from '$lib/format';

export type Reported = { id: string; createdAt: string; emote?: { id: string } | null };
export type ReportGroup<R extends Reported> = { key: string; emote: R['emote']; reports: R[] };

/** One group per emote (a deleted emote's reports each stand alone), the one with most reports first. */
export function groupReports<R extends Reported>(items: readonly R[]): ReportGroup<R>[] {
  const by = new Map<string, ReportGroup<R>>();
  for (const r of items) {
    const key = r.emote?.id ?? `gone-${r.id}`;
    const g: ReportGroup<R> = by.get(key) ?? { key, emote: r.emote, reports: [] };
    g.reports.push(r);
    by.set(key, g);
  }
  return [...by.values()].sort((a, b) => b.reports.length - a.reports.length);
}

/** The most recent report of a group. */
export function newestReport<R extends Reported>(reports: readonly R[]): R | undefined {
  let best: R | undefined;
  for (const r of reports) if (!best || Date.parse(r.createdAt) > Date.parse(best.createdAt)) best = r;
  return best;
}

/** How long ago: `now`, `5m`, `3h`, `2d`, then the date after two weeks. */
export function age(iso: string, now: number): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return '';
  const s = Math.max(0, Math.floor((now - t) / 1000));
  if (s < 60) return 'now';
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  if (s < 86400) return `${Math.floor(s / 3600)}h`;
  if (s < 14 * 86400) return `${Math.floor(s / 86400)}d`;
  return date(iso);
}

/** The key to pick after the group at `at` is gone: the one that took its place, else the last, else none. */
export function nextKey(keys: readonly string[], at: number): string | null {
  if (!keys.length) return null;
  return keys[Math.min(Math.max(at, 0), keys.length - 1)];
}
