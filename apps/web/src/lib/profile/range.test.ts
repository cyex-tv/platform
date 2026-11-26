import { describe, expect, it } from 'vitest';
import { ALL_TIME, dayOf, dayProblem, fromQuery, labelOf, lastDays, presetOf, rangeOf, rangeProblem, readDay, toQuery } from './range';

// 2026/11/19 14:05 in the reader's own time zone.
const now = new Date(2026, 10, 19, 14, 5);

describe('activity date range', () => {
  it('turns a range into local midnight of the first day to midnight after the last', () => {
    const r = rangeOf({ from: '2026-11-01', to: '2026-11-19' });
    expect(r.since).toBe(new Date(2026, 10, 1).toISOString());
    expect(r.until).toBe(new Date(2026, 10, 20).toISOString());
    expect(rangeOf({ from: '2026-12-31', to: '2026-12-31' }).until).toBe(new Date(2027, 0, 1).toISOString());
    expect(rangeOf(ALL_TIME)).toEqual({ since: null, until: null });
    expect(rangeOf({ from: '2026-11-01', to: null })).toEqual({ since: new Date(2026, 10, 1).toISOString(), until: null });
    expect(rangeOf({ from: 'nope', to: null })).toEqual({ since: null, until: null });
  });

  it('has presets for the last 7, 30 and 90 days, today included, and knows them again', () => {
    expect(lastDays(7, now)).toEqual({ from: '2026-11-13', to: '2026-11-19' });
    expect(lastDays(30, now)).toEqual({ from: '2026-10-21', to: '2026-11-19' });
    expect(presetOf(lastDays(30, now), now)).toBe(30);
    expect(presetOf(ALL_TIME, now)).toBe(0);
    expect(presetOf({ from: '2026-11-02', to: '2026-11-03' }, now)).toBeNull();
  });

  it('refuses future, broken and backwards dates in words', () => {
    expect(dayProblem('2026-11-19', now)).toBeNull();
    expect(dayProblem('2026-11-20', now)).toMatch(/future/);
    expect(dayProblem('2026-02-30', now)).toBe('Pick a date.');
    expect(rangeProblem({ from: '2026-11-10', to: '2026-11-01' })).toMatch(/after the end/);
    expect(rangeProblem({ from: '2026-11-01', to: '2026-11-01' })).toBeNull();
  });

  it('reads the date fields in the cyex format (and the dashed one); empty is an open end', () => {
    expect(readDay('2026/11/19', now)).toEqual({ day: '2026-11-19' });
    expect(readDay(' 2026/1/5 ', now)).toEqual({ day: '2026-01-05' });
    expect(readDay('2026-11-02', now)).toEqual({ day: '2026-11-02' });
    expect(readDay('', now)).toEqual({ day: null });
    expect(readDay('2026/11/21', now)).toEqual({ problem: expect.stringMatching(/future/) });
    expect(readDay('2026/02/30', now)).toEqual({ problem: expect.stringMatching(/isn’t a date/) });
    expect(readDay('25.12.2026', now)).toEqual({ problem: expect.stringMatching(/2026\/11\/19/) });
  });

  it('labels in cyex date format', () => {
    expect(labelOf({ from: '2026-11-01', to: '2026-11-19' })).toBe('2026/11/01 – 2026/11/19');
    expect(labelOf({ from: '2026-11-03', to: '2026-11-03' })).toBe('2026/11/03');
    expect(labelOf({ from: '2026-11-03', to: null })).toBe('From 2026/11/03');
    expect(labelOf({ from: null, to: '2026-11-03' })).toBe('Until 2026/11/03');
    expect(labelOf(ALL_TIME)).toBe('All time');
  });

  it('round-trips through the address (?from= ?to=) and drops what it cannot use', () => {
    for (const r of [lastDays(7, now), { from: '2026-11-01', to: null }, { from: null, to: '2026-11-02' }, ALL_TIME]) {
      expect(fromQuery(new URLSearchParams(toQuery(r)), now)).toEqual(r);
    }
    expect(toQuery({ from: '2026-11-01', to: '2026-11-19' })).toEqual({ from: '2026-11-01', to: '2026-11-19' });
    expect(fromQuery(new URLSearchParams('from=2030-01-01'), now)).toEqual(ALL_TIME);
    expect(fromQuery(new URLSearchParams('from=2026-11-10&to=2026-11-01'), now)).toEqual(ALL_TIME);
    // The old Day and Month addresses are all time now.
    expect(fromQuery(new URLSearchParams('d=2026-11-19&m=2026-11'), now)).toEqual(ALL_TIME);
    expect(dayOf(new Date(2026, 0, 5))).toBe('2026-01-05');
  });
});
