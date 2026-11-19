// The activity feed's timeframe, the way cyex logs does it (logs-frontend, the "Date" stat and its
// jump popover): the feed starts on everything recent, newest first; a day or a month narrows it,
// picked in a small popover ("Jump to date" with a date field, "Jump to month" with a year and a
// month), and a future date is refused with a line saying so. The label is the cyex date format:
// 2026/11/19 for a day, 2026/11 for a month, and for "All time" the span of what has loaded
// (2026/11/02 – 2026/11/19), like logs' Date stat. Days and months are the reader's own calendar.
// Pure, so it is tested (timeframe.test.ts).

export type Timeframe =
  | { frame: 'all' }
  | { frame: 'day'; day: string }
  | { frame: 'month'; year: number; month: number };

export type Frame = Timeframe['frame'];

const pad = (n: number) => String(n).padStart(2, '0');

/** `YYYY-MM-DD` of `d` in the reader's time zone (what `<input type="date">` takes). */
export function dayOf(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Local midnight of a `YYYY-MM-DD`, or null for anything else. */
function midnight(day: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) return null;
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  return dayOf(d) === day ? d : null;
}

/** Switching the frame: a day starts on today, a month on this month (logs' defaults). */
export function frameTo(frame: Frame, now = new Date()): Timeframe {
  if (frame === 'day') return { frame, day: dayOf(now) };
  if (frame === 'month') return { frame, year: now.getFullYear(), month: now.getMonth() + 1 };
  return { frame: 'all' };
}

/** The API's `since` (inclusive) and `until` (exclusive) for a timeframe, as RFC 3339. */
export function rangeOf(t: Timeframe): { since: string | null; until: string | null } {
  if (t.frame === 'day') {
    const start = midnight(t.day);
    if (!start) return { since: null, until: null };
    const end = new Date(start.getFullYear(), start.getMonth(), start.getDate() + 1);
    return { since: start.toISOString(), until: end.toISOString() };
  }
  if (t.frame === 'month') {
    const start = new Date(t.year, t.month - 1, 1);
    const end = new Date(t.year, t.month, 1);
    return { since: start.toISOString(), until: end.toISOString() };
  }
  return { since: null, until: null };
}

/** Why a picked day can't be used (empty, not a date, in the future); null when it can. */
export function dayProblem(day: string, now = new Date()): string | null {
  const d = midnight(day);
  if (!d) return 'Pick a date.';
  if (day > dayOf(now)) return 'That date is in the future. Pick today or an earlier date.';
  return null;
}

/** Why a picked month can't be used; null when it can. */
export function monthProblem(year: number, month: number, now = new Date()): string | null {
  if (!Number.isInteger(year) || year < 2000 || !Number.isInteger(month) || month < 1 || month > 12) return 'Pick a year and a month.';
  const y = now.getFullYear();
  if (year > y || (year === y && month > now.getMonth() + 1)) return 'That month is in the future. Pick this month or an earlier one.';
  return null;
}

/** `2026/11/19` from a `YYYY-MM-DD`. */
const slashDay = (day: string) => day.replaceAll('-', '/');

/**
 * The timeframe's label in cyex format. "All time" names the span of the loaded lines
 * (`shown`, RFC 3339), one date when they share a day, "All time" when there are none.
 */
export function labelOf(t: Timeframe, shown: readonly string[] = []): string {
  if (t.frame === 'day') return slashDay(t.day);
  if (t.frame === 'month') return `${t.year}/${pad(t.month)}`;
  const days = shown
    .map((iso) => new Date(iso))
    .filter((d) => !Number.isNaN(d.getTime()))
    .map(dayOf)
    .sort();
  if (!days.length) return 'All time';
  const [a, b] = [days[0], days[days.length - 1]];
  return a === b ? slashDay(a) : `${slashDay(a)} – ${slashDay(b)}`;
}

/** The timeframe in the address: `d=2026-11-19` or `m=2026-11`; nothing for all time. */
export function toQuery(t: Timeframe): Record<string, string> {
  if (t.frame === 'day') return { d: t.day };
  if (t.frame === 'month') return { m: `${t.year}-${pad(t.month)}` };
  return {};
}

/** Reads `toQuery` back; anything unknown, broken or in the future is all time. */
export function fromQuery(q: URLSearchParams, now = new Date()): Timeframe {
  const d = q.get('d');
  if (d && !dayProblem(d, now)) return { frame: 'day', day: d };
  const m = /^(\d{4})-(\d{2})$/.exec(q.get('m') ?? '');
  if (m && !monthProblem(Number(m[1]), Number(m[2]), now)) return { frame: 'month', year: Number(m[1]), month: Number(m[2]) };
  return { frame: 'all' };
}
