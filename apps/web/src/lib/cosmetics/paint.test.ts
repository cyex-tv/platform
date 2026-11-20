import { describe, expect, it } from 'vitest';
import { contrast, luminance, paintColor, paintFloor, paintFilter, paintImage, paintRing, paintStyle, safeColor, safeUrl, type PaintLike } from './paint';

const base: PaintLike = { kind: 'LINEAR_GRADIENT', angle: 90, repeat: false, shape: 'CIRCLE', color: '#FF7A45', stops: [], shadows: [] };
const stops = [
  { at: 0, color: '#FF7A45' },
  { at: 1, color: '#8F5BFF' },
];
const img = { host: 'https://cdn.cyex.tv/cosmetics/paints/abc', animated: true, ready: true, files: [{ name: '1x.webp' }, { name: '1x_static.webp' }] };

describe('safeColor', () => {
  it('takes #RRGGBB and #RRGGBBAA only', () => {
    expect(safeColor('#FFaa00')).toBe('#ffaa00');
    expect(safeColor('#ffaa0080')).toBe('#ffaa0080');
    for (const bad of ['red', '#fff', '#ffaa0', 'ffaa00', '#ffaa00;color:red', '#ffaa00 ', 'url(x)', '', null, undefined])
      expect(safeColor(bad as string)).toBeNull();
  });
});

describe('safeUrl', () => {
  it('allows plain http(s) addresses only', () => {
    expect(safeUrl('https://cdn.cyex.tv/a/1x.webp')).toBe('https://cdn.cyex.tv/a/1x.webp');
    expect(safeUrl('javascript:alert(1)')).toBeNull();
    expect(safeUrl('data:image/png;base64,AAAA')).toBeNull();
    expect(safeUrl('https://x.test/a")b')).toBeNull();
    expect(safeUrl("https://x.test/a'b")).toBeNull();
    expect(safeUrl('https://x.test/a b')).toBeNull();
    expect(safeUrl('https://x.test/a\\b')).toBeNull();
  });
});

