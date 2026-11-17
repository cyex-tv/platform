import { describe, expect, it } from 'vitest';
import { backgroundImage, paintCss, paintStyleText, shadowFilter } from './css';
import { fromApi, pictureUrl, toInput } from './backend';
import { searchItems, SIZES } from './anim';
import { alphaOf, blankPaint, checkPicture, fmtBytes, LIMITS, normHex, rgbaToHex, sortStops, validatePaint, withAlpha, type StudioPaint } from './format';
import { anyColour, engineShadows, exportPaint, hexToInt, importPaint, intToHex, projectDoc, toEngineShadows } from './interop';
import { STATIC_PRESETS } from './presets';

const paint = (o: Partial<StudioPaint> = {}): StudioPaint => ({ ...blankPaint(), name: 'Test', ...o });

describe('hex colours', () => {
  it('normalises short, long and alpha forms', () => {
    expect(normHex('#abc')).toBe('#AABBCC');
    expect(normHex('abcd')).toBe('#AABBCCDD');
    expect(normHex('#ff7a45ff')).toBe('#FF7A45');
    expect(normHex('#FF7A4580')).toBe('#FF7A4580');
    expect(normHex('red')).toBeNull();
    expect(normHex('#12345')).toBeNull();
  });
  it('sets and reads alpha', () => {
    expect(withAlpha('#FF0000', 0.5)).toBe('#FF000080');
    expect(withAlpha('#FF000080', 1)).toBe('#FF0000');
    expect(alphaOf('#00000000')).toBe(0);
    expect(rgbaToHex(255, 255, 255, 255)).toBe('#FFFFFF');
  });
});

describe('integer colours', () => {
  it('round-trips through packed signed 32-bit RGBA', () => {
    for (const hex of ['#BF2B2B', '#00000000', '#FFFFFF', '#7FFFFFFF'.slice(0, 7), '#12345678', '#FF000080']) expect(intToHex(hexToInt(hex))).toBe(hex);
  });
  it('matches known values', () => {
    expect(hexToInt('#BF2B2B')).toBe(-1087689729);
    expect(intToHex(255)).toBe('#000000');
    expect(intToHex(-1)).toBe('#FFFFFF');
    expect(hexToInt('#7FFFFFFF')).toBe(2147483647);
    expect(hexToInt('#80000000')).toBe(-2147483648);
  });
  it('reads numbers, numeric strings and hex', () => {
    expect(anyColour(-1087689729)).toBe('#BF2B2B');
    expect(anyColour('-1087689729')).toBe('#BF2B2B');
    expect(anyColour('#bf2b2b')).toBe('#BF2B2B');
    expect(anyColour('nope')).toBeNull();
  });
});

