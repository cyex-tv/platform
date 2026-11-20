import { render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import BadgeIcon from './BadgeIcon.svelte';
import { display } from '$lib/dpr.svelte';

const host = 'https://cdn.cyex.tv/cosmetics/badges/abc';
const file = (name: string, px: number) => ({ name, width: px, height: px, isStatic: name.includes('_static') });
const ladder = [file('1x.webp', 18), file('2x.webp', 36), file('4x.webp', 72), file('128.webp', 128), file('256.webp', 256), file('512.webp', 512)];
const badge = (files = ladder) => ({ name: 'Star', tooltip: 'A star', image: { id: 'i', kind: 'BADGE', host, animated: false, ready: true, width: 512, height: 512, files } }) as never;

const shown = (size: number, dpr: number, files = ladder) => {
  display.dpr = dpr;
  flushSync();
  const { container, unmount } = render(BadgeIcon, { badge: badge(files), size });
  const img = container.querySelector('img') as HTMLImageElement;
  const out = { src: img.getAttribute('src'), srcset: img.getAttribute('srcset'), sizes: img.getAttribute('sizes'), width: img.getAttribute('width') };
  unmount();
  return out;
};

afterEach(() => {
  display.dpr = 0;
});

describe('BadgeIcon picks from the size ladder', () => {
  it('draws the smallest file at or above size × pixel ratio, with no srcset after mount', () => {
    expect(shown(18, 1).src).toBe(`${host}/1x.webp`);
    expect(shown(28, 2).src).toBe(`${host}/4x.webp`);
    expect(shown(72, 2).src).toBe(`${host}/256.webp`);
    expect(shown(36, 3).src).toBe(`${host}/128.webp`);
    const after = shown(28, 2);
    expect(after.srcset).toBeNull();
    expect(after.width).toBe('28');
  });

  it('offers width descriptors before mount (ratio 0), with the drawn size', () => {
    const before = shown(28, 0);
    expect(before.src).toBe(`${host}/2x.webp`);
    expect(before.srcset).toBe(`${host}/1x.webp 18w, ${host}/2x.webp 36w, ${host}/4x.webp 72w, ${host}/128.webp 128w, ${host}/256.webp 256w, ${host}/512.webp 512w`);
    expect(before.sizes).toBe('28px');
  });

  it('falls back to the largest file of a picture made before the ladder', () => {
    const old = ladder.slice(0, 3);
    expect(shown(72, 2, old).src).toBe(`${host}/4x.webp`);
    expect(shown(18, 2, old).src).toBe(`${host}/2x.webp`);
  });
});
