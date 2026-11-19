import { describe, expect, it } from 'vitest';

import { near, personMatch, quickTier, similar } from './search.mjs';

describe('mock emote search follows the API tiers', () => {
  it('ranks exact, case, prefix, word, substring', () => {
    expect(quickTier('PogChamp', 'PogChamp')).toBe(1);
    expect(quickTier('PogChamp', 'pogchamp')).toBe(2);
    expect(quickTier('PogChamp', 'pog')).toBe(3);
    expect(quickTier('PepePoint', 'point')).toBe(4);
    expect(quickTier('PepePoint', 'pp')).toBe(4);
    expect(quickTier('PogChamp', 'gcha')).toBe(5);
  });
  it('finds near misses from three letters like pg_trgm', () => {
    expect(quickTier('PogChamp', 'pogchmp')).toBe(6);
    expect(quickTier('PogChamp', 'pogchammp')).toBe(6);
    expect(quickTier('PogChamp', 'zzzzzz')).toBe(0);
    expect(quickTier('PogChamp', 'pq')).toBe(0);
  });
  it('counts trigrams per word, padded like pg_trgm', () => {
    // pg_trgm: similarity('word','word') = 1; 'hello world' vs 'world hello' = 1 (words, not the whole string)
    expect(similar('hello world', 'world hello')).toBe(1);
    expect(near('abc', 'xyz')).toBe(false);
  });
});

describe('mock people search matches every linked login', () => {
  const quiet = [
    { platform: 'TWITCH', login: 'quietoak', displayName: 'QuietOak' },
    { platform: 'KICK', login: 'oak_on_kick', displayName: 'QuietOak' },
  ];
  it('matches a login that is not the display name', () => {
    expect(personMatch(quiet, 'oak_on_kick').tier).toBe(3);
    expect(personMatch(quiet, 'oak_on').tier).toBe(2);
    expect(personMatch(quiet, 'on_kick').tier).toBeGreaterThan(0);
    expect(personMatch(quiet, 'nobody').tier).toBe(-1);
  });
  it('finds near misses on logins and names', () => {
    expect(personMatch(quiet, 'quitoak').tier).toBeGreaterThanOrEqual(0);
    expect(personMatch(quiet, 'oak_on_kik').tier).toBeGreaterThanOrEqual(0);
  });
});
