import { describe, expect, it } from 'vitest';
import { active, apiSort, apiVars, change, cleanTags, CLEARED, parse, toSearch, topTags, without } from './params';

const u = (s: string) => parse(new URLSearchParams(s));

describe('browse params', () => {
  it('defaults', () => {
    const b = u('');
    expect(b).toMatchObject({ q: '', sort: 'top', window: 'week', kind: 'all', pages: 1, nsfw: false, tags: [] });
    expect(toSearch(b)).toBe('');
  });
  it('relevance needs a query', () => {
    expect(u('sort=relevance').sort).toBe('top');
    expect(u('q=pog').sort).toBe('relevance');
    expect(u('q=pog&sort=new').sort).toBe('new');
  });
  it('round trips', () => {
    const s = '?q=pog&sort=new&kind=animated&zw=1&wide=1&nsfw=1&tag=cat&tag=happy&n=3';
    expect(toSearch(u(s.slice(1)))).toBe(s);
  });
  it('clamps pages and junk', () => {
    expect(u('n=99').pages).toBe(10);
    expect(u('n=-4').pages).toBe(1);
    expect(u('n=x&kind=zzz').kind).toBe('all');
    expect(u('sort=zzz').sort).toBe('top');
  });
  it('a change resets the page count and follows the query', () => {
    const b = change(u('n=4'), { q: 'cat' });
    expect(b).toMatchObject({ q: 'cat', sort: 'relevance', pages: 1 });
    expect(change(b, { q: '' }).sort).toBe('top');
  });
  it('lists active filters, one chip per tag', () => {
    expect(active(u('kind=still&wide=1')).map((a) => a.label)).toEqual(['Still', 'Wide']);
    expect(active(u('nsfw=1&tag=cat&tag=dog')).map((a) => a.label)).toEqual(['NSFW', '#cat', '#dog']);
  });
  it('removes one filter at a time, a tag alone', () => {
    const b = u('zw=1&tag=cat&tag=dog&kind=animated');
    expect(change(b, without(b, 'tag:cat')).tags).toEqual(['dog']);
    expect(change(b, without(b, 'zeroWidth')).zeroWidth).toBe(false);
    expect(change(b, without(b, 'kind')).kind).toBe('all');
    expect(toSearch(change(b, CLEARED))).toBe('');
  });
  it('counts the most used tags, leaving out the chosen ones', () => {
    const items = [{ tags: ['cat', 'Cute'] }, { tags: ['cute', 'cute'] }, { tags: [] }, {}];
    expect(topTags(items)).toEqual([{ tag: 'cute', count: 2 }, { tag: 'cat', count: 1 }]);
    expect(topTags(items, 1)).toHaveLength(1);
    expect(topTags(items, 12, ['cute'])).toEqual([{ tag: 'cat', count: 1 }]);
  });
});

describe('tags in the URL', () => {
  it('are cleaned, without repeats, ten at most', () => {
    expect(u('tag=%23CAT&tag=cat&tag=&tag=%20Dog%20').tags).toEqual(['cat', 'dog']);
    const many = Array.from({ length: 14 }, (_, i) => `tag=t${i}`).join('&');
    expect(u(many).tags).toHaveLength(10);
    expect(cleanTags(['a'.repeat(40)])[0]).toHaveLength(32);
  });
  it('change cleans tags too', () => {
    expect(change(u(''), { tags: ['#Pog', 'pog', 'Kek'] }).tags).toEqual(['pog', 'kek']);
  });
});

describe('trending', () => {
  it('is a sort with a window, the window only in the URL when it is not 7 days', () => {
    expect(u('sort=trending')).toMatchObject({ sort: 'trending', window: 'week' });
    expect(toSearch(u('sort=trending'))).toBe('?sort=trending');
    expect(toSearch(u('sort=trending&t=day'))).toBe('?sort=trending&t=day');
    expect(u('sort=trending&t=zzz').window).toBe('week');
    // A window without the Trending sort means nothing and is dropped.
    expect(toSearch(u('t=month'))).toBe('');
  });
  it('stays the sort when a query is typed', () => {
    expect(change(u('sort=trending&t=month'), { q: 'cat' })).toMatchObject({ sort: 'trending', window: 'month' });
  });
});

describe('apiSort and apiVars', () => {
  it('names the API order for each sort, relevance only with a query', () => {
    expect(apiSort(u('sort=new').sort)).toBe('NEW');
    expect(apiSort(u('').sort)).toBe('TOP');
    expect(apiSort(u('q=fern').sort)).toBe('RELEVANCE');
    expect(apiSort(u('sort=relevance').sort)).toBe('TOP');
    expect(apiSort(u('sort=trending').sort)).toBe('TRENDING');
  });
  it('sends every filter to the API', () => {
    expect(apiVars(u('q=pog&kind=still&zw=1&wide=1&nsfw=1&tag=cat&exact=1&sort=trending&t=day'), 2)).toEqual({
      query: 'pog',
      animated: false,
      sort: 'TRENDING',
      window: 'DAY',
      flags: ['ZERO_WIDTH', 'WIDE', 'NSFW'],
      tags: ['cat'],
      exact: true,
      page: 2,
      perPage: 48,
    });
  });
  it('leaves out what is off', () => {
    expect(apiVars(u(''), 1)).toEqual({
      query: '',
      animated: null,
      sort: 'TOP',
      window: null,
      flags: null,
      tags: null,
      exact: null,
      page: 1,
      perPage: 48,
    });
    expect(apiVars(u('kind=animated&sort=new&t=day'), 1)).toMatchObject({ animated: true, window: null });
  });
});

describe('exact and select', () => {
  it('round-trip through the URL, leaving defaults out', () => {
    const b = u('q=cat&exact=1&select=1');
    expect(b).toMatchObject({ exact: true, select: true });
    expect(toSearch(b)).toBe('?q=cat&exact=1&select=1');
  });
  it('the old layout switch is gone from the URL', () => {
    expect(toSearch(u('q=cat&view=table'))).toBe('?q=cat');
  });
  it('exact needs a query', () => {
    expect(u('exact=1').exact).toBe(false);
    expect(change(u('q=cat&exact=1'), { q: '' }).exact).toBe(false);
    expect(active(u('q=KEKW&exact=1')).map((a) => a.key)).toEqual(['exact']);
  });
  it('a change keeps select mode but starts at page one', () => {
    const next = change(u('select=1&n=3'), { kind: 'animated' });
    expect(next).toMatchObject({ select: true, pages: 1 });
  });
});

describe('browse params: emotes only (people are found from the header search)', () => {
  it('has no tab: an old ?tab=users address reads as the emote search', () => {
    expect('tab' in u('tab=users')).toBe(false);
    expect(toSearch(u('tab=users&q=fox&n=2'))).toBe('?q=fox&n=2');
    expect(toSearch(u('tab=users'))).toBe('');
  });
});
