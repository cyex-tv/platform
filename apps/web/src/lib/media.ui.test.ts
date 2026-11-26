import { flushSync } from 'svelte';
import { render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import MediaHarness from './test/MediaHarness.svelte';

// A matchMedia that can be flipped: one list per query, as the browser keeps them.
type List = { matches: boolean; listeners: Set<() => void>; added: number };
const lists = new Map<string, List>();
const original = window.matchMedia;
function flip(query: string, matches: boolean) {
  const l = lists.get(query)!;
  l.matches = matches;
  l.listeners.forEach((f) => f());
  flushSync();
}

beforeEach(() => {
  lists.clear();
  delete document.documentElement.dataset.motion;
  window.matchMedia = ((query: string) => {
    let l = lists.get(query);
    if (!l) lists.set(query, (l = { matches: false, listeners: new Set(), added: 0 }));
    const list = l;
    return {
      get matches() {
        return list.matches;
      },
      media: query,
      addEventListener: (_: string, f: () => void) => (list.listeners.add(f), list.added++),
      removeEventListener: (_: string, f: () => void) => list.listeners.delete(f),
    } as unknown as MediaQueryList;
  }) as typeof window.matchMedia;
});
afterEach(() => {
  window.matchMedia = original;
});

const text = (c: HTMLElement, sel: string) => c.querySelector(sel)?.textContent;

describe('media queries', () => {
  it('follows the query and shares one listener between readers', () => {
    const a = render(MediaHarness, { query: '(min-width: 1100px)' });
    const b = render(MediaHarness, { query: '(min-width: 1100px)' });
    flushSync();
    expect(text(a.container, '[data-q]')).toBe('false');
    expect(lists.get('(min-width: 1100px)')!.listeners.size).toBe(1);
    flip('(min-width: 1100px)', true);
    expect(text(a.container, '[data-q]')).toBe('true');
    expect(text(b.container, '[data-q]')).toBe('true');
    a.unmount();
    b.unmount();
    expect(lists.get('(min-width: 1100px)')!.listeners.size).toBe(0);
  });

  it('reduced motion follows the system, and the site setting wins over it', () => {
    const r = render(MediaHarness, { query: '(min-width: 1px)' });
    flushSync();
    expect(text(r.container, '[data-reduced]')).toBe('false');
    flip('(prefers-reduced-motion: reduce)', true);
    expect(text(r.container, '[data-reduced]')).toBe('true');
    document.documentElement.dataset.motion = 'full';
    flip('(prefers-reduced-motion: reduce)', true);
    expect(text(r.container, '[data-reduced]')).toBe('false');
    document.documentElement.dataset.motion = 'reduce';
    flip('(prefers-reduced-motion: reduce)', false);
    expect(text(r.container, '[data-reduced]')).toBe('true');
  });
});