describe('limits', () => {
  it('passes a good paint', () => {
    expect(validatePaint(paint())).toEqual([]);
  });
  it('checks the name', () => {
    expect(validatePaint(paint({ name: '' }))[0].field).toBe('name');
    expect(validatePaint(paint({ name: 'x'.repeat(33) }))[0].message).toMatch(/32/);
    expect(validatePaint(paint({ name: 'a​b' }))[0].message).toMatch(/hidden/);
    expect(validatePaint(paint({ name: 'é'.repeat(32) }))).toEqual([]);
  });
  it('checks stops', () => {
    const one = validatePaint(paint({ stops: [{ at: 0, color: '#FFFFFF' }] }));
    expect(one.some((i) => i.field === 'stops' && /at least 2/.test(i.message))).toBe(true);
    const many = Array.from({ length: 11 }, (_, i) => ({ at: i / 10, color: '#FFFFFF' }));
    expect(validatePaint(paint({ stops: many })).find((i) => i.field === 'stops')?.message).toMatch(/Remove 1/);
    const back = validatePaint(paint({ stops: [{ at: 0.5, color: '#FFFFFF' }, { at: 0.2, color: '#000000' }] }));
    expect(back.find((i) => i.index === 1)?.message).toMatch(/before/);
    expect(validatePaint(paint({ stops: [{ at: 0, color: '#FFF' }, { at: 1, color: '#000000' }] }))[0].message).toMatch(/#RRGGBB/);
  });
  it('checks shadows, angle and the fallback colour', () => {
    const sh = { x: 0, y: 0, radius: 0, color: '#000000' };
    expect(validatePaint(paint({ shadows: [sh, sh, sh, sh, sh] }))[0].message).toMatch(/At most 4/);
    expect(validatePaint(paint({ shadows: [{ ...sh, x: 17 }] }))[0].message).toMatch(/−16 to 16/);
    expect(validatePaint(paint({ shadows: [{ ...sh, radius: -1 }] }))[0].message).toMatch(/0 to 16/);
    expect(validatePaint(paint({ angle: 361 }))[0].field).toBe('angle');
    expect(validatePaint(paint({ angle: 12.5 }))[0].field).toBe('angle');
    expect(validatePaint(paint({ kind: 'RADIAL_GRADIENT', angle: 999 }))).toEqual([]);
    expect(validatePaint(paint({ color: 'red' }))[0].field).toBe('color');
  });
  it('needs a picture for image paints, and no stops are checked', () => {
    expect(validatePaint(paint({ kind: 'IMAGE', stops: [], image: null }))[0].field).toBe('image');
    expect(validatePaint(paint({ kind: 'IMAGE', stops: [], image: { id: 'x', url: null } }))).toEqual([]);
  });
  it('checks pictures before upload', () => {
    expect(checkPicture({ bytes: 1000, width: 256, height: 64 })).toEqual([]);
    expect(checkPicture({ bytes: 1000, width: 256, height: 32 })[0]).toMatch(/64 px tall/);
    expect(checkPicture({ bytes: 1000, width: 1200, height: 300 })[0]).toMatch(/1\.000 px/);
    expect(checkPicture({ bytes: 6 * 1024 * 1024, width: 256, height: 64 })[0]).toMatch(/6,0 MB/);
  });
  it('keeps every preset and output size within the limits', () => {
    for (const p of STATIC_PRESETS) expect(validatePaint(p), p.name).toEqual([]);
    for (const s of SIZES) expect(checkPicture({ bytes: 1, width: s.width, height: s.height }), s.label).toEqual([]);
  });
  it('sorts stops stably and formats sizes', () => {
    expect(sortStops([{ at: 0.5, color: '#A' }, { at: 0.1, color: '#B' }, { at: 0.5, color: '#C' }]).map((s) => s.color)).toEqual(['#B', '#A', '#C']);
    expect(fmtBytes(1536)).toBe('1,5 KB');
    expect(fmtBytes(5 * 1024 * 1024)).toBe('5,0 MB');
    expect(LIMITS.stopsMax).toBe(10);
  });
});

describe('gradient CSS (through the site renderer)', () => {
  it('builds linear, repeating and radial gradients', () => {
    const p = paint({ angle: 45, stops: [{ at: 0, color: '#FF0000' }, { at: 0.333, color: '#00FF0080' }] });
    expect(backgroundImage(p)).toBe('linear-gradient(45deg, #ff0000 0%, #00ff0080 33.3%)');
    expect(backgroundImage({ ...p, repeat: true })).toMatch(/^repeating-linear-gradient\(45deg,/);
    expect(backgroundImage({ ...p, kind: 'RADIAL_GRADIENT', shape: 'ELLIPSE' })).toBe('radial-gradient(ellipse, #ff0000 0%, #00ff0080 33.3%)');
    expect(backgroundImage({ ...p, stops: [p.stops[0]] })).toBe('none');
  });
  it('draws pictures covering the name, or tiled with repeat, and quotes local URLs', () => {
    const img = paint({ kind: 'IMAGE', stops: [], image: { id: '1', url: 'https://cdn.example/a"b.webp' } });
    const s = paintStyleText(img);
    expect(s).toContain('background-image: url("https://cdn.example/a%22b.webp")');
    expect(s).toContain('background-size: cover');
    expect(paintStyleText({ ...img, repeat: true })).toContain('background-repeat: repeat;');
    expect(paintStyleText(img, 'blob:http://localhost/x')).toContain('url("blob:http://localhost/x")');
    expect(paintStyleText({ ...img, image: null })).not.toContain('background-image');
  });
  it('turns shadows into drop-shadow filters', () => {
    expect(shadowFilter({ shadows: [] })).toBe('none');
    expect(shadowFilter({ shadows: [{ x: 1, y: -2, radius: 3, color: '#00000099' }, { x: 0, y: 0, radius: 0, color: '#FFFFFF' }] })).toBe(
      'drop-shadow(1px -2px 3px #00000099) drop-shadow(0px 0px 0px #ffffff)',
    );
    expect(paintCss(paint())).toMatch(/background-clip: text;/);
  });
});

describe('import and export', () => {
  it('reads single-gradient JSON with integer colours', () => {
    const r = importPaint({ name: 'Old', function: 'LINEAR_GRADIENT', angle: 90, repeat: false, stops: [{ at: 0, color: -1087689729 }, { at: 1, color: 255 }], shadows: [{ x_offset: 0, y_offset: 1, radius: 2, color: 255 }] });
    expect(r.warnings).toEqual([]);
    expect(r.paint).toMatchObject({ name: 'Old', kind: 'LINEAR_GRADIENT', stops: [{ at: 0, color: '#BF2B2B' }, { at: 1, color: '#000000' }], shadows: [{ x: 0, y: 1, radius: 2, color: '#000000' }] });
    expect(validatePaint(r.paint)).toEqual([]);
  });
  it('round-trips through the integer export', () => {
    const p = paint({ name: 'Round', angle: 135, repeat: true, color: '#123456', shadows: [{ x: -3, y: 2, radius: 4, color: '#FF000080' }] });
    const back = importPaint(exportPaint(p)).paint;
    expect(back).toEqual({ ...p, image: null });
  });
  it('accepts the API wrapper, percent positions and hex strings', () => {
    const r = importPaint({ name: 'W', data: { function: 'radial-gradient', shape: 'ellipse', stops: [{ at: 0, color: '#fff' }, { at: 100, color: '#000000ff' }] } });
    expect(r.paint.kind).toBe('RADIAL_GRADIENT');
    expect(r.paint.shape).toBe('ELLIPSE');
    expect(r.paint.stops).toEqual([{ at: 0, color: '#FFFFFF' }, { at: 1, color: '#000000' }]);
  });
  it('brings out-of-range values into cyex.tv limits and says so', () => {
    const r = importPaint({
      name: 'x'.repeat(40),
      angle: 370,
      stops: Array.from({ length: 12 }, (_, i) => ({ at: i / 11, color: i % 2 ? 'zz' : '#ffffff' })),
      shadows: Array.from({ length: 6 }, () => ({ x_offset: 30, y_offset: 0, radius: 40, color: 255 })),
      gradients: undefined,
    });
    expect(r.paint.name).toHaveLength(32);
    expect(r.paint.angle).toBe(10);
    expect(r.paint.stops).toHaveLength(10);
    expect(r.paint.stops[0].at).toBe(0);
    expect(r.paint.stops[9].at).toBe(1);
    expect(r.paint.shadows).toHaveLength(4);
    expect(r.paint.shadows[0]).toMatchObject({ x: 16, radius: 16 });
    expect(validatePaint(r.paint)).toEqual([]);
    expect(r.warnings.join(' ')).toMatch(/32 characters.*370.*10 stops.*4 shadows.*limited.*colours weren’t valid/s);
  });
  it('keeps the first layer of a layered paint and drops text styles', () => {
    const r = importPaint({ name: 'L', gradients: [{ function: 'CONIC_GRADIENT', stops: [{ at: 0, color: 255 }, { at: 1, color: -1 }] }, { function: 'LINEAR_GRADIENT', stops: [] }], text: { weight: 7 } });
    expect(r.paint.kind).toBe('LINEAR_GRADIENT');
    expect(r.warnings.join(' ')).toMatch(/first of 2 layers.*Text styles.*Conic/s);
  });
  it('turns image paints into IMAGE with only safe links', () => {
    expect(importPaint({ function: 'URL', image_url: 'https://cdn.example/p.gif' }).paint).toMatchObject({ kind: 'IMAGE', stops: [], image: { id: null, url: 'https://cdn.example/p.gif' } });
    expect(importPaint({ function: 'URL', image_url: 'javascript:alert(1)' }).paint.image?.url).toBeNull();
  });
  it('moves backwards stops up and refuses non-objects', () => {
    const r = importPaint({ stops: [{ at: 0.5, color: 255 }, { at: 0.2, color: 255 }] });
    expect(r.paint.stops.map((s) => s.at)).toEqual([0.5, 0.5]);
    expect(() => importPaint('[1,2]')).toThrow(/paint/);
  });
  it('recognises project files and converts engine shadows', () => {
    expect(projectDoc({ cyexPaintProject: 1, doc: { layers: [] } })).toEqual({ layers: [] });
    expect(projectDoc({ name: 'x' })).toBeNull();
    const sh = [{ x: 1, y: 2, radius: 3, color: '#FF000080' }];
    expect(engineShadows(toEngineShadows(sh))).toEqual(sh);
  });
});

describe('API mapping', () => {
  const img = { id: 'img1', host: 'https://cdn.example/cosmetics/paints/abc/', ready: true, width: 256, height: 64, animated: true,
    files: [
      { name: '1x.webp', format: 'WEBP' as const, scale: 1, width: 256, height: 64, isStatic: false },
      { name: '4x.webp', format: 'WEBP' as const, scale: 4, width: 1024, height: 256, isStatic: false },
      { name: '4x_static.webp', format: 'WEBP' as const, scale: 4, width: 1024, height: 256, isStatic: true },
      { name: '4x.avif', format: 'AVIF' as const, scale: 4, width: 1024, height: 256, isStatic: false },
    ] };
  it('picks the largest animated WebP and its still', () => {
    expect(pictureUrl(img)).toBe('https://cdn.example/cosmetics/paints/abc/4x.webp');
    expect(pictureUrl(img, true)).toBe('https://cdn.example/cosmetics/paints/abc/4x_static.webp');
    expect(pictureUrl({ host: 'h', files: [] })).toBeNull();
  });
  it('maps both ways', () => {
    const saved = fromApi({ id: 'p1', name: 'Pic', kind: 'IMAGE', angle: 90, repeat: false, shape: 'CIRCLE', color: null, stops: [], shadows: [{ x: 0, y: 1, radius: 2, color: '#00000099' }], image: img, updatedAt: 'now' });
    expect(saved.paint.image).toEqual({ id: 'img1', url: 'https://cdn.example/cosmetics/paints/abc/4x.webp' });
    expect(toInput(saved.paint)).toEqual({ name: 'Pic', kind: 'IMAGE', angle: 90, repeat: false, shape: 'CIRCLE', color: null, stops: [], shadows: [{ x: 0, y: 1, radius: 2, color: '#00000099' }], imageId: 'img1' });
    const g = toInput(paint({ name: '  Sun  ', angle: 45 }));
    expect(g).toMatchObject({ name: 'Sun', angle: 45, imageId: null });
    expect(g.stops).toHaveLength(3);
  });
});

describe('engine', async () => {
  // The engine renders into ImageData; Node has none, so give it the minimal shape.
  if (typeof globalThis.ImageData === 'undefined') {
    (globalThis as unknown as { ImageData: unknown }).ImageData = class {
      data: Uint8ClampedArray;
      constructor(public width: number, public height: number) {
        this.data = new Uint8ClampedArray(width * height * 4);
      }
    };
  }
  const PL = (await import('./engine/index.js')).default;

  it('has the library and presets, and no code-running items', () => {
    expect(Object.keys(PL.LAYERS).length).toBeGreaterThan(140);
    expect(Object.keys(PL.EFFECTS).length).toBeGreaterThan(80);
    expect(PL.ANIM_PRESETS.length).toBeGreaterThan(30);
    expect(Object.keys(PL.Engine)).not.toContain('registerCustom');
  });
  it('renders a preset into frames that loop and are not blank', () => {
    const doc = PL.Engine.sanitizeDoc(PL.ANIM_PRESETS[PL.SCENE_COUNT]()).doc;
    doc.width = 64;
    doc.height = 16;
    const cp = PL.Engine.compile(doc);
    const a = PL.Engine.renderFrame(cp, 0);
    expect(a.width).toBe(64);
    expect(a.data.some((v, i) => i % 4 === 3 && v > 0)).toBe(true);
  });
  it('encodes a GIF with the right header and size', () => {
    const doc = PL.Engine.sanitizeDoc({ layers: [{ type: 'linear' }], width: 32, height: 8, fps: 10, duration: 0.4 }).doc;
    const cp = PL.Engine.compile(doc);
    const frames = Array.from({ length: cp.frames }, (_, i) => PL.Engine.renderFrame(cp, i));
    const gif = PL.Encode.encodeGif(frames, { delayCs: 10 });
    expect(gif.type).toBe('image/gif');
    expect(gif.size).toBeGreaterThan(30);
  });
  it('drops unknown items and custom code from documents', () => {
    const { doc, dropped } = PL.Engine.sanitizeDoc({ layers: [{ type: 'custom_x' }, { type: 'linear' }], custom: { custom_x: { kind: 'layer', code: 'o[0]=1' } } });
    expect(dropped).toBe(1);
    expect(doc.layers).toHaveLength(1);
    expect('custom' in doc).toBe(false);
  });
  it('searches the library by name, tags and synonyms', () => {
    expect(searchItems(PL, PL.LAYERS, 'fire')[0].name.toLowerCase()).toContain('fire');
    expect(searchItems(PL, PL.LAYERS, 'metal').length).toBeGreaterThan(1);
    expect(searchItems(PL, PL.LAYERS, 'zzzqqq')).toEqual([]);
  });
});
