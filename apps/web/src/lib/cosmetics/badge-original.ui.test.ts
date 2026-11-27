import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import BadgeOriginal from './BadgeOriginal.svelte';

const host = 'https://cdn.cyex.tv/cosmetics/badges/abc';
const file = (name: string, px: number) => ({ name, width: px, height: px, isStatic: name.includes('_static') });
const ladder = [file('1x.webp', 18), file('2x.webp', 36), file('4x.webp', 72), file('128.webp', 128), file('256.webp', 256), file('512.webp', 512)];
const badge = (files: ReturnType<typeof file>[], px: number) => ({ name: 'Star', tooltip: 'A star', image: { id: 'i', kind: 'BADGE', host, animated: false, ready: true, width: px, height: px, files } }) as never;
const img = (b: never) => render(BadgeOriginal, { badge: b }).container.querySelector('img') as HTMLImageElement;

describe('BadgeOriginal shows the original at its real size', () => {
  it('uses original.webp, not a ladder size, at the picture’s own pixels', () => {
    const el = img(badge([...ladder, file('original.webp', 96)], 96));
    expect(el.getAttribute('src')).toBe(`${host}/original.webp`);
    expect([el.getAttribute('width'), el.getAttribute('height')]).toEqual(['96', '96']);
  });

  it('keeps small pixel art crisp and a 512 px picture smooth', () => {
    expect(img(badge([...ladder, file('original.webp', 96)], 96)).style.imageRendering).toBe('pixelated');
    const big = img(badge([...ladder, file('original.webp', 512)], 512));
    expect(big.getAttribute('width')).toBe('512');
    expect(big.style.imageRendering).toBe('auto');
  });

  it('falls back to the largest ladder size for a picture made before originals', () => {
    const el = img(badge(ladder, 512));
    expect(el.getAttribute('src')).toBe(`${host}/512.webp`);
  });
});
