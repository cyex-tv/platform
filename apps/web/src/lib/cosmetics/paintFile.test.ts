import { describe, expect, it } from 'vitest';
import { PAINT_LADDER, fitOfText, lacksPaintSizes, neededHeight, pickPaintFile, type PaintPicture } from './paintFile';
import { paintStyle, type PaintLike } from './paint';

const file = (stem: string, px: number, aspect = 4, still = false) => ({ name: `${stem}${still ? '_static' : ''}.webp`, width: px * aspect, height: px });
const ladder = (aspect = 4, animated = false): PaintPicture => ({
  animated,
  files: PAINT_LADDER.flatMap(({ stem, px }) => (animated ? [file(stem, px, aspect), file(stem, px, aspect, true)] : [file(stem, px, aspect)])),
});
const pick = (img: PaintPicture, fit: Parameters<typeof pickPaintFile>[2], still = false, repeat = false) => pickPaintFile(img, still, fit, repeat);

describe('the paint size ladder', () => {
  it('is 64, 128, 256 and 512 px tall', () => {
    expect(PAINT_LADDER.map((s) => [s.stem, s.px])).toEqual([['1x', 64], ['2x', 128], ['4x', 256], ['512', 512]]);
  });
});

describe('what a name needs', () => {
  it('is the larger of its height and the height that covers its width, times the pixel ratio', () => {
    expect(neededHeight({ height: 20, width: 100, dpr: 1 }, 4, false)).toBe(25);
    expect(neededHeight({ height: 20, width: 40, dpr: 2 }, 4, false)).toBe(40);
    // A repeating picture is `auto 100%`: only the height counts.
    expect(neededHeight({ height: 20, width: 400, dpr: 1 }, 4, true)).toBe(20);
    expect(neededHeight({ height: 20, dpr: 0 }, 4, false)).toBe(20);
    expect(neededHeight({ dpr: 2 }, 4, false)).toBeNull();
    expect(neededHeight(undefined, 4, false)).toBeNull();
  });
  it('is estimated from the font size and the name length', () => {
    expect(fitOfText(20, 10, 2)).toEqual({ height: 24, width: 120, dpr: 2 });
    expect(fitOfText(0, 10, 2)).toEqual({ dpr: 2 });
  });
});

