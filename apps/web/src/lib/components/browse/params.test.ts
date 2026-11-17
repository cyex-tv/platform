import { describe, expect, it } from 'vitest';
import { active, apiSort, change, matches, parse, toSearch } from './params';

const u = (s: string) => parse(new URLSearchParams(s));

describe('browse params', () => {
  it('defaults', () => {
    const b = u('');
    expect(b).toMatchObject({ q: '', sort: 'top', kind: 'all', pages: 1, nsfw: false });
    expect(toSearch(b)).toBe('');
  });
  it('relevance needs a query', () => {
    expect(u('sort=relevance').sort).toBe('top');
    expect(u('q=pog').sort).toBe('relevance');
    expect(u('q=pog&sort=new').sort).toBe('new');
  });
  it('round trips', () => {
    const s = '?q=pog&sort=new&kind=animated&zw=1&wide=1&nsfw=1&n=3';
    expect(toSearch(u(s.slice(1)))).toBe(s);
  });
  it('clamps pages and junk', () => {
    expect(u('n=99').pages).toBe(10);
    expect(u('n=-4').pages).toBe(1);
    expect(u('n=x&kind=zzz').kind).toBe('all');
  });
  it('a change resets the page count and follows the query', () => {
    const b = change(u('n=4'), { q: 'cat' });
    expect(b).toMatchObject({ q: 'cat', sort: 'relevance', pages: 1 });
    expect(change(b, { q: '' }).sort).toBe('top');
  });
  it('lists active filters', () => {
    expect(active(u('kind=still&wide=1')).map((a) => a.label)).toEqual(['Still', 'Wide']);
  });
  it('filters wide and zero-width client side', () => {
    const wide = { files: [{ name: '1x.webp', width: 64, height: 32 }], flags: { zeroWidth: false } };
    expect(matches(u('wide=1'), wide)).toBe(true);
    expect(matches(u('zw=1'), wide)).toBe(false);
  });
});

describe('apiSort', () => {
  it('names the API order for each sort, relevance only with a query', () => {
    expect(apiSort(parse(new URLSearchParams('sort=new')).sort)).toBe('NEW');
    expect(apiSort(parse(new URLSearchParams('')).sort)).toBe('TOP');
    expect(apiSort(parse(new URLSearchParams('q=fern')).sort)).toBe('RELEVANCE');
    expect(apiSort(parse(new URLSearchParams('sort=relevance')).sort)).toBe('TOP');
  });
});
