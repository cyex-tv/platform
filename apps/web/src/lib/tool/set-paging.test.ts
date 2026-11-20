import { describe, expect, it } from 'vitest';
import { SET_PAGE, appendPage, morePages, nextPage, pagesFor } from './set-paging';
import { appendResults, showingLine } from './add-emotes';

const m = (id: string) => ({ emote: { id } });

describe('set editor paging', () => {
  it('draws 120 at a time, like the public set page', () => {
    expect(SET_PAGE).toBe(120);
    expect(nextPage(120)).toBe(2);
    expect(nextPage(240)).toBe(3);
    expect(pagesFor(0)).toBe(1);
    expect(pagesFor(121)).toBe(2);
    expect(pagesFor(1000)).toBe(9);
  });
  it('lists the pages after the first that a thousand emotes need', () => {
    expect(morePages(1000, 200)).toEqual([2, 3, 4, 5]);
    expect(morePages(200, 200)).toEqual([]);
    expect(morePages(0, 200)).toEqual([]);
  });
  it('appends a page without repeating what is on screen', () => {
    expect(appendPage([m('a'), m('b')], [m('b'), m('c')]).map((x) => x.emote.id)).toEqual(['a', 'b', 'c']);
  });
});

describe('Add emotes paging', () => {
  it('appends the next page of results, each emote once', () => {
    expect(appendResults([{ id: 'a' }, { id: 'b' }], [{ id: 'b' }, { id: 'c' }]).map((e) => e.id)).toEqual(['a', 'b', 'c']);
  });
  it('says how much of the search is shown, only while there is more', () => {
    const c = (n: number) => String(n);
    expect(showingLine(40, 213, c)).toBe('Showing 40 of 213.');
    expect(showingLine(40, 40, c)).toBeNull();
    expect(showingLine(0, 0, c)).toBeNull();
  });
});
