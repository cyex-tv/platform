import { describe, expect, it } from 'vitest';
import { announce, emptyText, move, searchHref } from './search';

describe('header search', () => {
  it('moves and wraps, starting from none', () => {
    expect(move(-1, 1, 4)).toBe(0);
    expect(move(-1, -1, 4)).toBe(3);
    expect(move(3, 1, 4)).toBe(0);
    expect(move(0, -1, 4)).toBe(3);
    expect(move(-1, 1, 0)).toBe(-1);
  });
  it('Enter with no row picked opens the full search', () => {
    expect(searchHref('  pepe point ')).toBe('/search?q=pepe%20point');
    expect(searchHref('50%&x')).toBe('/search?q=50%25%26x');
  });
  it('announces counts in words, or that nothing matched', () => {
    expect(announce(5, 2, 'pog')).toBe('5 emotes, 2 people.');
    expect(announce(1, 1, 'pog')).toBe('1 emote, 1 person.');
    expect(announce(0, 3, 'ana')).toBe('3 people.');
    expect(announce(0, 0, 'zzz')).toBe(emptyText('zzz'));
    expect(announce(0, 0, '')).toBe('');
  });
});
