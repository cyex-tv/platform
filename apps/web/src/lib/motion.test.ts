import { afterEach, describe, expect, it, vi } from 'vitest';
import { dur, pop, still } from './motion';

const node = {} as Element;
function reduce(on: boolean) {
  vi.stubGlobal('matchMedia', (q: string) => ({ matches: on && q.includes('reduce') }));
}

describe('motion', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('pops with opacity and scale, cubic out', () => {
    reduce(false);
    const t = pop(node);
    expect(t.duration).toBe(150);
    expect(t.css?.(0, 1)).toBe('opacity: 0; scale: 0.97; translate: 0 0px;');
    expect(t.css?.(1, 0)).toBe('opacity: 1; scale: 1; translate: 0 0px;');
    expect(pop(node, { duration: 100, y: -4, from: 0.98 }).css?.(0, 1)).toBe('opacity: 0; scale: 0.98; translate: 0 -4px;');
  });

  it('does nothing under reduced motion, read at call time', () => {
    reduce(true);
    expect(still()).toBe(true);
    expect(pop(node, { duration: 200 })).toEqual({ duration: 0 });
    expect(dur(200)).toBe(0);
    reduce(false);
    expect(dur(200)).toBe(200);
    expect(pop(node).duration).toBe(150);
  });
});