describe('paintImage', () => {
  it('draws linear gradients with the angle and stops', () => {
    expect(paintImage({ ...base, stops })).toBe('linear-gradient(90deg, #ff7a45 0%, #8f5bff 100%)');
    expect(paintImage({ ...base, angle: 45, repeat: true, stops })).toBe('repeating-linear-gradient(45deg, #ff7a45 0%, #8f5bff 100%)');
  });
  it('draws radial gradients with the shape', () => {
    expect(paintImage({ ...base, kind: 'RADIAL_GRADIENT', shape: 'ELLIPSE', stops })).toBe('radial-gradient(ellipse, #ff7a45 0%, #8f5bff 100%)');
    expect(paintImage({ ...base, kind: 'RADIAL_GRADIENT', stops })).toMatch(/^radial-gradient\(circle,/);
  });
  it('clamps angles and stop positions, and never lets a stop go backwards', () => {
    expect(paintImage({ ...base, angle: 999, stops })).toMatch(/^linear-gradient\(360deg/);
    expect(paintImage({ ...base, stops: [{ at: 0.6, color: '#000000' }, { at: 0.2, color: '#ffffff' }, { at: 4, color: '#ffffff' }] })).toBe(
      'linear-gradient(90deg, #000000 60%, #ffffff 60%, #ffffff 100%)',
    );
  });
  it('refuses a gradient with a bad colour or fewer than two stops', () => {
    expect(paintImage({ ...base, stops: [stops[0]] })).toBeNull();
    expect(paintImage({ ...base, stops: [stops[0], { at: 1, color: 'red);background:url(x)' }] })).toBeNull();
  });
  it('uses an image paint’s picture, its first frame when still', () => {
    const p: PaintLike = { ...base, kind: 'IMAGE', image: img };
    expect(paintImage(p)).toBe('url("https://cdn.cyex.tv/cosmetics/paints/abc/1x.webp")');
    expect(paintImage(p, true)).toBe('url("https://cdn.cyex.tv/cosmetics/paints/abc/1x_static.webp")');
    expect(paintImage({ ...p, image: { ...img, ready: false } })).toBeNull();
    expect(paintImage({ ...p, image: { ...img, host: 'javascript:alert(1)//' } })).toBeNull();
    expect(paintImage({ ...p, image: null })).toBeNull();
  });
});

describe('paintFilter', () => {
  it('caps shadows at four and clamps their values', () => {
    const s = { x: 40, y: -40, radius: 99, color: '#00000099' };
    const f = paintFilter({ shadows: [s, s, s, s, s] });
    expect(f.match(/drop-shadow/g)).toHaveLength(4);
    expect(f).toContain('drop-shadow(16px -16px 16px #00000099)');
  });
  it('drops shadows with a bad colour', () => {
    expect(paintFilter({ shadows: [{ x: 0, y: 1, radius: 2, color: 'red' }] })).toBe('');
  });
});

describe('paintStyle', () => {
  it('clips the paint to the text with a fallback colour first', () => {
    const css = paintStyle({ ...base, stops });
    expect(css.startsWith('color: #ff7a45')).toBe(true);
    expect(css).toContain('background-clip: text');
    expect(css).toContain('-webkit-text-fill-color: transparent');
  });
  it('keeps an image-painted name readable when its picture fails to load', () => {
    // Regression: the text is transparent and clipped to the background, so with only the picture
    // as background a blocked or missing picture drew nothing at all (an invisible name).
    for (const color of ['#FF7A45', null]) {
      const css = paintStyle({ ...base, kind: 'IMAGE', color, image: img });
      expect(css).toContain('-webkit-text-fill-color: transparent');
      const layers = css.match(/background-image: ([^;]+)/)![1];
      expect(layers).toBe('url("https://cdn.cyex.tv/cosmetics/paints/abc/1x.webp"), linear-gradient(currentColor, currentColor)');
      expect(css).toContain('background-size: cover, 100% 100%');
      if (color) expect(css.startsWith('color: #ff7a45')).toBe(true);
    }
    expect(paintStyle({ ...base, kind: 'IMAGE', repeat: true, image: img })).toContain('background-repeat: repeat, no-repeat');
  });
  it('falls back to the plain colour when the paint can’t be drawn', () => {
    expect(paintStyle({ ...base, stops: [] })).toBe('color: #ff7a45');
    expect(paintStyle({ ...base, color: null, stops: [] })).toBe('');
    expect(paintStyle(null)).toBe('');
  });
  it('never lets a bad value through', () => {
    const css = paintStyle({ ...base, color: '#fff;x:y', stops: [{ at: 0, color: '#000000' }, { at: 1, color: '#ffffff' }] });
    expect(css).not.toContain(';x:y');
    expect(css.startsWith('color: #000000')).toBe(true);
  });
});

describe('paintColor and paintRing', () => {
  it('uses the fallback colour, else the first stop', () => {
    expect(paintColor({ ...base, color: null, stops })).toBe('#ff7a45');
    expect(paintColor({ ...base, color: 'nope', stops: [] })).toBeNull();
  });
  it('rings in the gradient, or the plain colour for image paints', () => {
    expect(paintRing({ ...base, repeat: true, stops })).toBe('background: linear-gradient(90deg, #ff7a45 0%, #8f5bff 100%)');
    expect(paintRing({ ...base, kind: 'IMAGE', image: img })).toBe('background: #ff7a45');
    expect(paintRing(undefined)).toBe('');
  });
});

describe('readability floor', () => {
  const paint = (colors: string[]): PaintLike => ({ ...base, color: colors[0]!, stops: colors.map((color, i) => ({ at: i / Math.max(1, colors.length - 1), color })) });

  it('computes WCAG contrast', () => {
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 1);
    expect(contrast('#ffffff', '#ffffff')).toBeCloseTo(1, 5);
    expect(contrast('#777777', '#ffffff')).toBeCloseTo(4.48, 1);
    expect(luminance('#ffffff')).toBeCloseTo(1, 5);
  });
  it('blends a translucent colour onto the ground first', () => {
    expect(contrast('#00000000', '#ffffff')).toBeCloseTo(1, 5);
    expect(contrast('#000000ff', '#ffffff')).toBeCloseTo(21, 1);
  });
  it('flags pale paints for light and dark paints for dark', () => {
    expect(paintFloor(paint(['#ffb020', '#ff5a1f']))).toBe('light'); // Ember: pale amber
    expect(paintFloor(paint(['#ff7a45', '#ff3d7f', '#8f5bff']))).toBe('light'); // Sunset
    expect(paintFloor(paint(['#0b1d4a', '#101217']))).toBe('dark');
    expect(paintFloor(paint(['#0f7bd9', '#c2410c']))).toBeNull();
    expect(paintFloor(paint(['#fff3b0', '#10131a']))).toBe('both');
  });
  it('leaves a paint with shadows of its own alone', () => {
    const shadowed = { ...paint(['#ffb020', '#ff5a1f']), shadows: [{ x: 0, y: 1, radius: 2, color: '#00000099' }] };
    expect(paintFloor(shadowed)).toBeNull();
  });
  it('has no opinion about unknown colours', () => {
    expect(paintFloor(null)).toBeNull();
    expect(paintFloor({ ...base, kind: 'IMAGE', color: null, stops: [] })).toBeNull();
  });
});

describe('one renderer for every paint kind (site and paint studio)', () => {
  // Every kind the studio can save, drawn by the site, and by the studio's preview and CSS export.
  const kinds: [string, PaintLike][] = [
    ['linear', { ...base, stops }],
    ['repeating linear', { ...base, angle: 45, repeat: true, stops }],
    ['radial circle', { ...base, kind: 'RADIAL_GRADIENT', stops }],
    ['radial ellipse', { ...base, kind: 'RADIAL_GRADIENT', shape: 'ELLIPSE', stops }],
    ['repeating radial', { ...base, kind: 'RADIAL_GRADIENT', repeat: true, stops }],
    ['translucent stops', { ...base, stops: [{ at: 0, color: '#FFD54F' }, { at: 1, color: '#FF704300' }] }],
    ['shadows', { ...base, stops, shadows: [{ x: 0, y: 1, radius: 0, color: '#3B2500' }, { x: 0, y: 0, radius: 4, color: '#FFD70099' }] }],
    ['image (still)', { ...base, kind: 'IMAGE', image: { ...img, animated: false, files: [{ name: '1x.webp' }] } }],
    ['image (animated, rendered by the studio)', { ...base, kind: 'IMAGE', image: img }],
    ['repeating image', { ...base, kind: 'IMAGE', repeat: true, image: img }],
  ];
  it.each(kinds)('%s: the name is drawn with a background clipped to the text', (_, p) => {
    const css = paintStyle(p);
    expect(css).toContain('background-clip: text');
    expect(css).toContain('-webkit-text-fill-color: transparent');
    expect(css).toMatch(/background-image: (repeating-)?(linear|radial)-gradient\(|background-image: url\("https:\/\/cdn\.cyex\.tv\//);
  });
  it.each(kinds)('%s: the studio draws exactly what the site draws', async (_, p) => {
    const { paintStyleText } = await import('$lib/paint-studio/css');
    const studio = { name: 'x', ...p, color: p.color ?? null, stops: [...p.stops], shadows: [...p.shadows], image: p.kind === 'IMAGE' ? { id: 'i', url: `${img.host}/1x.webp` } : null };
    expect(paintStyleText(studio)).toBe(paintStyle(p));
  });
  it('previews a picture that is not on the CDN yet (the studio render) with the same declarations', () => {
    const p: PaintLike = { ...base, kind: 'IMAGE', image: null };
    const css = paintStyle(p, false, 'blob:http://localhost:5173/6f1c-42');
    expect(css).toContain('background-image: url("blob:http://localhost:5173/6f1c-42"), linear-gradient(currentColor, currentColor)');
    expect(paintStyle(p, false, 'javascript:alert(1)')).not.toContain('url(');
    expect(paintStyle(p, false, 'https://x.test/a")b')).toContain('url("https://x.test/a%22%29b")');
  });
});

describe('readability floor on a fixed ground (studio preview)', () => {
  it('gives a pale paint its halo on the light ground only', async () => {
    const { floorFilter, FLOOR_FILTER } = await import('./paint');
    const icy: PaintLike = { ...base, stops: [{ at: 0, color: '#D4F6FF' }, { at: 1, color: '#EFFCFF' }] };
    expect(floorFilter(icy, 'light')).toBe(FLOOR_FILTER.light);
    expect(floorFilter(icy, 'dark')).toBe('');
    expect(floorFilter({ ...base, stops: [{ at: 0, color: '#E8173A' }, { at: 1, color: '#D81B60' }] }, 'light')).toBe('');
  });
});

describe('a see-through fallback colour', () => {
  // Live "Icy Wave": an image paint whose fallback is #FF7A4500. With its picture blocked the
  // name drew in that colour, fully see-through, so it vanished.
  const icy: PaintLike = {
    ...base,
    kind: 'IMAGE',
    color: '#FF7A4500',
    image: { host: 'https://cdn.cyex.tv/cosmetics/paints/abc', animated: false, ready: true, files: [{ name: '1x.webp' }] },
  };

  it('is not used as the name colour', () => {
    expect(paintColor(icy)).toBeNull();
    expect(paintStyle(icy)).not.toMatch(/(^|; )color: /);
    // the layer under the picture then takes the surrounding text colour
    expect(paintStyle(icy)).toContain('linear-gradient(currentColor, currentColor)');
  });

  it('keeps colours that are mostly opaque', () => {
    expect(paintColor({ ...icy, color: '#FF7A45CC' })).toBe('#ff7a45cc');
    expect(paintColor({ ...icy, color: '#FF7A45' })).toBe('#ff7a45');
  });
});
