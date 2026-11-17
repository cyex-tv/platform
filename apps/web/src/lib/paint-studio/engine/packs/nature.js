// @ts-nocheck
/* Pack: Nature and space: water, sky, earth, fire, cosmos. Same contracts as js/library.js; see js/packs/README.md. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, smoothstep, tri, wrap, hexToRgb, lutAt, hash2, vnoise, fbm, ridged, luma, hsvToRgb } = PL;
  const P = PL.P;
  const { rgb01, colorPrep, eachPx, bilinear, remap, boxBlur } = PL.fx;
  const add = (store, id, def) => { if (store[id]) throw new Error('Duplicate layer or effect id: ' + id); def.id = id; def.pack = 'nature'; store[id] = def; };
  const layer = (id, def) => add(PL.LAYERS, id, def);
  const effect = (id, def) => add(PL.EFFECTS, id, def);
  for (const c of ['Nature', 'Space']) if (!PL.LAYER_CATS.includes(c)) PL.LAYER_CATS.push(c);
  for (const c of []) if (!PL.EFFECT_CATS.includes(c)) PL.EFFECT_CATS.push(c);

  // ---- shared helpers ----
  const mix3 = (o, a, b, f) => { o[0] = a[0] + (b[0] - a[0]) * f; o[1] = a[1] + (b[1] - a[1]) * f; o[2] = a[2] + (b[2] - a[2]) * f; o[3] = 1; return o; };
  const wrapD = (d) => d - Math.round(d);
  const sq = (x) => x * x;
  // straight-alpha source r,g,b,a over a background colour with its own alpha
  const onBg = (o, bg, bgA, r, g, b, a) => {
    a = clamp(a);
    const oa = a + bgA * (1 - a);
    if (oa < 1e-5) { o[0] = bg[0]; o[1] = bg[1]; o[2] = bg[2]; o[3] = 0; return o; }
    const k = bgA * (1 - a);
    o[0] = clamp((r * a + bg[0] * k) / oa); o[1] = clamp((g * a + bg[1] * k) / oa); o[2] = clamp((b * a + bg[2] * k) / oa); o[3] = oa;
    return o;
  };
  // churn offset on a circle: p._cx, p._cy (cos, sin of one whole turn per loop at speed 1), cached per frame
  const churn = (t, p) => { if (p._ct !== t) { const a = TAU * t * p.speed; p._ct = t; p._cx = Math.cos(a); p._cy = Math.sin(a); } };
  // one candidate point per grid cell (wraps both ways); fn(dx, dy, h, h2, wx, wy) works in pixels from the point
  function cellScan(u, v, c, cell, density, seed, fn, near) {
    const nx = Math.max(1, Math.round(c.W / cell)), ny = Math.max(1, Math.round(c.H / cell));
    const gx = u * nx, gy = v * ny, ix = Math.floor(gx), iy = Math.floor(gy), cw = c.W / nx, ch = c.H / ny;
    let acc = 0;
    // near: shapes no wider than half a cell, so only the 2 x 2 closest cells can reach this pixel
    const si = near ? (gx - ix < 0.5 ? -1 : 1) : 0, sj = near ? (gy - iy < 0.5 ? -1 : 1) : 0;
    for (let jj = near ? 0 : -1; jj <= (near ? 1 : 1); jj++) for (let ii = near ? 0 : -1; ii <= (near ? 1 : 1); ii++) {
      const i = near ? ii * si : ii, j = near ? jj * sj : jj;
      const cx = ix + i, cy = iy + j, wx = wrap(cx, nx), wy = wrap(cy, ny);
      const h = hash2(wx, wy, seed);
      if (h > density) continue;
      const px = cx + hash2(wx, wy, seed + 1), py = cy + hash2(wx, wy, seed + 2);
      const r = fn((gx - px) * cw, (gy - py) * ch, h / density, hash2(wx, wy, seed + 3), wx, wy);
      if (r > acc) acc = r;
    }
    return acc;
  }
  const _sf = { t: 0, tw: 1 };
  const _sfFn = (dx, dy, h, h2) => {
    const s = 0.45 + h * 0.9, d = Math.hypot(dx, dy);
    if (d > s * 2.2) return 0;
    const k = d / s, tw = _sf.tw ? 0.55 + 0.45 * Math.sin(TAU * (_sf.t * _sf.tw + h2)) : 1;
    return Math.exp(-k * k * 2.4) * tw * (0.3 + 0.7 * h2);
  };
  const starField = (u, v, t, c, seed, cell, density, tw) => { _sf.t = t; _sf.tw = tw; return cellScan(u, v, c, cell, density, seed, _sfFn, true); };

  // =====================================================================================
  // Nature
  // =====================================================================================
  layer('oceanSwell', {
    name: 'Ocean swell', cat: 'Nature', desc: 'Rolling blue swells that travel across, with white foam on the crests.',
    tags: ['ocean', 'sea', 'waves', 'swell', 'foam', 'water', 'blue', 'surf'],
    params: [P.pal([{ at: 0, color: '#031a3a' }, { at: 0.45, color: '#0b4f86' }, { at: 0.78, color: '#1f93bf' }, { at: 1, color: '#a9e8f2' }]), P.color('foam', 'Foam', '#f2fbff'), P.int('waves', 'Swells', 1, 10, 3), P.range('foamAmt', 'Foam', 0, 1, 0.01, 0.55), P.range('chop', 'Chop', 0, 1, 0.01, 0.5), P.speed(1), P.seed(3)],
    prepare: colorPrep(['foam']),
    sample(u, v, t, p, o, c) {
      churn(t, p);
      const ph = TAU * t * p.speed, k = p.waves;
      const n = fbm(u * 8 + p._cx * 0.8, v * 4 + p._cy * 0.8, 8, 4, 3, p.seed), w = (n - 0.5) * p.chop;
      const h = 0.5 * Math.sin(TAU * (u * k + v * 0.35 + w * 0.25) - ph) + 0.3 * Math.sin(TAU * (u * k * 2 - v * 0.8 + w * 0.4) - ph) + 0.2 * Math.sin(TAU * (u * k * 3 + v * 1.9 + w * 0.6) - ph);
      const h01 = 0.5 + 0.5 * h;
      lutAt(p._stops, clamp(0.1 + h01 * 0.78 + (1 - v) * 0.1), o);
      const th = 1 - p.foamAmt * 0.38;
      const nn = vnoise(u * 48 + p._cx * 1.5, v * 16 + p._cy * 1.5, 48, 16, p.seed + 5);
      const fm = smoothstep(0, 1, (h01 - th) / 0.12 + (nn - 0.5) * 1.4) * (p.foamAmt > 0 ? 1 : 0);
      o[0] = mix(o[0], p._foam[0], fm); o[1] = mix(o[1], p._foam[1], fm); o[2] = mix(o[2], p._foam[2], fm);
      return o;
    },
  });

  layer('pondRipples', {
    name: 'Pond ripples', cat: 'Nature', desc: 'Raindrops landing on a still pond, each sending out fading rings.',
    tags: ['rain', 'ripple', 'pond', 'water', 'drops', 'rings', 'calm', 'lake'],
    params: [P.color('bg', 'Water', '#0d4252'), P.color('ring', 'Ring', '#d6f6ff'), P.range('cell', 'Spacing (px)', 10, 60, 1, 24), P.range('density', 'Density', 0.1, 1, 0.01, 0.6), P.range('strength', 'Strength', 0.2, 1.5, 0.05, 1), P.speed(1, 'speed', 'Drops per loop', 1, 6), P.seed(8)],
    prepare(p) {
      colorPrep(['bg', 'ring'])(p);
      p._fn = (dx, dy, h, h2) => {
        const ph = fract(p._t * p.speed + h2), R = ph * p.cell * 0.95 * (0.6 + 0.4 * h), d = Math.hypot(dx, dy * 2), w = 0.9 + ph * 1.3, a = 1 - ph;
        if (Math.abs(d - R) > w * 4) return 0;
        return (Math.exp(-sq((d - R) / w)) + 0.5 * Math.exp(-sq((d - R * 0.72) / w))) * a * a;
      };
    },
    sample(u, v, t, p, o, c) {
      p._t = t;
      const a = clamp(cellScan(u, v, c, p.cell, p.density, p.seed, p._fn) * p.strength);
      const n = fbm(u * 10, v * 5, 10, 5, 2, p.seed + 2), b = 0.8 + 0.25 * n + 0.15 * v, bg = p._bg;
      o[0] = clamp(mix(bg[0] * b, p._ring[0], a)); o[1] = clamp(mix(bg[1] * b, p._ring[1], a)); o[2] = clamp(mix(bg[2] * b, p._ring[2], a)); o[3] = 1;
      return o;
    },
  });

  layer('sunsetSky', {
    name: 'Sunset sky', cat: 'Nature', desc: 'A glowing sunset gradient with lit clouds drifting past a low sun.',
    tags: ['sunset', 'sky', 'dusk', 'clouds', 'sun', 'evening', 'warm', 'horizon'],
    params: [P.pal('sunset'), P.color('cloudLight', 'Cloud light', '#ffd9b8'), P.color('cloudShade', 'Cloud shade', '#6b2f5e'), P.range('cover', 'Cloud cover', 0, 1, 0.01, 0.5), P.range('sun', 'Sun glow', 0, 1, 0.01, 0.6), P.range('sunX', 'Sun position', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Drift', 0, 4), P.seed(6)],
    prepare: colorPrep(['cloudLight', 'cloudShade']),
    sample(u, v, t, p, o, c) {
      lutAt(p._stops, clamp(v * 0.95), o);
      const gx = wrapD(u - p.sunX) * c.A, gy = v - 0.9, glow = Math.exp(-(sq(gx * 0.5) + sq(gy * 1.7)) * 12) * p.sun;
      o[0] = clamp(o[0] + glow * 1.0); o[1] = clamp(o[1] + glow * 0.72); o[2] = clamp(o[2] + glow * 0.4);
      const sh = t * p.speed;
      const n = fbm((u - sh) * 6, v * 3.2, 6, 4, 4, p.seed), n2 = fbm((u - sh) * 6 + 0.18, v * 3.2 - 0.14, 6, 4, 4, p.seed);
      const th = 0.64 - p.cover * 0.3, m = smoothstep(th, th + 0.2, n) * smoothstep(0.02, 0.22, v);
      const lit = clamp(0.5 + (n - n2) * 7 + glow * 0.7 + (v - 0.5) * 0.4);
      const L = p._cloudLight, S = p._cloudShade;
      o[0] = mix(o[0], mix(S[0], L[0], lit), m * 0.92); o[1] = mix(o[1], mix(S[1], L[1], lit), m * 0.92); o[2] = mix(o[2], mix(S[2], L[2], lit), m * 0.92);
      o[3] = 1;
      return o;
    },
  });

  layer('sandDunes', {
    name: 'Sand dunes', cat: 'Nature', desc: 'Wind-blown dunes with sharp lee faces and fine ripples that creep along.',
    tags: ['sand', 'dunes', 'desert', 'wind', 'ripples', 'beige', 'warm', 'sahara'],
    params: [P.pal([{ at: 0, color: '#6b3f1e' }, { at: 0.4, color: '#c98b4a' }, { at: 0.75, color: '#efc27f' }, { at: 1, color: '#fff0c8' }]), P.int('dunes', 'Dunes', 1, 10, 3), P.range('ripples', 'Ripples', 4, 40, 1, 18), P.range('wind', 'Ripple depth', 0, 1, 0.01, 0.6), P.speed(1, 'drift', 'Dune drift', 0, 4), P.speed(1), P.seed(2)],
    sample(u, v, t, p, o, c) {
      const nv = (fbm(u * 4, v * 3, 4, 3, 3, p.seed) - 0.5) * 0.35;
      const s = fract(u * p.dunes + nv + v * 0.25 - t * p.drift);
      const lit = mix(0.45, -0.85, smoothstep(0.7, 0.75, s)), prof = s < 0.72 ? s / 0.72 : (1 - s) / 0.28;
      const rv = 0.5 + 0.5 * Math.sin(TAU * (v * p.ripples + nv * 1.4 - t * p.speed));
      const f = 0.5 + lit * 0.22 + (prof - 0.5) * 0.25 + (rv - 0.5) * 0.3 * p.wind * (lit > 0 ? 1 : 0.35);
      return lutAt(p._stops, clamp(f), o);
    },
  });

  layer('lavaCrust', {
    name: 'Lava crust', cat: 'Nature', desc: 'A slow lava flow: dark cooling plates floating on glowing molten rock.',
    tags: ['lava', 'magma', 'volcano', 'crust', 'molten', 'flow', 'hot', 'rock'],
    params: [P.pal([{ at: 0, color: '#3a0500' }, { at: 0.35, color: '#b3200a' }, { at: 0.65, color: '#ff6a10' }, { at: 1, color: '#ffe27a' }]), P.color('crust', 'Crust', '#1c1311'), P.int('sx', 'Scale x', 2, 16, 6), P.int('sy', 'Scale y', 1, 6, 3), P.range('crustAmt', 'Crust', 0.2, 0.8, 0.01, 0.52), P.range('churn', 'Churn', 0, 2, 0.05, 0.5), P.speed(1, 'speed', 'Flow', -4, 4), P.seed(4)],
    prepare: colorPrep(['crust']),
    sample(u, v, t, p, o, c) {
      churn(t, p);
      const x = (u - t * p.speed) * p.sx, n = fbm(x + p._cx * p.churn, v * p.sy + p._cy * p.churn, p.sx, p.sy, 4, p.seed);
      const r = ridged(x * 2 + 0.5, v * p.sy * 2, p.sx * 2, p.sy * 2, 3, p.seed + 3);
      const th = p.crustAmt, m = smoothstep(th - 0.05, th + 0.05, n), rim = Math.exp(-sq((n - th) / 0.045));
      lutAt(p._stops, clamp(0.3 + (1 - m) * 0.4 + rim * 0.35 + (r - 0.5) * 0.3 * (1 - m)), o);
      const cr = 0.5 + 1.1 * fbm(x * 3, v * p.sy * 3, p.sx * 3, p.sy * 3, 2, p.seed + 8), k = m * (1 - rim * 0.8);
      o[0] = mix(o[0], p._crust[0] * cr, k); o[1] = mix(o[1], p._crust[1] * cr, k); o[2] = mix(o[2], p._crust[2] * cr, k);
      return o;
    },
  });

  layer('magmaCracks', {
    name: 'Magma cracks', cat: 'Nature', desc: 'Dark cooled rock split by glowing cracks that pulse with heat.',
    tags: ['magma', 'cracks', 'lava', 'glow', 'volcanic', 'rock', 'ember', 'fissure'],
    params: [P.pal([{ at: 0, color: '#2a0400' }, { at: 0.4, color: '#c2310a' }, { at: 0.75, color: '#ff8a1f' }, { at: 1, color: '#fff0a0' }]), P.color('rock', 'Rock', '#17100e'), P.int('cx', 'Columns', 3, 40, 12), P.int('cy', 'Rows', 1, 12, 3), P.range('width', 'Crack width', 0.02, 0.4, 0.01, 0.1), P.range('jitter', 'Jitter', 0, 1, 0.01, 0.9), P.speed(1, 'speed', 'Pulse'), P.seed(9)],
    prepare: colorPrep(['rock']),
    sample(u, v, t, p, o, c) {
      const gx = u * p.cx, gy = v * p.cy, ix = Math.floor(gx), iy = Math.floor(gy), kx = c.W / p.cx, ky = c.H / p.cy, k = Math.min(kx, ky);
      let f1 = 1e9, f2 = 1e9, h1 = 0, h2 = 0;
      for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
        const cx = ix + i, cy = iy + j, wx = wrap(cx, p.cx), wy = wrap(cy, p.cy);
        const a = hash2(wx, wy, p.seed), b = hash2(wx, wy, p.seed + 1);
        const px = cx + 0.5 + (a - 0.5) * p.jitter, py = cy + 0.5 + (b - 0.5) * p.jitter;
        const d = Math.hypot((gx - px) * kx / k, (gy - py) * ky / k);
        if (d < f1) { f2 = f1; h2 = h1; f1 = d; h1 = a; } else if (d < f2) { f2 = d; h2 = a; }
      }
      const e = f2 - f1, pl = (h) => 0.65 + 0.35 * Math.sin(TAU * (t * p.speed + h * 3));
      const pulse = 0.5 * (pl(h1) + pl(h2)), g = clamp((Math.exp(-sq(e / p.width)) + 0.3 * Math.exp(-sq(e / (p.width * 3)))) * pulse);
      lutAt(p._stops, clamp(g * 1.1), o);
      const rk = 0.55 + 0.9 * h1, s = smoothstep(0, 0.25, g);
      o[0] = mix(p._rock[0] * rk, o[0], s); o[1] = mix(p._rock[1] * rk, o[1], s); o[2] = mix(p._rock[2] * rk, o[2], s); o[3] = 1;
      return o;
    },
  });

  layer('frostCrystals', {
    name: 'Frost crystals', cat: 'Nature', desc: 'Feathery ice crystals on cold glass with a few twinkling glints.',
    tags: ['frost', 'ice', 'crystal', 'winter', 'cold', 'snow', 'glass', 'freeze'],
    params: [P.pal([{ at: 0, color: '#0a2447' }, { at: 0.5, color: '#4f93c8' }, { at: 0.8, color: '#cfeaff' }, { at: 1, color: '#ffffff' }]), P.int('sx', 'Scale x', 2, 16, 6), P.int('sy', 'Scale y', 1, 5, 3), P.range('warp', 'Feathering', 0, 1, 0.01, 0.5), P.range('glints', 'Glints', 0, 1, 0.01, 0.5), P.speed(1), P.seed(12)],
    sample(u, v, t, p, o, c) {
      churn(t, p);
      const wx = (fbm(u * 5 + p._cx * 0.5, v * 3 + p._cy * 0.5, 5, 3, 2, p.seed + 3) - 0.5) * 0.5 * p.warp;
      const r = ridged((u + wx) * p.sx, (v + wx * 0.5) * p.sy, p.sx, p.sy, 3, p.seed);
      const fine = ridged(u * p.sx * 3 + wx * 3, v * p.sy * 3, p.sx * 3, p.sy * 3, 2, p.seed + 9);
      const f = smoothstep(0.2, 0.9, r * 0.85 + fine * 0.35 - 0.08);
      lutAt(p._stops, clamp(f), o);
      const g = starField(u, v, t, c, p.seed + 40, 10, 0.3, 2) * p.glints;
      o[0] = clamp(o[0] + g); o[1] = clamp(o[1] + g); o[2] = clamp(o[2] + g);
      return o;
    },
  });

  layer('lightShafts', {
    name: 'Underwater light', cat: 'Nature', desc: 'Soft shafts of sunlight slanting down through deep water with drifting plankton.',
    tags: ['underwater', 'light', 'rays', 'sea', 'god rays', 'deep', 'ocean', 'shafts'],
    params: [P.pal([{ at: 0, color: '#021a2e' }, { at: 0.5, color: '#05506e' }, { at: 1, color: '#1fb2c4' }]), P.color('light', 'Light', '#d8ffef'), P.int('count', 'Shafts', 1, 12, 5), P.range('slant', 'Slant', -1, 1, 0.01, 0.35), P.range('sharp', 'Sharpness', 1, 8, 0.1, 3), P.range('intensity', 'Intensity', 0, 2, 0.05, 1), P.speed(1, 'speed', 'Sway'), P.seed(5)],
    prepare: colorPrep(['light']),
    sample(u, v, t, p, o, c) {
      churn(t, p);
      const n = fbm(u * 6 + p._cx * 0.6, v * 3 + p._cy * 0.6, 6, 3, 3, p.seed);
      const x = u * p.count + v * p.slant + (n - 0.5) * 0.6 + 0.18 * p._cy;
      const beam = Math.pow(0.5 + 0.5 * Math.sin(TAU * x), p.sharp) * Math.pow(1 - v, 1.3) * p.intensity;
      lutAt(p._stops, clamp(0.12 + (1 - v) * 0.55 + n * 0.15), o);
      const sp = starField(u, v, t, c, p.seed + 20, 9, 0.35, 2) * 0.45;
      const k = clamp(beam * 0.7 + sp);
      o[0] = clamp(o[0] + p._light[0] * k); o[1] = clamp(o[1] + p._light[1] * k); o[2] = clamp(o[2] + p._light[2] * k);
      return o;
    },
  });

  layer('biolume', {
    name: 'Bioluminescence', cat: 'Nature', desc: 'Glowing plankton specks pulsing in the dark of the deep sea.',
    tags: ['bioluminescence', 'deep sea', 'plankton', 'glow', 'dark', 'ocean', 'specks', 'night'],
    params: [P.color('bg', 'Water', '#010a1a'), P.range('bgAlpha', 'Water opacity', 0, 1, 0.01, 1), P.color('glow', 'Glow', '#3df2ff'), P.color('glow2', 'Glow 2', '#7a5cff'), P.range('cell', 'Spacing (px)', 5, 40, 1, 12), P.range('density', 'Density', 0.05, 1, 0.01, 0.4), P.range('size', 'Size (px)', 0.5, 5, 0.1, 1.6), P.int('pulse', 'Pulses per loop', 1, 6, 2), P.seed(14)],
    prepare(p) {
      colorPrep(['bg', 'glow', 'glow2'])(p);
      p._fn = (dx, dy, h, h2) => {
        const ox = 3 * Math.cos(TAU * (p._t + h2 * 3)), oy = 2 * Math.sin(TAU * (p._t + h * 3));
        const d = Math.hypot(dx - ox, dy - oy), s = p.size * (0.5 + h), b = 0.3 + 0.7 * sq(Math.max(0, Math.sin(TAU * (p._t * p.pulse + h2))));
        const val = (Math.exp(-sq(d / s) * 2) + 0.25 * Math.exp(-sq(d / (s * 3)))) * b;
        if (val > p._bv) { p._bv = val; p._bh = h2; }
        return val;
      };
    },
    sample(u, v, t, p, o, c) {
      p._t = t; p._bv = 0; p._bh = 0;
      const a = clamp(cellScan(u, v, c, p.cell, p.density, p.seed, p._fn));
      const g1 = p._glow, g2 = p._glow2, f = p._bh;
      const bgl = 0.06 * fbm(u * 6, v * 3, 6, 3, 2, p.seed + 1);
      const bg = [p._bg[0] + bgl * 0.2, p._bg[1] + bgl * 0.8, p._bg[2] + bgl];
      return onBg(o, bg, p.bgAlpha, mix(g1[0], g2[0], f), mix(g1[1], g2[1], f), mix(g1[2], g2[2], f), a);
    },
  });

  layer('fireflies', {
    name: 'Fireflies', cat: 'Nature', desc: 'Fireflies blinking and wandering above a dark dusk meadow.',
    tags: ['fireflies', 'dusk', 'night', 'meadow', 'glow', 'summer', 'lightning bugs', 'blink'],
    params: [P.pal([{ at: 0, color: '#070f1c' }, { at: 0.6, color: '#12291f' }, { at: 1, color: '#1a3a22' }]), P.color('glow', 'Glow', '#e9ff6a'), P.range('cell', 'Spacing (px)', 8, 50, 1, 20), P.range('density', 'Density', 0.05, 1, 0.01, 0.5), P.range('size', 'Size (px)', 0.5, 5, 0.1, 1.5), P.int('blinks', 'Blinks per loop', 1, 6, 2), P.range('meadow', 'Meadow', 0, 1, 0.01, 0.6), P.seed(17)],
    prepare(p) {
      colorPrep(['glow'])(p);
      p._fn = (dx, dy, h, h2) => {
        const k = h > 0.5 ? 2 : 1, wx = p.cell * 0.32 * Math.sin(TAU * (p._t * k + h2)), wy = p.cell * 0.2 * Math.cos(TAU * (p._t + h * 2));
        const d = Math.hypot(dx - wx, dy - wy), s = p.size * (0.6 + h2 * 0.6);
        if (d > s * 9) return 0;
        const b = smoothstep(0.05, 0.7, 0.5 + 0.5 * Math.sin(TAU * (p._t * p.blinks + h2)));
        return (Math.exp(-sq(d / s) * 2) + 0.35 * Math.exp(-sq(d / (s * 3.5)))) * b;
      };
    },
    sample(u, v, t, p, o, c) {
      p._t = t;
      lutAt(p._stops, v, o);
      const ground = 0.9 - (fbm(u * 7, 1, 7, 4, 3, p.seed) - 0.3) * 0.28 * p.meadow;
      if (v > ground && p.meadow > 0) { const g = 0.25 + 0.5 * (1 - (v - ground) * 3); o[0] *= g * 0.6; o[1] *= g * 0.8; o[2] *= g * 0.6; }
      const a = clamp(cellScan(u, v, c, p.cell, p.density, p.seed, p._fn));
      o[0] = mix(o[0], p._glow[0], a); o[1] = mix(o[1], p._glow[1], a); o[2] = mix(o[2], p._glow[2], a); o[3] = 1;
      return o;
    },
  });

  layer('risingEmbers', {
    name: 'Rising embers', cat: 'Nature', desc: 'Glowing sparks lifting from a fire, swaying and cooling as they climb.',
    tags: ['embers', 'sparks', 'fire', 'campfire', 'rising', 'hot', 'glow', 'orange'],
    params: [P.pal('ember'), P.color('bg', 'Background', '#140603'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.range('cell', 'Spacing (px)', 5, 40, 1, 11), P.range('density', 'Density', 0.05, 1, 0.01, 0.5), P.range('size', 'Size (px)', 0.5, 5, 0.1, 1.4), P.range('sway', 'Sway', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Rise speed', 1, 6), P.seed(19)],
    prepare(p) {
      colorPrep(['bg'])(p);
      p._fn = (dx, dy, h, h2) => {
        const ox = p.sway * p.cell * 0.35 * Math.sin(TAU * (p._t * 2 + h2));
        const d = Math.hypot(dx - ox, dy * 0.8), s = p.size * (0.5 + h);
        const fl = 0.6 + 0.4 * Math.sin(TAU * (p._t * 3 + h2));
        return Math.exp(-sq(d / s) * 2) * fl;
      };
    },
    sample(u, v, t, p, o, c) {
      p._t = t;
      const a = cellScan(u, fract(v + t * p.speed), c, p.cell, p.density, p.seed, p._fn) * (0.3 + 0.7 * v);
      lutAt(p._stops, clamp(0.35 + a * 0.75), o);
      return onBg(o, p._bg, p.bgAlpha, o[0], o[1], o[2], clamp(a * 1.4));
    },
  });

  layer('inkBloom', {
    name: 'Ink bloom', cat: 'Nature', desc: 'Drops of ink blooming into curling clouds in clear water, then thinning away.',
    tags: ['ink', 'drop', 'water', 'bloom', 'cloud', 'diffuse', 'swirl', 'calligraphy'],
    params: [P.color('water', 'Water', '#e6f1f4'), P.color('ink', 'Ink', '#1b1470'), P.color('ink2', 'Ink 2', '#a3135f'), P.range('cell', 'Spacing (px)', 24, 100, 1, 64), P.range('density', 'Density', 0.1, 1, 0.01, 0.8), P.range('curl', 'Curl', 0, 1.5, 0.01, 0.8), P.speed(1, 'speed', 'Drops per loop', 1, 4), P.seed(23)],
    prepare: colorPrep(['water', 'ink', 'ink2']),
    sample(u, v, t, p, o, c) {
      const nx = Math.max(1, Math.round(c.W / p.cell)), ny = Math.max(1, Math.round(c.H / p.cell));
      if (p._ft !== t || p._nx !== nx || p._ny !== ny) { // per-frame drop table
        p._ft = t; p._nx = nx; p._ny = ny;
        const n = nx * ny; if (!p._d || p._d.length !== n * 6) p._d = new Float32Array(n * 6);
        for (let y = 0; y < ny; y++) for (let x = 0; x < nx; x++) {
          const q = (y * nx + x) * 6, h = hash2(x, y, p.seed), h2 = hash2(x, y, p.seed + 3), ph = fract(t * p.speed + h2);
          p._d[q] = h > p.density ? 0 : 1; p._d[q + 1] = hash2(x, y, p.seed + 1); p._d[q + 2] = hash2(x, y, p.seed + 2); p._d[q + 3] = h2;
          p._d[q + 4] = (0.15 + 0.85 * Math.sqrt(ph)) * p.cell * 0.8 * (0.6 + 0.4 * h / p.density); p._d[q + 5] = Math.pow(1 - ph, 1.4);
        }
      }
      const gx = u * nx, gy = v * ny, ix = Math.floor(gx), iy = Math.floor(gy), cw = c.W / nx, ch = c.H / ny, D = p._d;
      let best = 0, bh = 0;
      for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
        const cx = ix + i, cy = iy + j, q = (wrap(cy, ny) * nx + wrap(cx, nx)) * 6;
        if (!D[q]) continue;
        const dx = (gx - cx - D[q + 1]) * cw, dy = (gy - cy - D[q + 2]) * ch, R = D[q + 4], h2 = D[q + 3];
        const d0 = Math.hypot(dx, dy);
        if (d0 > R * (1 + 0.9 * p.curl) + 1) continue;
        const nz = 0.75 * vnoise(dx * 0.09 + h2 * 9 + 20, dy * 0.09 + D[q + 1] * 5 + 20, 64, 64, p.seed) + 0.25 * vnoise(dx * 0.25 + 3, dy * 0.25 + h2 * 7, 64, 64, p.seed + 1);
        const dens = smoothstep(R, R * 0.55, d0 + (nz - 0.5) * R * p.curl * 1.8) * D[q + 5] * (0.55 + 0.6 * nz);
        if (dens > best) { best = dens; bh = h2; }
      }
      const f = clamp(best * 1.3);
      o[0] = mix(p._water[0], mix(p._ink[0], p._ink2[0], bh), f); o[1] = mix(p._water[1], mix(p._ink[1], p._ink2[1], bh), f); o[2] = mix(p._water[2], mix(p._ink[2], p._ink2[2], bh), f); o[3] = 1;
      return o;
    },
  });

  layer('oilSlick', {
    name: 'Oil slick', cat: 'Nature', desc: 'A dark puddle with swirling rainbow thin-film colours that slowly shift.',
    tags: ['oil', 'slick', 'iridescent', 'rainbow', 'film', 'petrol', 'puddle', 'holographic'],
    params: [P.int('sx', 'Scale x', 1, 12, 4), P.int('sy', 'Scale y', 1, 6, 2), P.range('film', 'Film thickness', 0.3, 5, 0.05, 1.8), P.range('sat', 'Saturation', 0, 1, 0.01, 0.8), P.range('churn', 'Churn', 0, 2, 0.05, 0.9), P.speed(1), P.speed(1, 'shift', 'Colour shift'), P.seed(31)],
    sample(u, v, t, p, o, c) {
      churn(t, p);
      const n = fbm(u * p.sx + p._cx * p.churn, v * p.sy + p._cy * p.churn, p.sx, p.sy, 4, p.seed);
      const n2 = fbm(u * p.sx * 2 - p._cy * p.churn * 0.8, v * p.sy * 2 + p._cx * p.churn * 0.8, p.sx * 2, p.sy * 2, 3, p.seed + 5);
      const th = n * p.film + n2 * 0.4 + v * 0.25 - t * p.shift, b = 0.22 + 0.78 * smoothstep(0.2, 0.8, n);
      const r = 0.5 + 0.5 * Math.cos(TAU * th), g = 0.5 + 0.5 * Math.cos(TAU * (th + 0.33)), bl = 0.5 + 0.5 * Math.cos(TAU * (th + 0.67)), s = p.sat, gr = (r + g + bl) / 3;
      o[0] = clamp(0.04 + b * mix(gr, r, s)); o[1] = clamp(0.04 + b * mix(gr, g, s)); o[2] = clamp(0.06 + b * mix(gr, bl, s)); o[3] = 1;
      return o;
    },
  });

  layer('soapBubbles', {
    name: 'Soap bubbles', cat: 'Nature', desc: 'Iridescent soap bubbles drifting upward with bright highlights.',
    tags: ['bubbles', 'soap', 'iridescent', 'rainbow', 'float', 'thin film', 'foam', 'sphere'],
    params: [P.color('bg', 'Background', '#0e3b55'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.range('cell', 'Spacing (px)', 10, 60, 1, 26), P.range('density', 'Density', 0.1, 1, 0.01, 0.6), P.range('size', 'Size', 0.4, 1.6, 0.01, 1), P.range('sway', 'Sway', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Rise speed', 1, 5), P.seed(37)],
    prepare: colorPrep(['bg']),
    sample(u, v, t, p, o, c) {
      const nx = Math.max(1, Math.round(c.W / p.cell)), ny = Math.max(1, Math.round(c.H / p.cell));
      const gx = u * nx, gy = fract(v + t * p.speed) * ny, ix = Math.floor(gx), iy = Math.floor(gy), cw = c.W / nx, ch = c.H / ny;
      let ba = 0, br = 0, bg = 0, bb = 0;
      for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
        const cx = ix + i, cy = iy + j, wx = wrap(cx, nx), wy = wrap(cy, ny), h = hash2(wx, wy, p.seed);
        if (h > p.density) continue;
        const h2 = hash2(wx, wy, p.seed + 3), rad = p.cell * (0.18 + 0.2 * h2) * p.size;
        const dx = (gx - cx - hash2(wx, wy, p.seed + 1)) * cw - p.sway * p.cell * 0.25 * Math.sin(TAU * (t + h2)), dy = (gy - cy - hash2(wx, wy, p.seed + 2)) * ch;
        const d = Math.hypot(dx, dy), q = d / rad;
        if (q > 1.3) continue;
        const rim = Math.exp(-sq((q - 1) / 0.12)), body = q < 1 ? 0.06 + 0.2 * Math.pow(q, 3) : 0;
        const hl = Math.exp(-(sq(dx / rad + 0.4) * 10 + sq(dy / rad + 0.42) * 26));
        const hue = fract(q * 1.4 + h2 + 0.25 * dx / rad), fc = hsvToRgb(hue, 0.5, 1);
        const a = clamp(Math.max(body, rim * 0.85) + hl * 0.9);
        if (a > ba) { ba = a; br = mix(fc[0], 1, hl); bg = mix(fc[1], 1, hl); bb = mix(fc[2], 1, hl); }
      }
      return onBg(o, p._bg, p.bgAlpha, br, bg, bb, ba);
    },
  });

  layer('dappledLight', {
    name: 'Dappled light', cat: 'Nature', desc: 'Sunlight breaking through a forest canopy in shifting golden patches.',
    tags: ['forest', 'canopy', 'leaves', 'sunlight', 'dappled', 'green', 'trees', 'light'],
    params: [P.pal([{ at: 0, color: '#04180a' }, { at: 0.4, color: '#0e4a1c' }, { at: 0.7, color: '#4f9a2c' }, { at: 0.88, color: '#c9e66a' }, { at: 1, color: '#fff7c0' }]), P.int('sx', 'Scale x', 2, 20, 8), P.int('sy', 'Scale y', 1, 8, 3), P.range('light', 'Light amount', 0, 1, 0.01, 0.5), P.range('churn', 'Shimmer', 0, 2, 0.05, 0.8), P.speed(1), P.seed(41)],
    sample(u, v, t, p, o, c) {
      churn(t, p);
      const n = fbm(u * p.sx + p._cx * p.churn, v * p.sy + p._cy * p.churn, p.sx, p.sy, 4, p.seed);
      const n2 = fbm(u * p.sx * 2 - p._cy * p.churn, v * p.sy * 2 + p._cx * p.churn, p.sx * 2, p.sy * 2, 3, p.seed + 7);
      const th = 0.75 - p.light * 0.35, g = smoothstep(th, th + 0.12, n * 0.75 + n2 * 0.35), leaf = 0.15 + 0.35 * n2;
      return lutAt(p._stops, clamp(leaf * (1 - g) + g * (0.6 + 0.4 * smoothstep(th + 0.1, th + 0.3, n))), o);
    },
  });

  layer('grassBlades', {
    name: 'Grass blades', cat: 'Nature', desc: 'Blades of grass swaying in a rolling breeze, with open space above.',
    tags: ['grass', 'blades', 'meadow', 'wind', 'sway', 'green', 'lawn', 'field'],
    params: [P.pal([{ at: 0, color: '#0f3d12' }, { at: 0.5, color: '#3e9a2b' }, { at: 1, color: '#c5f06a' }]), P.color('sky', 'Sky', '#9fd8ff'), P.range('skyAlpha', 'Sky opacity', 0, 1, 0.01, 0), P.int('blades', 'Blades', 8, 160, 64), P.range('height', 'Height', 0.2, 1, 0.01, 0.75), P.range('sway', 'Sway', 0, 1, 0.01, 0.5), P.speed(1), P.seed(43)],
    prepare: colorPrep(['sky']),
    sample(u, v, t, p, o, c) {
      const N = p.blades, gx = u * N, ix = Math.floor(gx), hgt = 1 - v;
      let best = -1, bs = 0, bd = 0;
      for (let i = ix - 2; i <= ix + 2; i++) {
        const wi = wrap(i, N), hi = hash2(wi, 0, p.seed), dep = hash2(wi, 1, p.seed), bh = p.height * (0.55 + 0.45 * hi);
        if (hgt > bh) continue;
        const s = hgt / bh, wind = Math.sin(TAU * (t * p.speed + u * 2 + hi * 0.15));
        const bend = ((hash2(wi, 2, p.seed) - 0.5) * 1.2 + p.sway * 1.6 * wind) * s * s;
        const dx = gx - (i + 0.5 + (hash2(wi, 3, p.seed) - 0.5) * 0.5 + bend), hw = 0.55 * (1 - s * 0.9);
        if (Math.abs(dx) < hw && dep > best) { best = dep; bs = s; bd = dx / hw; }
      }
      if (best < 0) return onBg(o, p._sky, p.skyAlpha, 0, 0, 0, 0);
      lutAt(p._stops, clamp(bs * 0.8 + 0.1 + best * 0.1 - Math.abs(bd) * 0.12), o);
      const sh = 0.55 + 0.45 * best; o[0] *= sh; o[1] *= sh; o[2] *= sh; o[3] = 1;
      return o;
    },
  });

  layer('thunderstorm', {
    name: 'Thunderstorm', cat: 'Nature', desc: 'Dark roiling storm clouds lit from within by flashes and the odd fork of lightning.',
    tags: ['storm', 'thunder', 'lightning', 'clouds', 'dark', 'flash', 'rain', 'weather'],
    params: [P.pal([{ at: 0, color: '#05090f' }, { at: 0.5, color: '#1b2a3d' }, { at: 1, color: '#5a6c86' }]), P.color('flashColor', 'Flash', '#d2e2ff'), P.int('flashes', 'Flashes per loop', 1, 4, 2), P.range('bolt', 'Lightning bolt', 0, 1, 0.01, 0.7), P.range('churn', 'Churn', 0, 2, 0.05, 0.7), P.speed(1, 'speed', 'Drift', 0, 4), P.seed(47)],
    prepare: colorPrep(['flashColor']),
    sample(u, v, t, p, o, c) {
      const F = p.flashes, ft = fract(t * F), idx = wrap(Math.floor(t * F), F);
      const flash = Math.exp(-sq((ft - 0.16) / 0.04)) + 0.6 * Math.exp(-sq((ft - 0.32) / 0.025));
      const n = fbm((u - t * p.speed) * 5 + Math.cos(TAU * t) * p.churn * 0.3, v * 3 + Math.sin(TAU * t) * p.churn * 0.3, 5, 4, 4, p.seed);
      const u0 = hash2(idx, 7, p.seed), du = wrapD(u - u0) * c.A;
      const near = 0.3 + 0.7 * Math.exp(-sq(du) * 0.5);
      lutAt(p._stops, clamp(n * 0.9 + (1 - v) * 0.1), o);
      const k = clamp(flash * near * (0.25 + n * 0.9));
      o[0] = clamp(o[0] + p._flashColor[0] * k); o[1] = clamp(o[1] + p._flashColor[1] * k); o[2] = clamp(o[2] + p._flashColor[2] * k);
      if (p.bolt > 0) {
        const bx = u0 + (vnoise(v * 9 + idx * 3.1, idx, 64, 64, p.seed + 2) - 0.5) * 0.12 + (v - 0.5) * 0.05;
        const bd = wrapD(u - bx) * c.A, b = Math.exp(-sq(bd / 0.03)) * flash * p.bolt;
        o[0] = clamp(o[0] + b); o[1] = clamp(o[1] + b); o[2] = clamp(o[2] + b);
      }
      o[3] = 1;
      return o;
    },
  });

  layer('rainbowMist', {
    name: 'Rainbow in mist', cat: 'Nature', desc: 'A soft rainbow arc seen through drifting mist, transparent around the arc.',
    tags: ['rainbow', 'mist', 'arc', 'fog', 'rain', 'spectrum', 'pastel', 'sky'],
    params: [P.color('mist', 'Mist', '#dcecf7'), P.range('mistAmt', 'Mist', 0, 1, 0.01, 0.35), P.range('radius', 'Arc radius', 0.4, 1.2, 0.01, 0.85), P.range('thick', 'Thickness', 0.04, 0.4, 0.01, 0.17), P.range('bright', 'Brightness', 0.2, 1.5, 0.01, 1), P.speed(1, 'speed', 'Drift', 0, 4), P.seed(53)],
    prepare: colorPrep(['mist']),
    sample(u, v, t, p, o, c) {
      const dx = (u - 0.5) * 2.2, dy = v - 1.2, d = Math.hypot(dx, dy), q = (d - p.radius) / p.thick + 0.5;
      const band = smoothstep(0, 0.12, q) * (1 - smoothstep(0.88, 1, q));
      const mist = fbm((u - t * p.speed) * 6, v * 3, 6, 3, 3, p.seed), m = 0.45 + 0.75 * mist;
      const rc = hsvToRgb(clamp(0.78 * (1 - clamp(q))), 0.7, 1);
      const a = clamp(band * m * p.bright), fa = clamp(p.mistAmt * mist * (0.4 + 0.6 * Math.exp(-sq((d - p.radius) / 0.45))));
      const oa = a + fa * (1 - a);
      if (oa < 1e-5) { o[0] = o[1] = o[2] = 0; o[3] = 0; return o; }
      const w = fa * (1 - a);
      o[0] = clamp((mix(rc[0], 1, 0.2) * a + p._mist[0] * w) / oa); o[1] = clamp((mix(rc[1], 1, 0.2) * a + p._mist[1] * w) / oa); o[2] = clamp((mix(rc[2], 1, 0.2) * a + p._mist[2] * w) / oa); o[3] = oa;
      return o;
    },
  });

  // =====================================================================================
  // Space
  // =====================================================================================
  layer('nebula', {
    name: 'Nebula', cat: 'Space', desc: 'Two-colour gas clouds with dust lanes and a sprinkling of stars.',
    tags: ['nebula', 'space', 'gas', 'clouds', 'cosmic', 'stars', 'pink', 'blue'],
    params: [P.color('bg', 'Space', '#04030f'), P.color('ca', 'Colour A', '#ff4fa3'), P.color('cb', 'Colour B', '#3a7bff'), P.int('sx', 'Scale x', 1, 12, 4), P.int('sy', 'Scale y', 1, 6, 2), P.range('warp', 'Swirl', 0, 3, 0.05, 1.4), P.range('density', 'Density', 0.2, 2, 0.05, 1), P.range('stars', 'Stars', 0, 1, 0.01, 0.5), P.speed(1), P.seed(61)],
    prepare: colorPrep(['bg', 'ca', 'cb']),
    sample(u, v, t, p, o, c) {
      churn(t, p);
      const q1 = fbm(u * p.sx + p._cx * 0.5, v * p.sy + p._cy * 0.5, p.sx, p.sy, 2, p.seed), q2 = fbm(u * p.sx - p._cy * 0.5 + 3, v * p.sy + p._cx * 0.5 + 7, p.sx, p.sy, 2, p.seed + 1);
      const n = fbm(u * p.sx + p.warp * q1, v * p.sy + p.warp * q2, p.sx, p.sy, 2, p.seed + 2);
      const hm = vnoise(u * p.sx + p.warp * q2 + 5, v * p.sy + p.warp * q1, p.sx, p.sy, p.seed + 3);
      const raw = smoothstep(0.28, 0.85, n) * p.density;
      const dust = raw > 0.02 ? smoothstep(0.55, 0.8, ridged(u * p.sx * 2, v * p.sy * 2 + p._cx * 0.3, p.sx * 2, p.sy * 2, 1, p.seed + 4)) : 0;
      const d = clamp(raw * (1 - 0.6 * dust)), core = Math.pow(d, 2.2);
      const s = d < 0.85 && p.stars > 0 ? starField(u, v, t, c, p.seed + 70, 8, 0.5, 2) * p.stars * (1 - d * 0.7) : 0;
      const sm = smoothstep(0.3, 0.7, hm);
      for (let k = 0; k < 3; k++) {
        const col = mix(p._ca[k], p._cb[k], sm);
        o[k] = clamp(p._bg[k] + col * d * 0.95 + core * 0.35 + s);
      }
      o[3] = 1;
      return o;
    },
  });

  layer('milkyWay', {
    name: 'Milky Way band', cat: 'Space', desc: 'A dense band of stars and glowing gas crossed by dark dust lanes.',
    tags: ['milky way', 'galaxy', 'stars', 'band', 'night sky', 'dust', 'cosmos', 'astro'],
    params: [P.pal([{ at: 0, color: '#03040c' }, { at: 0.35, color: '#1c2150' }, { at: 0.65, color: '#7d6fa6' }, { at: 1, color: '#ffe3c8' }]), P.int('bends', 'Bends', 1, 3, 1), P.range('width', 'Band width', 0.08, 0.6, 0.01, 0.22), P.range('stars', 'Stars', 0, 1.5, 0.01, 1), P.range('dust', 'Dust lanes', 0, 1, 0.01, 0.6), P.speed(1, 'speed', 'Drift', 0, 3), P.seed(67)],
    prepare(p) {
      p._fn = (dx, dy, h, h2) => {
        if (h > 0.04 + 0.96 * p._b) return 0;
        const s = 0.4 + h2 * 0.8, d = Math.hypot(dx, dy);
        if (d > s * 2.2) return 0;
        const tw = 0.6 + 0.4 * Math.sin(TAU * (p._t * 2 + h2 * 5));
        return Math.exp(-sq(d / s) * 2.4) * tw * (0.35 + 0.65 * h2);
      };
    },
    sample(u, v, t, p, o, c) {
      p._t = t;
      const b = v - (0.5 + 0.16 * Math.sin(TAU * (u * p.bends + 0.1))), dens = Math.exp(-sq(b / p.width));
      p._b = dens;
      const n = fbm((u - t * p.speed) * 6, v * 4, 6, 4, 3, p.seed);
      const r = ridged((u - t * p.speed) * 8, v * 5, 8, 5, 2, p.seed + 3), dust = smoothstep(0.5, 0.82, r) * Math.exp(-sq(b / (p.width * 0.8))) * p.dust;
      lutAt(p._stops, clamp(dens * (0.35 + n * 0.9) - dust * 0.6), o);
      const s = cellScan(u, v, c, 5, 0.8, p.seed + 11, p._fn, true) * p.stars * (1 - dust * 0.7);
      o[0] = clamp(o[0] + s); o[1] = clamp(o[1] + s); o[2] = clamp(o[2] + s * 0.95); o[3] = 1;
      return o;
    },
  });

  layer('blackHole', {
    name: 'Black hole', cat: 'Space', desc: 'A black hole with a swirling, hot accretion disc and a bright bent halo.',
    tags: ['black hole', 'accretion', 'disc', 'space', 'singularity', 'lensing', 'gravity', 'event horizon'],
    params: [P.pal([{ at: 0, color: '#2a0600' }, { at: 0.35, color: '#c23c08' }, { at: 0.7, color: '#ffb04a' }, { at: 1, color: '#fff3d0' }]), P.color('bg', 'Space', '#020208'), P.range('x', 'Position', 0.1, 0.9, 0.01, 0.5), P.range('size', 'Horizon size', 0.08, 0.3, 0.005, 0.16), P.range('tilt', 'Disc tilt', 0.08, 0.6, 0.01, 0.2), P.range('bright', 'Brightness', 0.3, 2, 0.05, 1), P.speed(1), P.seed(71)],
    prepare: colorPrep(['bg']),
    sample(u, v, t, p, o, c) {
      const dx = (u - p.x) * c.A, dy = v - 0.5, r = Math.hypot(dx, dy), rs = p.size;
      const yy = dy / p.tilt, dd = Math.hypot(dx, yy), ang = Math.atan2(yy, dx);
      const inner = rs * 1.3, outer = rs * 6;
      const disk = smoothstep(inner, inner + rs * 0.5, dd) * (1 - smoothstep(outer * 0.5, outer, dd));
      const sw = vnoise((ang / TAU - t * p.speed) * 10, dd / rs * 1.6, 10, 64, p.seed), fine = vnoise((ang / TAU - t * p.speed * 2) * 24, dd / rs * 3.5, 24, 64, p.seed + 2);
      const tex = 0.55 * sw + 0.45 * fine, heat = 1 - clamp((dd - inner) / (outer - inner));
      const dop = clamp(1 - 0.5 * dx / (dd + 1e-3), 0.4, 1.5);
      const diskI = disk * (0.4 + 0.9 * tex) * dop * (0.5 + heat);
      const halo = Math.exp(-sq((r - rs * 1.55) / (rs * 0.35))) * 0.55 * (0.5 + tex * 0.8), photon = Math.exp(-sq((r - rs * 1.07) / (rs * 0.06))) * 1.1;
      const front = dy > 0 && disk > 0.02;
      let I = diskI + halo + photon;
      if (r < rs && !front) I = photon * 0.2;
      I *= p.bright;
      lutAt(p._stops, clamp(I * 0.55 + heat * disk * 0.45), o);
      const st = starField(u, v, t, c, p.seed + 30, 8, 0.25, 2) * (1 - clamp(I)) * 0.8;
      const k = clamp(I * 1.1);
      for (let i = 0; i < 3; i++) o[i] = clamp(p._bg[i] + st + o[i] * k);
      o[3] = 1;
      return o;
    },
  });

  layer('gasGiant', {
    name: 'Gas giant', cat: 'Space', desc: 'Banded cloud belts on a gas giant, each shearing at its own speed, with a storm spot.',
    tags: ['planet', 'jupiter', 'gas giant', 'bands', 'storm', 'belts', 'space', 'saturn'],
    params: [P.pal([{ at: 0, color: '#5a3a24' }, { at: 0.2, color: '#a8693a' }, { at: 0.4, color: '#e3c08a' }, { at: 0.6, color: '#f6e6c4' }, { at: 0.8, color: '#c98a55' }, { at: 1, color: '#7a4a2a' }]), P.color('stormColor', 'Storm', '#b83a20'), P.int('bands', 'Bands', 4, 24, 12), P.range('turb', 'Turbulence', 0, 2, 0.05, 1), P.range('storm', 'Storm spot', 0, 1, 0.01, 0.8), P.speed(1), P.seed(73)],
    prepare: colorPrep(['stormColor']),
    sample(u, v, t, p, o, c) {
      const gb = v * p.bands, k = Math.floor(gb), fr = gb - k;
      const hk = hash2(k, 0, p.seed), hp = hash2(k - 1, 0, p.seed), hs = hash2(k, 1, p.seed);
      const sp = (hs < 0.5 ? -1 : 1) * (1 + (hash2(k, 2, p.seed) > 0.6 ? 1 : 0));
      const n = fbm((u - t * sp * p.speed) * 6, gb * 0.9 + hk * 5, 6, 64, 4, p.seed + k);
      const tone = mix(hp, hk, smoothstep(0, 0.2, fr));
      lutAt(p._stops, clamp(tone * 0.8 + 0.1 + (n - 0.5) * p.turb * 0.6), o);
      if (p.storm > 0) {
        const ex = (u - 0.72) * c.A / 0.5, ey = (v - 0.66) / 0.28, e = Math.hypot(ex, ey), a = Math.atan2(ey, ex);
        const sw = vnoise((a / TAU - t * p.speed) * 5, e * 3, 5, 16, p.seed + 9), spot = (1 - smoothstep(0.7, 1, e)) * p.storm, sc = 0.55 + 0.7 * sw;
        o[0] = mix(o[0], clamp(p._stormColor[0] * sc), spot); o[1] = mix(o[1], clamp(p._stormColor[1] * sc), spot); o[2] = mix(o[2], clamp(p._stormColor[2] * sc), spot);
      }
      o[3] = 1;
      return o;
    },
  });

  layer('sunGranules', {
    name: 'Sun surface', cat: 'Space', desc: 'Boiling granules on the surface of a star, crossed by cooler dark sunspots.',
    tags: ['sun', 'star', 'granulation', 'solar', 'surface', 'plasma', 'sunspot', 'hot'],
    params: [P.pal([{ at: 0, color: '#6a1400' }, { at: 0.4, color: '#e8560b' }, { at: 0.75, color: '#ffb327' }, { at: 1, color: '#fff4b0' }]), P.int('cx', 'Columns', 4, 48, 22), P.int('cy', 'Rows', 1, 12, 5), P.range('lanes', 'Dark lanes', 0, 1, 0.01, 0.6), P.range('spots', 'Sunspots', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Boil'), P.seed(79)],
    sample(u, v, t, p, o, c) {
      if (p._ft !== t) {
        p._ft = t; const n = p.cx * p.cy; if (!p._px || p._px.length !== n) { p._px = new Float32Array(n); p._py = new Float32Array(n); }
        for (let yy = 0; yy < p.cy; yy++) for (let xx = 0; xx < p.cx; xx++) {
          const h = hash2(xx, yy, p.seed), h3 = hash2(xx, yy, p.seed + 2), q = yy * p.cx + xx;
          p._px[q] = 0.5 + 0.35 * Math.sin(TAU * (t * p.speed + h)) + (h3 - 0.5) * 0.4; p._py[q] = 0.5 + 0.35 * Math.cos(TAU * (t * p.speed + h3)) + (hash2(xx, yy, p.seed + 1) - 0.5) * 0.4;
        }
      }
      const gx = u * p.cx, gy = v * p.cy, ix = Math.floor(gx), iy = Math.floor(gy), kx = c.W / p.cx, ky = c.H / p.cy, k = Math.min(kx, ky);
      let f1 = 1e9, f2 = 1e9;
      for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
        const cx = ix + i, cy = iy + j, q = wrap(cy, p.cy) * p.cx + wrap(cx, p.cx);
        const px = cx + p._px[q], py = cy + p._py[q];
        const d = Math.hypot((gx - px) * kx / k, (gy - py) * ky / k);
        if (d < f1) { f2 = f1; f1 = d; } else if (d < f2) f2 = d;
      }
      const ln = smoothstep(0, 0.28, f2 - f1), gr = 1 - clamp(f1 * 1.15);
      const spot = smoothstep(0.62, 0.74, fbm(u * 3 + Math.cos(TAU * t) * 0.2, v * 2, 3, 2, 2, p.seed + 4)) * p.spots;
      const f = clamp((gr * 0.55 + (ln * p.lanes + (1 - p.lanes)) * 0.5) * (1 - 0.7 * spot));
      return lutAt(p._stops, f, o);
    },
  });

  layer('solarCorona', {
    name: 'Solar corona', cat: 'Space', desc: 'A glowing sun disc with streaming rays of corona that slowly rotate.',
    tags: ['corona', 'sun', 'eclipse', 'rays', 'solar', 'streamers', 'glow', 'star'],
    params: [P.pal([{ at: 0, color: '#1a0500' }, { at: 0.35, color: '#c2410c' }, { at: 0.7, color: '#ffc063' }, { at: 1, color: '#ffffff' }]), P.color('bg', 'Space', '#030208'), P.range('x', 'Position', 0.1, 0.9, 0.01, 0.5), P.range('size', 'Disc size', 0.1, 0.5, 0.005, 0.24), P.int('rays', 'Streamers', 4, 24, 12), P.range('reach', 'Reach', 0.3, 2, 0.05, 1), P.speed(1), P.seed(83)],
    prepare: colorPrep(['bg']),
    sample(u, v, t, p, o, c) {
      const dx = (u - p.x) * c.A, dy = v - 0.5, r = Math.hypot(dx, dy), R = p.size, a = Math.atan2(dy, dx) / TAU;
      const s1 = vnoise((a + t * p.speed) * p.rays, r * 6, p.rays, 64, p.seed), s2 = vnoise((a - t * p.speed) * p.rays * 2, r * 10, p.rays * 2, 64, p.seed + 3);
      const st = 0.6 * s1 + 0.4 * s2;
      const cor = Math.exp(-Math.max(0, r - R) / (R * p.reach * (0.15 + 0.9 * st))) * (r >= R ? 1 : 0);
      const inside = 1 - smoothstep(R - 0.012, R, r), limb = 0.6 + 0.4 * Math.sqrt(Math.max(0, 1 - sq(r / R)));
      const gran = 0.9 + 0.2 * vnoise(dx * 40 + 100, dy * 40 + 100, 256, 256, p.seed + 6);
      const f = inside * clamp(limb * gran * 1.1) + (1 - inside) * clamp(cor * (0.35 + 0.65 * st) * 0.85);
      lutAt(p._stops, clamp(f), o);
      const k = clamp(f * 1.6);
      for (let i = 0; i < 3; i++) o[i] = clamp(p._bg[i] + o[i] * k);
      o[3] = 1;
      return o;
    },
  });

  layer('meteorShower', {
    name: 'Meteor shower', cat: 'Space', desc: 'Bright meteors streaking diagonally across a night sky in bursts, with faint stars.',
    tags: ['meteor', 'shooting star', 'shower', 'night', 'streak', 'comet', 'perseids', 'sky'],
    params: [P.color('bg', 'Sky', '#040716'), P.range('bgAlpha', 'Sky opacity', 0, 1, 0.01, 1), P.color('head', 'Head', '#ffffff'), P.color('tail', 'Tail', '#8fc4ff'), P.int('count', 'Meteors', 1, 24, 9), P.range('angle', 'Angle', 5, 70, 1, 28), P.range('length', 'Tail length (px)', 10, 120, 1, 55), P.range('stars', 'Stars', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Showers per loop', 1, 4), P.seed(89)],
    prepare(p) { colorPrep(['bg', 'head', 'tail'])(p); p._sl = Math.tan(p.angle * Math.PI / 180); },
    sample(u, v, t, p, o, c) {
      const x = u * c.W, y = v * c.H, W = c.W, H = c.H, L = p.length;
      let m = 0, hd = 0;
      for (let i = 0; i < p.count; i++) {
        const h = hash2(i, 0, p.seed), ph = fract(t * p.speed + h), env = smoothstep(0, 0.08, ph) * (1 - smoothstep(0.4, 0.55, ph));
        if (env <= 0) continue;
        const hx = fract(hash2(i, 1, p.seed) + ph * 0.6) * W, hy = (hash2(i, 2, p.seed) * 0.5 - 0.1) * H + ph * 0.6 * W * p._sl;
        let dx = x - hx; dx -= W * Math.round(dx / W);
        if (dx > 1.5 || dx < -L) continue;
        const err = y - (hy + p._sl * dx), tail = Math.max(0, 1 - (-dx) / L);
        const w = 0.5 + 0.35 * tail, s = Math.exp(-sq(err / w) * 1.5) * (dx > 0 ? Math.exp(-sq(dx / 1.2)) : tail * tail) * env;
        if (s > m) { m = s; hd = dx > -2.5 ? 1 : 0; }
      }
      const st = starField(u, v, t, c, p.seed + 50, 6, 0.3, 2) * p.stars * 0.8;
      const hc = p._head, tc = p._tail, f = clamp(m * 1.2) * 0.8 + (hd ? 0.2 : 0);
      return onBg(o, p._bg, p.bgAlpha, clamp(mix(tc[0], hc[0], f) * m + st), clamp(mix(tc[1], hc[1], f) * m + st), clamp(mix(tc[2], hc[2], f) * m + st), clamp(m + st));
    },
  });

  layer('moonCraters', {
    name: 'Moon craters', cat: 'Space', desc: 'A grey cratered lunar surface whose shadows swing round as the light moves.',
    tags: ['moon', 'craters', 'lunar', 'surface', 'grey', 'space', 'crater', 'regolith'],
    params: [P.pal([{ at: 0, color: '#1e1f24' }, { at: 0.4, color: '#5b5c63' }, { at: 0.75, color: '#a9a8a4' }, { at: 1, color: '#efede6' }]), P.int('cx', 'Craters across', 3, 24, 8), P.int('cy', 'Rows', 1, 6, 2), P.range('density', 'Density', 0.1, 1, 0.01, 0.7), P.range('depth', 'Relief', 0.2, 2, 0.05, 1), P.range('maria', 'Dark plains', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Light'), P.seed(97)],
    prepare(p) {
      p._cr = [];
      for (let s = 0; s < 2; s++) {
        const nx = p.cx * (s + 1), ny = p.cy * (s + 1), a = new Float32Array(nx * ny * 4);
        for (let y = 0; y < ny; y++) for (let x = 0; x < nx; x++) {
          const q = (y * nx + x) * 4, sd = p.seed + s * 50;
          a[q] = hash2(x, y, sd) <= p.density ? 1 : 0; a[q + 1] = 0.2 + hash2(x, y, sd + 1) * 0.6; a[q + 2] = 0.2 + hash2(x, y, sd + 2) * 0.6; a[q + 3] = 0.2 + 0.3 * hash2(x, y, sd + 3);
        }
        p._cr.push(a);
      }
    },
    sample(u, v, t, p, o, c) {
      const ang = TAU * (t * p.speed + 0.12), lx = Math.cos(ang), ly = Math.sin(ang);
      let sh = 0;
      for (let s = 0; s < 2; s++) {
        const cxn = p.cx * (s + 1), cyn = p.cy * (s + 1), gx = u * cxn, gy = v * cyn, ix = Math.floor(gx), iy = Math.floor(gy);
        const kx = c.W / cxn, ky = c.H / cyn, k = Math.min(kx, ky);
        for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
          const cx = ix + i, cy = iy + j, w = (wrap(cy, cyn) * cxn + wrap(cx, cxn)) * 4, cr = p._cr[s];
          if (!cr[w]) continue;
          const dxp = (gx - cx - cr[w + 1]) * kx, dyp = (gy - cy - cr[w + 2]) * ky;
          const rad = k * cr[w + 3], d = Math.hypot(dxp, dyp), q = d / rad;
          if (q > 1.5) continue;
          const dd = (dxp * lx + dyp * ly) / (d + 1e-3), rim = Math.exp(-sq((q - 1) / 0.1));
          if (q < 1) sh += (-dd * q * 0.6 - 0.1 * (1 - q)) * p.depth;
          sh += (0.1 + 0.28 * dd) * rim * p.depth + (q > 1 ? 0.05 * (1.5 - q) : 0);
        }
      }
      const base = 0.55 + (fbm(u * 12, v * 6, 12, 6, 3, p.seed) - 0.5) * 0.35 - smoothstep(0.55, 0.7, fbm(u * 3, v * 2, 3, 2, 3, p.seed + 7)) * 0.25 * p.maria;
      return lutAt(p._stops, clamp(base + sh * 0.6), o);
    },
  });

}
