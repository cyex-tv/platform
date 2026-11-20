import { describe, expect, it } from 'vitest';
import { SET_PAGE, appendPage, applyChange, morePages, pagesAfter, pagesFor } from './set-paging';
import { appendResults, showingLine } from './add-emotes';

const m = (id: string) => ({ emote: { id } });

describe('set editor paging', () => {
  it('draws 120 at a time, like the public set page', () => {
    expect(SET_PAGE).toBe(120);
    expect(pagesFor(0)).toBe(1);
    expect(pagesFor(121)).toBe(2);
    expect(pagesFor(1000)).toBe(9);
  });
  it('Load more asks for the next page, and after removals also the page the shifted emotes moved up into', () => {
    expect(pagesAfter(1, 0)).toEqual([2]);
    expect(pagesAfter(3, 0)).toEqual([4]);
    // One removal on page 1: 119 on screen, and the emote that moved up is on page 1 still.
    expect(pagesAfter(1, 1)).toEqual([1, 2]);
    expect(pagesAfter(2, 1)).toEqual([2, 3]);
    expect(pagesAfter(1, 120)).toEqual([1, 2]);
    expect(pagesAfter(2, 130)).toEqual([1, 2, 3]);
  });
  it('follows its own add, remove and rename in the member lists', () => {
    let m = { ids: new Set(['a', 'b']), names: new Set(['A', 'B']) };
    m = applyChange(m, { kind: 'add', id: 'c', name: 'C' });
    m = applyChange(m, { kind: 'remove', id: 'a', name: 'A' });
    m = applyChange(m, { kind: 'rename', id: 'b', from: 'B', to: 'Bee' });
    expect([...m.ids]).toEqual(['b', 'c']);
    expect([...m.names].sort()).toEqual(['Bee', 'C']);
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
