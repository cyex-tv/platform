import { afterEach, describe, expect, it, vi } from 'vitest';
import { densityFor, pictureFor, srcset } from './emote';
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

describe('animated emotes follow the site setting', () => {
  const e = { host: 'h', animated: true, files: [{ name: '1x_static.webp', width: 32, height: 32, isStatic: true }] };

  it('the still srcset names the _static files', () => {
    expect(srcset(e, true)).toContain('h/1x_static.webp 1x');
    expect(srcset(e)).not.toContain('_static');
  });
});

describe('emote pictures pick their file in script once mounted (no srcset)', () => {
  const e = {
    host: 'h',
    animated: true,
    files: [1, 2, 3, 4].flatMap((s) => [
      { name: `${s}x.webp`, width: 28 * s, height: 28 * s, isStatic: false },
      { name: `${s}x_static.webp`, width: 28 * s, height: 28 * s, isStatic: true },
    ]),
  };

  it('offers a srcset before mount (server render, hydration) and none after', () => {
    expect(pictureFor(e, 0).srcset).toContain('h/2x.webp 2x');
    expect(pictureFor(e, 0, false, 28).srcset).toContain('h/4x.webp 112w');
    expect(pictureFor(e, 1).srcset).toBeUndefined();
    expect(pictureFor(e, 2, false, 28).srcset).toBeUndefined();
  });

  it('picks what the srcset would: the smallest sharp enough file, else the biggest', () => {
    expect(densityFor(1)).toBe(1);
    expect(densityFor(1.25)).toBe(2);
    expect(densityFor(3)).toBe(4);
    expect(densityFor(5)).toBe(4);
    expect(densityFor(2, [1])).toBe(1);
    expect(pictureFor(e, 1).src).toBe('h/1x.webp');
    expect(pictureFor(e, 1.5, true).src).toBe('h/2x_static.webp');
    expect(pictureFor(e, 2, false, 40).src).toBe('h/3x.webp');
    expect(pictureFor(e, 3, false, 56).src).toBe('h/4x.webp');
  });
});