describe('picking a paint file', () => {
  const full = ladder();
  it('short names stay on the small files', () => {
    // 16 px font, 2 characters
    for (const dpr of [1, 2, 3]) expect(pick(full, fitOfText(16, 2, dpr))).toBe('1x.webp');
  });
  it('a 28 px profile name of 12 characters: 1x at dpr 1, 2x at dpr 2', () => {
    expect(pick(full, fitOfText(28, 12, 1))).toBe('1x.webp');
    expect(pick(full, fitOfText(28, 12, 2))).toBe('2x.webp');
  });
  it('a long name needs a taller file to cover its width (4:1 picture, 25 characters, 28 px)', () => {
    expect(pick(full, fitOfText(28, 25, 1))).toBe('2x.webp');
    expect(pick(full, fitOfText(28, 25, 2))).toBe('4x.webp');
    expect(pick(full, fitOfText(28, 25, 3))).toBe('512.webp');
  });
  it('a squarer picture covers a long name with more height: 512 at dpr 1', () => {
    expect(pick(ladder(1), fitOfText(28, 25, 1))).toBe('512.webp');
    // An 8:1 picture needs less.
    expect(pick(ladder(8), fitOfText(28, 25, 2))).toBe('2x.webp');
  });
  it('a repeating picture only needs the box height', () => {
    expect(pick(full, fitOfText(28, 25, 2), false, true)).toBe('2x.webp');
  });
  it('takes the largest when none is big enough', () => {
    const old = { ...full, files: full.files.filter((f) => f.name !== '512.webp') };
    expect(pick(old, fitOfText(28, 25, 3))).toBe('4x.webp');
    expect(pick(full, { height: 400, width: 4000, dpr: 3 })).toBe('512.webp');
  });
  it('without a known size: 2x from a pixel ratio of 1.5, else 1x', () => {
    expect(pick(full, undefined)).toBe('1x.webp');
    expect(pick(full, { dpr: 1 })).toBe('1x.webp');
    expect(pick(full, { dpr: 1.25 })).toBe('1x.webp');
    expect(pick(full, { dpr: 1.5 })).toBe('2x.webp');
    expect(pick(full, { dpr: 2 })).toBe('2x.webp');
    expect(pick(full, { dpr: 0 })).toBe('1x.webp');
  });
  it('shows the first frame of an animated picture when asked', () => {
    const anim = ladder(4, true);
    expect(pick(anim, fitOfText(28, 12, 2))).toBe('2x.webp');
    expect(pick(anim, fitOfText(28, 12, 2), true)).toBe('2x_static.webp');
    // A still picture has no still files: its own file serves.
    expect(pick(full, fitOfText(28, 12, 2), true)).toBe('2x.webp');
  });
  it('animates at the largest size that animates (a heavy 512 only has its still)', () => {
    const anim = ladder(4, true);
    const heavy = { ...anim, files: anim.files.filter((f) => f.name !== '512.webp') };
    expect(pick(heavy, { height: 300, dpr: 2 })).toBe('4x.webp');
    expect(pick(heavy, { height: 300, dpr: 2 }, true)).toBe('512_static.webp');
  });
  it('a picture smaller than its steps picks the first file of its own height', () => {
    const small: PaintPicture = {
      animated: false,
      files: [file('1x', 64), { name: '2x.webp', width: 400, height: 100 }, { name: '4x.webp', width: 400, height: 100 }, { name: '512.webp', width: 400, height: 100 }],
    };
    expect(pick(small, { height: 100, dpr: 3 })).toBe('2x.webp');
    expect(pick(small, { height: 30, dpr: 1 })).toBe('1x.webp');
  });
  it('old pictures (1x, 2x and 4x) work; nothing made gives null', () => {
    const old: PaintPicture = { animated: false, files: [file('1x', 64), file('2x', 128), file('4x', 256)] };
    expect(pick(old, { height: 20, dpr: 2 })).toBe('1x.webp');
    expect(pick(old, { height: 100, dpr: 3 })).toBe('4x.webp');
    expect(pick({ animated: false, files: [] }, { height: 20, dpr: 1 })).toBeNull();
    // Files without sizes use the ladder's.
    expect(pick({ animated: false, files: [{ name: '1x.webp' }, { name: '2x.webp' }] }, { height: 70, dpr: 1 })).toBe('2x.webp');
    // Only the picture's own size is known: it gives the shape.
    expect(pick({ animated: false, width: 256, height: 256, files: [{ name: '1x.webp' }, { name: '2x.webp' }, { name: '4x.webp' }] }, fitOfText(28, 12, 1))).toBe('4x.webp');
  });
});

describe('older pictures', () => {
  it('lack the 512 px size until made again', () => {
    expect(lacksPaintSizes(['1x.webp', '2x.webp', '4x.webp', '1x.avif'])).toBe(true);
    expect(lacksPaintSizes(['1x.webp', '2x.webp', '4x.webp', '512.webp'])).toBe(false);
    expect(lacksPaintSizes(['1x.webp', '2x.webp', '4x.webp', '512_static.webp'])).toBe(false);
    expect(lacksPaintSizes([])).toBe(false);
  });
});

describe('the painted name uses the picked file', () => {
  const p: PaintLike = {
    kind: 'IMAGE', angle: 90, repeat: false, shape: 'CIRCLE', color: '#ff7a45', stops: [], shadows: [],
    image: { host: 'https://cdn.cyex.tv/cosmetics/paints/abc', ready: true, ...ladder() },
  };
  const url = (fit?: Parameters<typeof paintStyle>[3]) => paintStyle(p, false, undefined, fit).match(/url\("([^"]+)"\)/)?.[1];
  it('on the profile, a shop card and a chat line at dpr 2', () => {
    expect(url(fitOfText(28, 12, 2))).toBe('https://cdn.cyex.tv/cosmetics/paints/abc/2x.webp');
    expect(url(fitOfText(18, 20, 2))).toBe('https://cdn.cyex.tv/cosmetics/paints/abc/2x.webp');
    expect(url(fitOfText(13, 8, 2))).toBe('https://cdn.cyex.tv/cosmetics/paints/abc/1x.webp');
    expect(url(fitOfText(34, 25, 2))).toBe('https://cdn.cyex.tv/cosmetics/paints/abc/4x.webp');
  });
  it('without a size on a 2x screen, 2x', () => {
    expect(url({ dpr: 2 })).toBe('https://cdn.cyex.tv/cosmetics/paints/abc/2x.webp');
    expect(url()).toBe('https://cdn.cyex.tv/cosmetics/paints/abc/1x.webp');
  });
});
