// @ts-nocheck
/* Surprise me: random but tasteful paints built from the library.
   PL.Surprise.make(seed?)       -> an animated document (PL.Engine.blankDoc() shape)
   PL.Surprise.makeStatic(seed?) -> a static gradient paint
   Palettes come from a colour-harmony rule (analogous, complementary, triad, split complement) or from a
   built-in palette with its hue turned. Animated results are rendered at 64x16 and re-rolled a few times
   when they are too dark, too bright or mostly transparent, so the name stays readable.
   Nothing here touches the DOM, and PL.Engine, PL.LAYERS, PL.EFFECTS, PL.PALETTES are only used when called. */
export default function install(PL) {
  'use strict';
  const { clamp } = PL;

  // ---- seeded helpers ----
  const strSeed = (s) => { let h = 2166136261; for (const ch of String(s)) h = Math.imul(h ^ ch.charCodeAt(0), 16777619); return h >>> 0; };
  function kit(seed) {
    const s = seed == null ? (Math.random() * 4294967296) >>> 0 : typeof seed === 'string' ? strSeed(seed) : seed >>> 0;
    const f = PL.rng(s || 1); f(); f(); f();
    return {
      f,
      between: (a, b) => a + (b - a) * f(),
      int: (a, b) => a + Math.floor((b - a + 1) * f()),
      pick: (arr) => arr[Math.floor(f() * arr.length)],
      chance: (p) => f() < p,
      // weighted pick from [[weight, item], ...]
      wpick: (list) => { let t = 0; for (const [w] of list) t += w; let x = f() * t; for (const [w, it] of list) { x -= w; if (x <= 0) return it; } return list[list.length - 1][1]; },
    };
  }

  // ---- colour ----
  const hsl = (h, s, l) => { const [r, g, b] = PL.hslToRgb(((h % 1) + 1) % 1, clamp(s), clamp(l)); return PL.rgbToHex(r * 255, g * 255, b * 255); };
  const toHsl = (hex) => { const [r, g, b] = PL.hexToRgb(hex); return PL.rgbToHsl(r / 255, g / 255, b / 255); };
  const lumaOf = (hex) => { const [r, g, b] = PL.hexToRgb(hex); return PL.luma(r / 255, g / 255, b / 255); };
  // blues look darker than their lightness says, so they get a little more
  const blueLift = (h) => 0.08 * Math.max(0, 1 - Math.abs((((h % 1) + 1) % 1) - 0.66) / 0.1);

  // Hue offsets (turns) for each harmony rule.
  const HARMONY = {
    analogous: () => [0, 0.06, 0.12, -0.06, -0.12],
    complementary: () => [0, 0.5, 0.03, 0.53],
    triad: () => [0, 1 / 3, 2 / 3],
    split: () => [0, 0.42, 0.58],
  };
  const skipPalettes = new Set(['mono', 'peaks']);

  // A palette: { hexes (3-6 colours, ordered), stops (even gradient), dark, light, accent, hue, rule }
  function makePalette(r, count) {
    let hexes, rule;
    if (r.chance(0.3) && PL.PALETTES) {
      // a built-in palette with its hue turned (sometimes left alone); skip ones with see-through stops
      const keys = Object.keys(PL.PALETTES).filter((k) => !skipPalettes.has(k) && PL.PALETTES[k].every((s) => (s.a ?? 1) >= 1));
      const src = PL.PALETTES[r.pick(keys)], rot = r.chance(0.25) ? 0 : r.between(-0.5, 0.5);
      hexes = [];
      for (const s of src) {
        const [h, sat, l] = toHsl(s.color);
        const hx = hsl(h + rot, Math.max(sat, 0.45), clamp(l, 0.32, 0.9));
        if (!hexes.includes(hx)) hexes.push(hx);
      }
      if (hexes.length > 6) hexes = [0, 1, 2, 3, 4, 5].map((i) => hexes[Math.round((i / 5) * (hexes.length - 1))]);
      rule = 'palette';
    } else {
      rule = r.wpick([[3, 'analogous'], [3, 'complementary'], [2, 'triad'], [2, 'split']]);
      const base = r.f(), offs = HARMONY[rule]();
      const n = clamp(count || r.int(3, 5), 3, 6);
      const mood = r.wpick([[3, [0.5, 0.74, 0.66, 0.94]], [2, [0.38, 0.62, 0.55, 0.9]], [2, [0.7, 0.88, 0.45, 0.75]]]); // lightness lo, hi; saturation lo, hi
      const sat0 = r.between(mood[2], mood[3]);
      hexes = [];
      for (let i = 0; i < n; i++) {
        const h = base + offs[i % offs.length] + (i >= offs.length ? 0.02 * i : 0);
        const l = mood[0] + (mood[1] - mood[0]) * (n === 1 ? 0.5 : i / (n - 1)) + blueLift(h) + r.between(-0.03, 0.03);
        hexes.push(hsl(h, sat0 + r.between(-0.08, 0.08), clamp(l, 0.36, 0.9)));
      }
      if (r.chance(0.5)) hexes.reverse();
    }
    const hue = toHsl(hexes[0])[0];
    const byLight = [...hexes].sort((a, b) => lumaOf(a) - lumaOf(b));
    const accent = [...hexes].sort((a, b) => toHsl(b)[1] - toHsl(a)[1])[0];
    const n = hexes.length;
    return {
      hexes, rule, hue, accent,
      dark: hsl(toHsl(byLight[0])[0], 0.5, 0.16),
      light: hsl(toHsl(byLight[n - 1])[0], 0.55, 0.93),
      stops: hexes.map((c, i) => ({ at: Math.round((n === 1 ? 0 : i / (n - 1)) * 1000) / 1000, color: c, a: 1 })),
    };
  }
  // The gradient closed into a ring (first colour repeated at the end) so scrolling has no seam.
  const loopStops = (P) => {
    const all = [...P.hexes, P.hexes[0]];
    return all.map((c, i) => ({ at: Math.round((i / (all.length - 1)) * 1000) / 1000, color: c, a: 1 }));
  };
  // Weighted palette ({ colors: [{ c, a, w }], blend, loop }) with random strengths.
  const weighted = (r, hexes, loop = false) => ({ colors: hexes.map((c) => ({ c, a: 1, w: Math.round(r.between(0.6, 2.4) * 20) / 20 })), blend: 1, loop });

  // ---- names ----
  const ADJ = ['Velvet', 'Glass', 'Neon', 'Amber', 'Cobalt', 'Lunar', 'Solar', 'Frosted', 'Electric', 'Quiet', 'Golden', 'Crimson', 'Silent', 'Hazy', 'Mellow', 'Opal', 'Cosmic', 'Sunken', 'Gentle', 'Crystal', 'Midnight', 'Coral', 'Honey', 'Ivory', 'Violet', 'Emerald', 'Misty', 'Polar', 'Dusky', 'Slow', 'Paper', 'Copper', 'Silver', 'Indigo', 'Plush', 'Faded', 'Rosy', 'Wild', 'Bright', 'Saffron'];
  const NOUN = ['Comet', 'Harbor', 'Garden', 'Lantern', 'Meadow', 'Canyon', 'Echo', 'Tide', 'Orchard', 'Ember', 'Prism', 'Horizon', 'Drift', 'Bloom', 'Summit', 'Lagoon', 'Mirage', 'Cinder', 'Aurora', 'Compass', 'Voyage', 'Glacier', 'Parade', 'Carousel', 'Monsoon', 'Atlas', 'Mosaic', 'Thunder', 'Reverie', 'Harvest', 'Fable', 'Beacon', 'Dune', 'Pebble', 'Marble', 'Cascade', 'Skyline', 'Velour', 'Spark', 'Haven'];
  const makeName = (r) => r.pick(ADJ) + ' ' + r.pick(NOUN);

  // ---- animated documents ----
  const E = () => PL.Engine;
  const L = (type, params, over) => E().newLayer(type, Object.assign({ params }, over || {}));
  const F = (type, params) => E().newEffect(type, { params });
  const rd = (v, step = 0.01) => Math.round(v / step) * step;

  // Each base returns an array of layers. Every one moves (speed is never 0), and every one that scrolls
  // is built so it loops without a seam (mirrored, or a ring of colours).
  const BASES = [
    [3, (r, P) => [L('linear', { stops: loopStops(P), angle: r.pick([0, 45, 90, 135, 160, 180, 200]), count: r.pick([1, 1, 2]), speed: r.pick([1, 1, -1]) })]],
    [2, (r, P) => [L('linear', { stops: P.stops, angle: r.pick([20, 90, 120, 160]), count: r.pick([1, 2]), speed: 1, mirror: true })]],
    [2, (r, P) => [L('radial', { stops: P.stops, cx: rd(r.between(0.3, 0.7)), cy: 0.5, count: r.pick([1, 2]), speed: r.pick([1, -1]), mirror: true })]],
    [2, (r, P) => [L('conic', { stops: loopStops(P), cx: rd(r.between(0.3, 0.7)), count: r.pick([1, 2]), speed: r.pick([1, -1]) })]],
    [1, (r, P) => [L('diamond', { stops: P.stops, count: rd(r.between(1.5, 3), 0.25), speed: 1, mirror: true })]],
    [3, (r, P) => [L('mesh', { stops: weighted(r, P.hexes), soft: rd(r.between(1.6, 3), 0.1), move: rd(r.between(0.25, 0.45)), speed: 1 })]],
    [3, (r, P) => [L('clouds', { stops: weighted(r, P.hexes), sx: r.int(5, 10), sy: 2, contrast: rd(r.between(1.1, 1.8), 0.05), flow: rd(r.between(0.5, 1.1), 0.05), speed: 1 })]],
    [3, (r, P) => [L('plasma', { stops: weighted(r, P.hexes, true), freq: rd(r.between(1.5, 3), 0.25), cycles: r.pick([1, 1.5, 2]), speed: 1, shift: 1 })]],
    [2, (r, P) => [L('silk', { stops: weighted(r, P.hexes), warp: rd(r.between(1.2, 2.4), 0.05), sheen: rd(r.between(0.3, 0.6)), speed: 1 })]],
    [2, (r, P) => [L('liquid', { stops: P.stops, warp: rd(r.between(0.5, 0.9)), flow: 0.7, speed: 1 })]],
    [2, (r, P) => [L('waves', { stops: weighted(r, P.hexes), count: rd(r.between(1.5, 3), 0.5), amp: rd(r.between(0.08, 0.2)), freq: r.int(2, 3), speed: 1, mirror: true })]],
    [1, (r, P) => {
      const h = P.hexes, lt = P.light, md = h[Math.floor(h.length / 2)], dk = h[0];
      return [L('chrome', { stops: [{ at: 0, color: lt, a: 1 }, { at: 0.42, color: md, a: 1 }, { at: 0.5, color: dk, a: 1 }, { at: 0.56, color: lt, a: 1 }, { at: 1, color: md, a: 1 }], warp: 0.08, freq: r.int(2, 3), speed: 1 })];
    }],
    [1, (r, P) => [L('voronoi', { stops: weighted(r, P.hexes), mode: 'crystal', cx: r.int(10, 16), cy: 3, jitter: 0.9, edge: 0.03, edgeColor: P.light, speed: 1 })]],
    [1, (r, P) => [L('holo', { stops: weighted(r, P.hexes, true), angle: r.int(90, 150), scale: rd(r.between(1, 2), 0.25), noise: rd(r.between(0.3, 0.8)), speed: 1 })]],
    [1, (r, P) => [L('solid', { color: hsl(P.hue, 0.5, 0.22) }), L('aurora', { stops: weighted(r, P.hexes), height: rd(r.between(0.55, 0.8)), intensity: rd(r.between(1.3, 1.7), 0.05), speed: 1 }, { blend: 'screen' })]],
  ];

  // Overlays sit on top of the base with a light blend mode.
  const OVERLAYS = [
    ['sparkles', (r, P) => L('sparkles', { color: P.light, density: rd(r.between(0.15, 0.35)), size: rd(r.between(1.8, 3), 0.1), twinkle: r.int(2, 3) }, { blend: 'add', opacity: 0.9 })],
    ['glints', (r, P) => L('glints', { color: P.light, density: rd(r.between(0.3, 0.6)), rate: r.int(3, 5) }, { blend: 'add', opacity: rd(r.between(0.6, 0.8)) })],
    ['shine', (r, P) => L('shine', { color: P.light, angle: r.int(55, 75), width: rd(r.between(0.08, 0.14)), hold: rd(r.between(0.4, 0.55)), intensity: rd(r.between(0.6, 0.85)), double: r.chance(0.2) }, { blend: 'screen' })],
    ['particles', (r, P) => L('particles', { mode: r.pick(['snow', 'bubbles', 'embers']), color: P.light, density: rd(r.between(0.3, 0.5)), size: rd(r.between(1.2, 2), 0.1), speed: 1 }, { blend: 'screen' })],
    ['stars', (r, P) => L('stars', { color: '#ffffff', density: rd(r.between(0.3, 0.5)), speed: 1, twinkle: 2 }, { blend: 'screen' })],
    ['rays', (r, P) => L('rays', { color: P.light, count: r.int(8, 16), intensity: rd(r.between(0.3, 0.45)), speed: 1 }, { blend: 'screen', opacity: 0.8 })],
  ];

  const SHAPE_SOLO = ['heart', 'star', 'moon', 'sparkle', 'orb', 'drop', 'flower', 'diamond', 'crown', 'planet', 'sun'];
  const SHAPE_MANY = ['heart', 'star', 'sparkle', 'drop', 'flower', 'orb', 'diamond'];
  const SYMBOLS = ['✦', '★', '♥', '❄', '✿', '♪', '☾', '✧'];
  const elementPal = (r, P) => weighted(r, [P.light, P.accent, ...P.hexes.slice(0, 2)]);
  // Scene elements: one per paint, sometimes.
  const SCENE = [
    [3, (r, P) => { // one shape, floating
      const kind = r.pick(SHAPE_SOLO);
      return L('shape', { kind, fill: r.pick(['gradient', 'radial']), pal: elementPal(r, P), x: rd(r.between(0.2, 0.8)), y: 0.5, size: rd(r.between(0.5, 0.75)), motion: r.pick(['float', 'sway', 'bounce', 'orbit']), mspeed: 1, amount: rd(r.between(0.06, 0.14)), spin: kind === 'star' || kind === 'flower' ? r.pick([0, 1]) : 0, glow: rd(r.between(0.3, 0.5)), glowColor: P.light }, { name: 'Shape' });
    }],
    [3, (r, P) => L('shape', { kind: r.pick(SHAPE_MANY), fill: 'mixed', pal: weighted(r, [...P.hexes, P.light]), layout: 'row', count: r.int(3, 6), size: rd(r.between(0.35, 0.5)), sizeVar: 0.2, motion: 'bounce', amount: rd(r.between(0.12, 0.2)), stagger: 1, glow: 0.3, glowColor: P.light }, { name: 'Shapes' })],
    [3, (r, P) => L('shape', { kind: r.pick(SHAPE_MANY), fill: 'solid', pal: weighted(r, [P.light]), layout: 'scatter', count: r.int(8, 14), spread: 1.1, size: 0.3, sizeVar: 0.5, motion: r.pick(['rise', 'fall']), mspeed: 1, glow: 0.3, glowColor: P.light, seed: r.int(1, 999) }, { name: 'Shapes' })],
    [2, (r, P) => L('glyph', { text: r.pick(SYMBOLS), fill: 'solid', pal: weighted(r, [P.light]), layout: 'scatter', count: r.int(8, 14), spread: 1.1, size: 0.32, sizeVar: 0.5, motion: r.pick(['rise', 'fall', 'wander']), mspeed: 1, spin: r.pick([0, 1]), glow: 0.3, glowColor: P.light, seed: r.int(1, 999) }, { name: 'Symbols' })],
    [2, (r, P) => L('ribbon', { fill: r.pick(['along', 'mixed']), pal: weighted(r, [...P.hexes, P.light], true), lines: r.int(2, 5), y: 0.5, gap: rd(r.between(0.06, 0.1)), amp: rd(r.between(0.1, 0.25)), waves: r.int(2, 3), thick: rd(r.between(0.04, 0.08), 0.005), speed: 1, glow: rd(r.between(0.3, 0.6)), glowColor: P.accent }, { name: 'Ribbons' })],
    [1, (r, P) => L('mountains', { pal: weighted(r, [hsl(P.hue, 0.45, 0.38), hsl(P.hue, 0.5, 0.28), hsl(P.hue, 0.5, 0.18)]), ridges: r.int(2, 3), y: 0.6, height: 0.3, rough: r.int(3, 6), style: r.pick(['sharp', 'soft']), speed: 1, rim: 0.4, rimColor: P.light, seed: r.int(1, 999) }, { name: 'Mountains' })],
  ];

  // Effects: at most one lighting effect (gloss, tube, bevel), at most two in all.
  const EFFECTS = [
    [4, 'light', (r) => F('gloss', { strength: rd(r.between(0.2, 0.35)), curve: 0.15, soft: 0.04 })],
    [3, 'light', (r) => F('cylinder', { axis: 'h', strength: rd(r.between(0.3, 0.45)), hi: rd(r.between(0.35, 0.55)), pos: 0.28, width: 0.08 })],
    [1, 'light', (r) => F('bevel', { strength: 0.5, depth: 3, spin: 1, soft: 1 })],
    [3, 'glow', (r) => F('glow', { threshold: rd(r.between(0.55, 0.72)), radius: 2, strength: rd(r.between(0.5, 0.8), 0.05) })],
    [2, 'vignette', (r) => F('vignette', { amount: rd(r.between(0.25, 0.4)), size: 0.8 })],
    [1, 'hue', (r) => F('hue', { deg: 0, cycles: 1 })],
    [1, 'grain', (r) => F('grain', { amount: 0.05 })],
  ];

  function shadow(r, P) {
    if (!r.chance(0.35)) return [];
    if (r.chance(0.5)) return [{ x_offset: 0, y_offset: 1, radius: 0, color: PL.hexAlphaToDec(P.dark, 0.9) }];
    return [{ x_offset: 0, y_offset: 0, radius: r.pick([2, 3, 4]), color: PL.hexAlphaToDec(P.accent, rd(r.between(0.5, 0.7))) }];
  }

  function build(r) {
    const P = makePalette(r);
    const doc = E().blankDoc();
    doc.name = makeName(r);
    doc.layers.push(...r.wpick(BASES)(r, P));
    // 0-2 overlays, different kinds
    const nOver = r.wpick([[2, 0], [5, 1], [3, 2]]);
    const pool = [...OVERLAYS];
    for (let i = 0; i < nOver; i++) { const [, make] = pool.splice(Math.floor(r.f() * pool.length), 1)[0]; doc.layers.push(make(r, P)); }
    if (r.chance(0.35)) doc.layers.push(r.wpick(SCENE)(r, P));
    const nFx = r.wpick([[3, 0], [5, 1], [3, 2]]);
    const left = [...EFFECTS], used = new Set();
    for (let i = 0; i < nFx; i++) {
      const avail = left.filter(([, grp]) => !used.has(grp));
      if (!avail.length) break;
      const [, grp, make] = r.wpick(avail.map((x) => [x[0], x]));
      used.add(grp); doc.effects.push(make(r));
    }
    doc.shadows = shadow(r, P);
    return doc;
  }

  // Readability: render a 64x16 frame at three moments; the alpha-weighted mean luminance has to sit between
  // 0.25 and 0.92 and the paint has to be mostly opaque. Returns null when the document can't be rendered.
  const LO = 0.25, HI = 0.92;
  function measure(doc) {
    try {
      const d = Object.assign({}, doc, { width: 64, height: 16 });
      const cp = E().compile(d), n = E().frameCount(d);
      let sum = 0, wsum = 0, alpha = 0, total = 0;
      for (const f of [0, Math.floor(n / 3), Math.floor((n * 2) / 3)]) {
        const px = E().renderFrame(cp, f).data;
        for (let i = 0; i < px.length; i += 4) {
          const a = px[i + 3] / 255;
          sum += a * PL.luma(px[i] / 255, px[i + 1] / 255, px[i + 2] / 255); wsum += a; alpha += a; total++;
        }
      }
      return { luma: wsum ? sum / wsum : 0, alpha: alpha / total };
    } catch (e) { return null; }
  }
  const penalty = (m) => (!m ? Infinity : Math.max(0, LO - m.luma, m.luma - HI) + Math.max(0, 0.6 - m.alpha));

  const TRIES = 6;
  function make(seed) {
    const r = kit(seed);
    let best = null, bestPen = Infinity;
    for (let i = 0; i < TRIES; i++) {
      let doc;
      try { doc = build(r); } catch (e) { continue; }
      const pen = penalty(measure(doc));
      if (pen === 0) return doc;
      if (pen < bestPen || !best) { best = doc; bestPen = pen; }
    }
    if (best && bestPen < Infinity) return best;
    // nothing rendered: a plain scrolling gradient, which can't fail
    const P = makePalette(r, 4), doc = E().blankDoc();
    doc.name = makeName(r);
    doc.layers.push(L('linear', { stops: loopStops(P), angle: 90, speed: 1 }));
    return doc;
  }

  // ---- static gradient paints ----
  function makeStatic(seed) {
    const r = kit(seed);
    let P, n;
    for (let i = 0; i < TRIES; i++) {
      n = r.int(2, 6);
      P = makePalette(r, n);
      const avg = P.hexes.reduce((s, c) => s + lumaOf(c), 0) / P.hexes.length;
      if (avg >= LO && avg <= HI) break;
    }
    let hexes = P.hexes.slice(0, 6);
    if (hexes.length < 2) hexes = [hexes[0], P.light];
    const radial = r.chance(0.2), repeat = !radial && r.chance(0.15), banded = r.chance(0.12);
    let stops = [];
    const m = hexes.length;
    if (banded) { // hard colour bands
      hexes.forEach((c, i) => { stops.push({ at: i / m, color: c }, { at: (i + 1) / m, color: c }); });
    } else {
      // even spacing with a little jitter on the inner stops
      hexes.forEach((c, i) => {
        const base = i / (m - 1), jit = i === 0 || i === m - 1 ? 0 : r.between(-0.06, 0.06);
        stops.push({ at: clamp(base + jit), color: c });
      });
    }
    if (repeat) { // one tile of the pattern, closed on its own first colour so the repeat has no seam
      const len = r.pick([0.2, 0.25, 0.33, 0.5]);
      stops = stops.map((s) => ({ at: s.at * len, color: s.color }));
      stops.push({ at: len, color: stops[0].color });
    }
    stops = stops.map((s) => ({ at: Math.round(s.at * 1000) / 1000, color: PL.hexAlphaToDec(s.color, 1) }));
    const sh = shadow(r, P);
    return {
      name: makeName(r),
      function: radial ? 'RADIAL_GRADIENT' : 'LINEAR_GRADIENT',
      repeat,
      angle: radial ? 90 : r.chance(0.7) ? r.pick([0, 45, 90, 115, 135, 160, 180, 225, 270]) : r.int(0, 359),
      shape: radial ? r.pick(['circle', 'ellipse']) : 'circle',
      image_url: '',
      stops,
      shadows: sh,
    };
  }

  PL.Surprise = { make, makeStatic, makePalette: (seed, count) => makePalette(kit(seed), count), measure };
}
