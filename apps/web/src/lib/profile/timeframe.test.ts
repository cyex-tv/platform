import { describe, expect, it } from 'vitest';
import { dayOf, dayProblem, frameTo, fromQuery, labelOf, monthProblem, rangeOf, toQuery } from './timeframe';

// 2026/11/19 14:05 in the reader's own time zone.
const now = new Date(2026, 10, 19, 14, 5);

describe('timeframe', () => {
  it('starts a day on today and a month on this month', () => {
    expect(frameTo('day', now)).toEqual({ frame: 'day', day: '2026-11-19' });
    expect(frameTo('month', now)).toEqual({ frame: 'month', year: 2026, month: 11 });
    expect(frameTo('all', now)).toEqual({ frame: 'all' });
  });

  it('turns a day into local midnight to midnight', () => {
    const r = rangeOf({ frame: 'day', day: '2026-11-19' });
    expect(r.since).toBe(new Date(2026, 10, 19).toISOString());
    expect(r.until).toBe(new Date(2026, 10, 20).toISOString());
  });

  it('turns a month into its first to the next first, across the year end', () => {
    const r = rangeOf({ frame: 'month', year: 2026, month: 12 });
    expect(r.since).toBe(new Date(2026, 11, 1).toISOString());
    expect(r.until).toBe(new Date(2027, 0, 1).toISOString());
    expect(rangeOf({ frame: 'all' })).toEqual({ since: null, until: null });
    expect(rangeOf({ frame: 'day', day: 'nope' })).toEqual({ since: null, until: null });
  });

  it('refuses future and broken dates the way cyex logs does', () => {
    expect(dayProblem('2026-11-19', now)).toBeNull();
    expect(dayProblem('2026-11-20', now)).toMatch(/future/);
    expect(dayProblem('2026-02-30', now)).toBe('Pick a date.');
    expect(dayProblem('', now)).toBe('Pick a date.');
    expect(monthProblem(2026, 11, now)).toBeNull();
    expect(monthProblem(2026, 12, now)).toMatch(/future/);
    expect(monthProblem(2027, 1, now)).toMatch(/future/);
    expect(monthProblem(2026, 13, now)).toBe('Pick a year and a month.');
  });

  it('labels in cyex date format', () => {
    expect(labelOf({ frame: 'day', day: '2026-11-03' })).toBe('2026/11/03');
    expect(labelOf({ frame: 'month', year: 2026, month: 4 })).toBe('2026/04');
    expect(labelOf({ frame: 'all' })).toBe('All time');
    const a = new Date(2026, 10, 2, 9).toISOString();
    const b = new Date(2026, 10, 19, 23, 59).toISOString();
    expect(labelOf({ frame: 'all' }, [b, a, 'junk'])).toBe('2026/11/02 – 2026/11/19');
    expect(labelOf({ frame: 'all' }, [b, b])).toBe('2026/11/19');
  });

  it('round-trips through the address and drops what it cannot use', () => {
    for (const t of [frameTo('day', now), frameTo('month', now), frameTo('all', now)]) {
      expect(fromQuery(new URLSearchParams(toQuery(t)), now)).toEqual(t);
    }
    expect(fromQuery(new URLSearchParams('d=2030-01-01'), now)).toEqual({ frame: 'all' });
    expect(fromQuery(new URLSearchParams('m=2026-13'), now)).toEqual({ frame: 'all' });
    expect(dayOf(new Date(2026, 0, 5))).toBe('2026-01-05');
  });
});
