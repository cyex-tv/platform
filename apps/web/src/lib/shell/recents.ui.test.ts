import { afterEach, describe, expect, it, vi } from 'vitest';
import { load, push, RECENTS_KEY, save, tidy } from './recents';

afterEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
});

describe('recent searches', () => {
  it('keeps the newest five, without repeats in any case', () => {
    let l: string[] = [];
    for (const q of ['a', 'b', 'c', 'd', 'e', 'f']) l = push(l, q);
    expect(l).toEqual(['f', 'e', 'd', 'c', 'b']);
    expect(push(l, 'D')).toEqual(['D', 'f', 'e', 'c', 'b']);
  });
  it('cleans what it reads', () => {
    expect(tidy(['  pog ', '', 3, 'POG', null, 'kappa'])).toEqual(['pog', 'kappa']);
    expect(tidy('nope')).toEqual([]);
  });
  it('round-trips through storage and clears with an empty list', () => {
    expect(save(['pog', 'kappa'])).toBe(true);
    expect(load()).toEqual(['pog', 'kappa']);
    save([]);
    expect(localStorage.getItem(RECENTS_KEY)).toBeNull();
  });
  it('survives storage that throws: nothing remembered, nothing breaks', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('denied'); });
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('denied'); });
    expect(save(['pog'])).toBe(false);
    expect(load()).toEqual([]);
  });
  it('ignores a corrupt value', () => {
    localStorage.setItem(RECENTS_KEY, '{oops');
    expect(load()).toEqual([]);
  });
});
