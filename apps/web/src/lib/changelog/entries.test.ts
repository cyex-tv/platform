import { describe, expect, it } from 'vitest';
import { byDay, ENTRIES, LATEST, shownDate } from './entries';

describe('changelog entries', () => {
  it('are newest first, with unique ids and real dates', () => {
    expect(new Set(ENTRIES.map((e) => e.id)).size).toBe(ENTRIES.length);
    const dates = ENTRIES.map((e) => e.date);
    expect([...dates].sort().reverse()).toEqual(dates);
    for (const e of ENTRIES) {
      expect(e.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(Number.isNaN(Date.parse(e.date))).toBe(false);
      expect(e.id.startsWith(e.date)).toBe(true);
    }
    expect(LATEST).toBe(ENTRIES[0]);
  });

  it('read in the cyex voice: no emoji, no exclamation marks, sentence case titles', () => {
    for (const e of ENTRIES) {
      for (const line of [e.title, ...e.items, ...(e.staff ?? [])]) {
        expect(line).not.toMatch(/!/);
        expect(line).not.toMatch(/\p{Extended_Pictographic}/u);
        expect(line).not.toMatch(/ - /);
      }
      expect(e.title[0]).toBe(e.title[0].toUpperCase());
      expect(e.title.split(' ').slice(1).some((w) => /^[A-Z][a-z]/.test(w) && w !== 'Kick' && w !== 'Twitch')).toBe(false);
    }
  });

  it('dates show as yyyy/mm/dd and group by day', () => {
    expect(shownDate('2026-11-20')).toBe('2026/11/20');
    const days = byDay();
    expect(days[0].date).toBe(ENTRIES[0].date);
    expect(days.reduce((n, d) => n + d.entries.length, 0)).toBe(ENTRIES.length);
    expect(new Set(days.map((d) => d.date)).size).toBe(days.length);
  });
});
