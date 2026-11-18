import { describe, expect, it } from 'vitest';
import { batches, mergeLooks, toAsk } from './merge';

const badge = { id: 'b1', name: 'Founder', tooltip: null, image: null } as never;
const paint = { id: 'p1', name: 'Dusk' } as never;

describe('toAsk', () => {
  it('drops known, empty and repeated ids, keeping order', () => {
    expect(toAsk(['a', 'b', null, undefined, 'a', '', 'c'], new Set(['b']))).toEqual(['a', 'c']);
  });
});

describe('batches', () => {
  it('splits at 100', () => {
    const ids = Array.from({ length: 250 }, (_, i) => String(i));
    expect(batches(ids).map((b) => b.length)).toEqual([100, 100, 50]);
    expect(batches([])).toEqual([]);
  });
});

describe('mergeLooks', () => {
  it('gives every asked id a look and the rest an empty one', () => {
    const got = mergeLooks(['a', 'b'], [{ accountId: 'a', badge, paint }]);
    expect(got.a).toEqual({ badge, paint });
    expect(got.b).toEqual({ badge: null, paint: null });
  });
  it('ignores rows nobody asked for and keeps what is already known', () => {
    const got = mergeLooks(['a', 'me'], [{ accountId: 'x', badge }, { accountId: 'me', paint }], new Set(['me']));
    expect(Object.keys(got)).toEqual(['a']);
  });
  it('treats a missing badge or paint as none', () => {
    expect(mergeLooks(['a'], [{ accountId: 'a', paint }]).a).toEqual({ badge: null, paint });
  });
});
