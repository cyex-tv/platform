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
