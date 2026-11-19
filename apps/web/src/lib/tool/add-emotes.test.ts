import { describe, expect, it } from 'vitest';
import { freeName, mergeResults, ownMatches } from './add-emotes';

const e = (id: string, name: string, state = 'APPROVED', tags: string[] = []) => ({ id, name, state, tags });

describe('add emotes results', () => {
  it('matches your uploads by name or tag, case-insensitively', () => {
    expect(ownMatches('cat', e('1', 'CatJam'), false)).toBe(true);
    expect(ownMatches('happy', e('1', 'CatJam', 'APPROVED', ['Happy']), false)).toBe(true);
    expect(ownMatches('dog', e('1', 'CatJam'), false)).toBe(false);
    expect(ownMatches('  ', e('1', 'CatJam'), false)).toBe(false);
  });

  it('offers pending uploads to channel sets only, and never rejected ones', () => {
    expect(ownMatches('cat', e('1', 'CatJam', 'PENDING'), false)).toBe(true);
    expect(ownMatches('cat', e('1', 'CatJam', 'PENDING'), true)).toBe(false);
    expect(ownMatches('cat', e('1', 'CatJam', 'REJECTED'), false)).toBe(false);
  });

  it('puts your uploads first and lists each emote once', () => {
    const own = [e('1', 'CatJam'), e('2', 'Dog')];
    const everyone = [e('3', 'CatDance'), e('1', 'CatJam')];
    expect(mergeResults('cat', own, everyone, false).map((x) => x.id)).toEqual(['1', '3']);
  });
});

describe('a name for an emote that clashes', () => {
  it('adds the first free number', () => {
    expect(freeName('KEKW', new Set(['KEKW']))).toBe('KEKW2');
    expect(freeName('KEKW', new Set(['KEKW', 'KEKW2', 'KEKW3']))).toBe('KEKW4');
  });
  it('stays within 100 characters', () => {
    const long = 'a'.repeat(100);
    const out = freeName(long, new Set([long]));
    expect([...out]).toHaveLength(100);
    expect(out.endsWith('2')).toBe(true);
  });
});
