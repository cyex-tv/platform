import { describe, expect, it } from 'vitest';
import { capacityText, cloneError, cloneOutcome, fillPercent, filterMembers } from './public';

const m = (name: string, own: string) => ({ name, emote: { name: own } });

describe('filterMembers', () => {
  const items = [m('KEKW', 'KEKW'), m('laugh', 'KEKW'), m('Sadge', 'Sadge')];
  it('returns everything for an empty or blank search', () => {
    expect(filterMembers(items, '')).toHaveLength(3);
    expect(filterMembers(items, '   ')).toHaveLength(3);
  });
  it('matches the name in the set and the emote’s own name, ignoring case', () => {
    expect(filterMembers(items, 'kek').map((x) => x.name)).toEqual(['KEKW', 'laugh']);
    expect(filterMembers(items, 'LAUGH')).toHaveLength(1);
  });
  it('returns nothing when nothing matches', () => {
    expect(filterMembers(items, 'zzz')).toEqual([]);
  });
});

describe('capacity', () => {
  it('writes n / cap with dots', () => expect(capacityText(1234, 5000)).toBe('1.234 / 5.000'));
  it('rounds the fill and clamps it', () => {
    expect(fillPercent(1, 3)).toBe(33);
    expect(fillPercent(2000, 1000)).toBe(100);
    expect(fillPercent(0, 1000)).toBe(0);
    expect(fillPercent(5, 0)).toBe(0);
  });
});

describe('cloneOutcome', () => {
  it('says what came along', () => {
    expect(cloneOutcome(1, 0)).toBe('1 emote copied.');
    expect(cloneOutcome(18, 0)).toBe('18 emotes copied.');
  });
  it('says how many were left out', () => expect(cloneOutcome(1200, 3)).toContain('3 left out'));
});

describe('cloneError', () => {
  it('has words for the API codes', () => {
    expect(cloneError('rate_limited', '')).toContain('Try again');
    expect(cloneError('bad_name', '')).toContain('1 to 100');
    expect(cloneError('not_found', '')).toContain('gone');
  });
  it('falls back to the API’s own message', () => {
    expect(cloneError('weird', 'Server says no.')).toBe('Server says no.');
    expect(cloneError(undefined, '')).toBe('That didn’t work. Try again.');
  });
});
