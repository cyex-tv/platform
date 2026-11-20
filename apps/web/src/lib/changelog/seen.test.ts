import { beforeEach, describe, expect, it } from 'vitest';
import { SEEN_KEY, whatsNew } from './seen.svelte';
import { LATEST } from './entries';

const store = new Map<string, string>();
globalThis.localStorage = {
  getItem: (k: string) => store.get(k) ?? null,
  setItem: (k: string, v: string) => void store.set(k, v),
  removeItem: (k: string) => void store.delete(k),
  clear: () => store.clear(),
  key: () => null,
  length: 0,
} as Storage;

describe('what’s new', () => {
  beforeEach(() => store.clear());

  it('unseen until this device opens the newest entry, then quiet until a newer one', () => {
    whatsNew.load();
    expect(whatsNew.unseen).toBe(true);
    whatsNew.markSeen();
    expect(store.get(SEEN_KEY)).toBe(LATEST.id);
    whatsNew.load();
    expect(whatsNew.unseen).toBe(false);
    whatsNew.load('2099-01-01-later');
    expect(whatsNew.unseen).toBe(true);
  });

  it('a blocked storage never throws', () => {
    const real = globalThis.localStorage;
    globalThis.localStorage = { getItem: () => { throw new Error('blocked'); }, setItem: () => { throw new Error('blocked'); } } as unknown as Storage;
    expect(() => whatsNew.load()).not.toThrow();
    expect(() => whatsNew.markSeen()).not.toThrow();
    globalThis.localStorage = real;
  });
});
