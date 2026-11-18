import { describe, expect, it } from 'vitest';
import { active, apiQuery, apiSort, change, clientSide, matches, parse, toSearch, topTags } from './params';

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
    const s = '?q=pog&sort=new&kind=animated&zw=1&wide=1&nsfw=1&tag=cat&n=3';
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
  it('filters by one tag, any case, and asks the API for the tag without a query', () => {
    const cat = { files: [{ name: '1x.webp', width: 32, height: 32 }], tags: ['Cat', 'cute'] };
    expect(u('tag=%23CAT').tag).toBe('cat');
    expect(matches(u('tag=cat'), cat)).toBe(true);
    expect(matches(u('tag=dog'), cat)).toBe(false);
    expect(matches(u('tag=dog'), { files: cat.files })).toBe(false);
    expect(apiQuery(u('tag=cat'))).toBe('cat');
    expect(apiQuery(u('q=pog&tag=cat'))).toBe('pog');
    expect(clientSide(u('tag=cat'))).toBe(true);
    expect(clientSide(u('kind=still'))).toBe(false);
    expect(active(u('tag=cat')).map((a) => a.label)).toEqual(['#cat']);
  });
  it('counts the most used tags', () => {
    const items = [{ tags: ['cat', 'Cute'] }, { tags: ['cute', 'cute'] }, { tags: [] }, {}];
    expect(topTags(items)).toEqual([{ tag: 'cute', count: 2 }, { tag: 'cat', count: 1 }]);
    expect(topTags(items, 1)).toHaveLength(1);
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

describe('view, exact and select', () => {
  it('round-trips through the URL, leaving defaults out', () => {
    const b = u('q=cat&view=table&exact=1&select=1');
    expect(b).toMatchObject({ layout: 'table', exact: true, select: true });
    expect(toSearch(b)).toBe('?q=cat&view=table&exact=1&select=1');
    expect(u('').layout).toBe('');
    expect(toSearch(u(''))).toBe('');
  });
  it('exact needs a query', () => {
    expect(u('exact=1').exact).toBe(false);
    expect(change(u('q=cat&exact=1'), { q: '' }).exact).toBe(false);
  });
  it('exact keeps only names that equal the query, in any case', () => {
    const b = u('q=KEKW&exact=1');
    expect(matches(b, { name: 'kekw', files: [] })).toBe(true);
    expect(matches(b, { name: 'KEKW2', files: [] })).toBe(false);
    expect(clientSide(b)).toBe(true);
    expect(active(b).map((a) => a.key)).toEqual(['exact']);
  });
  it('a change keeps the layout and select mode but starts at page one', () => {
    const next = change(u('view=table&select=1&n=3'), { kind: 'animated' });
    expect(next).toMatchObject({ layout: 'table', select: true, pages: 1 });
  });
});

describe('browse params: the Users tab', () => {
  it('is emotes unless the address says users', () => {
    expect(u('').tab).toBe('emotes');
    expect(u('tab=zzz').tab).toBe('emotes');
    expect(u('tab=users').tab).toBe('users');
  });
  it('keeps only the name and pages in the address', () => {
    const b = u('tab=users&q=fox&kind=animated&nsfw=1&sort=new&n=2');
    expect(toSearch(b)).toBe('?tab=users&q=fox&n=2');
    expect(toSearch(u('tab=users'))).toBe('?tab=users');
  });
  it('switching tab keeps the name and starts at the first page', () => {
    const users = change(u('q=fox&n=3'), { tab: 'users' });
    expect(users).toMatchObject({ tab: 'users', q: 'fox', pages: 1, select: false });
    const back = change(users, { tab: 'emotes' });
    expect(toSearch(back)).toBe('?q=fox');
  });
});
