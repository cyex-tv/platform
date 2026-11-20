import { describe, expect, it } from 'vitest';
import { favouritesFirst, withoutHidden } from './lists.svelte';

const e = (id: string, owner?: string) => ({ id, owner: owner ? { id: owner } : null });

describe('hide for me', () => {
  it('drops hidden emotes and everything by a hidden creator', () => {
    const list = [e('a', 'x'), e('b', 'y'), e('c', 'x'), e('d')];
    expect(withoutHidden(list, new Set(['b']), new Set()).map((x) => x.id)).toEqual(['a', 'c', 'd']);
    expect(withoutHidden(list, new Set(), new Set(['x'])).map((x) => x.id)).toEqual(['b', 'd']);
    expect(withoutHidden(list, new Set(['d']), new Set(['x'])).map((x) => x.id)).toEqual(['b']);
  });
  it('keeps the list as it is when nothing is hidden', () => {
    const list = [e('a')];
    expect(withoutHidden(list, new Set(), new Set())).toBe(list);
  });
  it('keeps emotes whose creator is unknown or gone', () => {
    expect(withoutHidden([{ id: 'a' }, { id: 'b', owner: {} }], new Set(), new Set(['x'])).map((x) => x.id)).toEqual(['a', 'b']);
  });
});

describe('favourites first', () => {
  it('puts favourites first, each emote once, cut at the most', () => {
    const favs = [e('f1'), e('b')];
    const rest = [e('a'), e('b'), e('c'), e('d')];
    expect(favouritesFirst(favs, rest, 4).map((x) => x.id)).toEqual(['f1', 'b', 'a', 'c']);
    expect(favouritesFirst([], rest, 2).map((x) => x.id)).toEqual(['a', 'b']);
    expect(favouritesFirst(favs, [], Infinity).map((x) => x.id)).toEqual(['f1', 'b']);
  });
});
