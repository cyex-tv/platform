// @ts-nocheck
/* The layer and effect library for animated paints.
   Every layer is a per-pixel sampler: sample(u, v, t, p, out, c) with u, v in 0..1 across the paint,
   t in 0..1 across one loop, out = [r, g, b, a] (0..1, straight alpha). All motion uses whole numbers
   of cycles per loop, so every animation loops without a seam.
   Every effect runs on the composited float buffer: run(buf, W, H, t, p, c) -> buf. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, smoothstep, tri, wrap, hexToRgb, lutAt, hash2, vnoise, fbm, ridged, hslToRgb, rgbToHsl, luma } = PL;
  const { rgb01, colorPrep, eachPx, bilinear, remap, boxBlur, hueMatrix } = PL.fx;

  // ---------- palettes ----------
  const S = (...pairs) => pairs.map(([at, color, a = 1]) => ({ at, color, a }));
  const PALETTES = {
    rainbow: S([0, '#ff3b3b'], [0.17, '#ffb13b'], [0.33, '#f6ff3b'], [0.5, '#3bff7a'], [0.67, '#3bc4ff'], [0.83, '#8a3bff'], [1, '#ff3b3b']),
    fire: S([0, '#000000', 0], [0.25, '#7a0a00', 0.85], [0.5, '#ff4d00'], [0.75, '#ffb000'], [1, '#fff7c2']),
    ice: S([0, '#0b2a3a'], [0.35, '#2b6f86'], [0.65, '#9fe6ff'], [1, '#f2fdff']),
    icyLake: S([0, '#d4f6ff'], [0.15, '#4a8a8f'], [0.25, '#2b4a52'], [0.35, '#4a8a8f'], [0.45, '#d4f6ff'], [0.5, '#effcff'], [0.55, '#d4f6ff'], [0.65, '#4a8a8f'], [0.75, '#2b4a52'], [0.85, '#4a8a8f'], [1, '#d4f6ff']),
    gold: S([0, '#5c3a00'], [0.3, '#b8860b'], [0.5, '#ffe9a3'], [0.7, '#d4a017'], [1, '#6b4500']),
    chrome: S([0, '#ffffff'], [0.42, '#9fb2c6'], [0.5, '#1c232c'], [0.56, '#d0dae3'], [1, '#6d7d8c']),
    neon: S([0, '#ff00e6'], [0.5, '#00f0ff'], [1, '#ff00e6']),
    ocean: S([0, '#021a33'], [0.4, '#0a5c8a'], [0.75, '#2ec4d6'], [1, '#d9fbff']),
    toxic: S([0, '#0b1f00'], [0.5, '#39ff14'], [1, '#e6ff9e']),
    sunset: S([0, '#2b0f4a'], [0.35, '#c2185b'], [0.65, '#ff7043'], [1, '#ffd54f']),
    pastel: S([0, '#ffd1dc'], [0.25, '#fff5ba'], [0.5, '#c1f0c1'], [0.75, '#bde0fe'], [1, '#ffd1dc']),
    aurora: S([0, '#00ff9c'], [0.45, '#00c2ff'], [0.75, '#7b2cff'], [1, '#ff4fd8']),
    galaxy: S([0, '#05010f'], [0.35, '#2a0a5e'], [0.6, '#8e2de2'], [0.8, '#ff6ec7'], [1, '#ffffff']),
    lava: S([0, '#1a0000'], [0.4, '#a30000'], [0.7, '#ff5a00'], [1, '#ffe08a']),
    sakura: S([0, '#ffe4ef'], [0.5, '#ff9ec7'], [1, '#ffffff']),
    mono: S([0, '#000000'], [1, '#ffffff']),
    blood: S([0, '#1a0000'], [0.5, '#8b0000'], [1, '#ff3030']),
    mint: S([0, '#0f3d3e'], [0.5, '#3ddc97'], [1, '#e6fff4']),
    vapor: S([0, '#ff71ce'], [0.33, '#01cdfe'], [0.66, '#05ffa1'], [1, '#b967ff']),
    ember: S([0, '#000000', 0], [0.6, '#ff6a00', 0], [0.8, '#ffb347'], [1, '#ffffff']),
    rainbowLoop: S([0, '#ff3b3b'], [0.2, '#ffd23b'], [0.4, '#3bff7a'], [0.6, '#3bc4ff'], [0.8, '#a33bff'], [1, '#ff3b8a']),
    holo: S([0, '#ff9ad5'], [0.25, '#a5f3ff'], [0.5, '#c9a7ff'], [0.75, '#fff6a8'], [1, '#9dffc8']),
    opal: S([0, '#ffc2e6'], [0.25, '#b8f4ff'], [0.5, '#d4c2ff'], [0.75, '#fff0b8'], [1, '#bfffd6']),
    neonPop: S([0, '#ff00e6'], [0.33, '#00f0ff'], [0.66, '#fff200'], [1, '#00ff85']),
    night: S([0, '#2a3a8c'], [0.45, '#5a4fcf'], [0.8, '#a66de8'], [1, '#ffb3d9']),
    dusk: S([0, '#1a1036'], [0.5, '#6a2c70'], [0.8, '#f08a5d'], [1, '#ffd79a']),
    peaks: S([0, '#3a2f6b'], [0.5, '#24204a'], [1, '#0d0b1f']),
    candy: S([0, '#ff5fa2'], [0.33, '#ffd166'], [0.66, '#4dd6ff'], [1, '#b388ff']),
  };
  PL.PALETTES = PALETTES;

  // ---------- param builders ----------
  const P = {
    stops: (d = 'rainbow', k = 'stops', l = 'Colours') => ({ k, t: 'stops', l, d: PL.clone(PALETTES[d] || d) }),
    range: (k, l, min, max, step, d, hint) => ({ k, t: 'range', l, min, max, step, d, hint }),
    int: (k, l, min, max, d, hint) => ({ k, t: 'range', l, min, max, step: 1, d, hint }),
    speed: (d = 1, k = 'speed', l = 'Speed', min = -8, max = 8) => ({ k, t: 'range', l, min, max, step: 1, d, hint: 'Cycles per loop' }),
    color: (k, l, d) => ({ k, t: 'color', l, d }),
    select: (k, l, o, d) => ({ k, t: 'select', l, o, d }),
    bool: (k, l, d = false) => ({ k, t: 'bool', l, d }),
    seed: (d = 1) => ({ k: 'seed', t: 'range', l: 'Seed', min: 1, max: 999, step: 1, d }),
    image: () => ({ k: 'src', t: 'image', l: 'Image', d: '' }),
    pal: (d = 'rainbow', loop = false, k = 'stops', l = 'Colours') => ({ k, t: 'palette', l, d: PL.stopsToPalette(PALETTES[d] || d, loop) }),
    text: (k, l, d) => ({ k, t: 'text', l, d }),
  };
  PL.P = P;

  const LAYERS = {}, EFFECTS = {};
  const layer = (id, def) => { def.id = id; LAYERS[id] = def; };
  const effect = (id, def) => { def.id = id; EFFECTS[id] = def; };
  const setC = (o, c, a) => { o[0] = c[0]; o[1] = c[1]; o[2] = c[2]; o[3] = a; return o; };
  const mixC = (o, a, b, f, alpha = 1) => { o[0] = mix(a[0], b[0], f); o[1] = mix(a[1], b[1], f); o[2] = mix(a[2], b[2], f); o[3] = alpha; return o; };
  const proj = (u, v, deg, A = 1) => { const r = (deg * Math.PI) / 180, dx = Math.sin(r), dy = -Math.cos(r); return ((u - 0.5) * A * dx + (v - 0.5) * dy) / (Math.abs(dx) * A + Math.abs(dy)) + 0.5; };
  const wrapF = (f, doWrap, mirror) => (mirror ? tri(f) : doWrap ? fract(f) : clamp(f));

  // =====================================================================================
  // Gradients
  // =====================================================================================
  layer('linear', {
    name: 'Linear gradient', cat: 'Gradients', desc: 'Straight colour ramp at any angle, optionally scrolling.',
    params: [P.stops('rainbow'), P.range('angle', 'Angle', 0, 360, 1, 90), P.range('count', 'Repeats', 0.25, 12, 0.25, 1), P.speed(1), P.bool('mirror', 'Mirror')],
    sample(u, v, t, p, o, c) {
      const f = proj(u, v, p.angle, c.A) * p.count - t * p.speed;
      return lutAt(p._stops, wrapF(f, p.speed !== 0 || p.count !== 1, p.mirror), o);
    },
  });
  layer('radial', {
    name: 'Radial gradient', cat: 'Gradients', desc: 'Rings from a centre point; pulses outward with speed.',
    params: [P.stops('sunset'), P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.range('count', 'Repeats', 0.25, 12, 0.25, 1), P.speed(0), P.bool('mirror', 'Mirror'), P.bool('circle', 'Circle (keep aspect)', false)],
    sample(u, v, t, p, o, c) {
      const ax = p.circle ? c.A : 1;
      const dx = (u - p.cx) * ax, dy = v - p.cy;
      const R = Math.hypot(Math.max(p.cx, 1 - p.cx) * ax, Math.max(p.cy, 1 - p.cy)) || 1;
      const f = (Math.hypot(dx, dy) / R) * p.count - t * p.speed;
      return lutAt(p._stops, wrapF(f, p.speed !== 0 || p.count !== 1, p.mirror), o);
    },
  });
  layer('conic', {
    name: 'Conic gradient', cat: 'Gradients', desc: 'Sweeps around a centre, like a colour wheel. Speed spins it.',
    params: [P.stops('rainbow'), P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.int('count', 'Repeats', 1, 12, 1), P.speed(1), P.bool('mirror', 'Mirror')],
    sample(u, v, t, p, o, c) {
      const a = Math.atan2(v - p.cy, (u - p.cx) * c.A) / TAU + 0.5;
      return lutAt(p._stops, wrapF(a * p.count - t * p.speed, true, p.mirror), o);
    },
  });
  layer('diamond', {
    name: 'Diamond gradient', cat: 'Gradients', desc: 'Square rings around a centre point.',
    params: [P.stops('vapor'), P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.range('count', 'Repeats', 0.25, 12, 0.25, 2), P.speed(1), P.bool('mirror', 'Mirror', true)],
    sample(u, v, t, p, o, c) {
      const f = (Math.abs(u - p.cx) + Math.abs(v - p.cy)) * p.count - t * p.speed;
      return lutAt(p._stops, wrapF(f, true, p.mirror), o);
    },
  });
  layer('solid', {
    name: 'Solid colour', cat: 'Gradients', desc: 'One flat colour. Useful as a base or a tint.',
    params: [P.color('color', 'Colour', '#1e2a44'), P.range('alpha', 'Opacity', 0, 1, 0.01, 1)],
    prepare: colorPrep(['color']),
    sample(u, v, t, p, o, c) { return setC(o, p._color, p.alpha); },
  });
  layer('rainbow', {
    name: 'Colour cycle', cat: 'Gradients', desc: 'A looping band of colours that scrolls forever. Rainbow by default; set your own colours and how much of each.',
    params: [P.pal('rainbowLoop', true), P.range('angle', 'Angle', 0, 360, 1, 90), P.range('count', 'Repeats', 0.25, 8, 0.25, 1), P.speed(1)],
    sample(u, v, t, p, o, c) { return lutAt(p._stops, fract(proj(u, v, p.angle, c.A) * p.count - t * p.speed), o); },
  });

  // =====================================================================================
  // Patterns
  // =====================================================================================
  layer('stripes', {
    name: 'Stripes', cat: 'Patterns', desc: 'Candy or barber stripes at any angle.',
    params: [P.color('a', 'Colour A', '#ffffff'), P.color('b', 'Colour B', '#ff3b6b'), P.range('angle', 'Angle', 0, 360, 1, 45), P.range('count', 'Stripes', 1, 40, 1, 8), P.range('duty', 'Width', 0.05, 0.95, 0.01, 0.5), P.range('soft', 'Softness', 0, 0.5, 0.01, 0.04), P.speed(1), P.range('alphaB', 'Colour B opacity', 0, 1, 0.01, 1)],
    prepare: colorPrep(['a', 'b']),
    sample(u, v, t, p, o, c) {
      const g = fract(proj(u, v, p.angle, c.A) * p.count - t * p.speed), s = p.soft + 1e-4;
      const m = smoothstep(0, s, g) * (1 - smoothstep(p.duty, p.duty + s, g));
      const al = mix(p.alphaB, 1, m);
      return mixC(o, p._b, p._a, m, al);
    },
  });
  layer('checker', {
    name: 'Checker', cat: 'Patterns', desc: 'Checkerboard that can scroll in both directions.',
    params: [P.color('a', 'Colour A', '#ffffff'), P.color('b', 'Colour B', '#202020'), P.int('cx', 'Columns', 1, 64, 16), P.int('cy', 'Rows', 1, 32, 4), P.speed(1, 'sx', 'Scroll x'), P.speed(0, 'sy', 'Scroll y')],
    prepare: colorPrep(['a', 'b']),
    sample(u, v, t, p, o, c) {
      const x = Math.floor((u - t * p.sx) * p.cx), y = Math.floor((v - t * p.sy) * p.cy);
      return setC(o, (x + y) & 1 ? p._a : p._b, 1);
    },
  });
  layer('dots', {
    name: 'Dots', cat: 'Patterns', desc: 'Halftone dot grid; dots can pulse.',
    params: [P.color('dot', 'Dot colour', '#ffffff'), P.color('bg', 'Background', '#141414'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.int('cols', 'Columns', 1, 64, 20), P.int('rows', 'Rows', 1, 16, 4), P.range('radius', 'Radius', 0.05, 0.7, 0.01, 0.35), P.range('soft', 'Softness', 0, 0.3, 0.01, 0.05), P.speed(1, 'pulse', 'Pulse'), P.range('amount', 'Pulse amount', 0, 1, 0.01, 0.5), P.bool('wave', 'Pulse as a wave', true)],
    prepare: colorPrep(['dot', 'bg']),
    sample(u, v, t, p, o, c) {
      const gx = u * p.cols, gy = v * p.rows, ix = Math.floor(gx), iy = Math.floor(gy);
      const cw = c.W / p.cols, ch = c.H / p.rows, k = Math.min(cw, ch);
      const dx = (gx - ix - 0.5) * cw / k, dy = (gy - iy - 0.5) * ch / k;
      const ph = p.wave ? (ix / p.cols) : hash2(ix, iy, 3);
      const r = p.radius * (1 - p.amount * 0.5 + p.amount * 0.5 * Math.sin(TAU * (t * p.pulse - ph)));
      const m = 1 - smoothstep(r - p.soft, r + 1e-4, Math.hypot(dx, dy));
      return mixC(o, p._bg, p._dot, m, mix(p.bgAlpha, 1, m));
    },
  });
  layer('waves', {
    name: 'Waves', cat: 'Patterns', desc: 'Wavy colour bands that roll across.',
    params: [P.pal('ocean'), P.range('count', 'Bands', 0.5, 12, 0.5, 2), P.range('amp', 'Amplitude', 0, 0.5, 0.01, 0.12), P.int('freq', 'Frequency', 1, 12, 2), P.speed(1), P.speed(0, 'flow', 'Band flow'), P.bool('mirror', 'Mirror', true)],
    sample(u, v, t, p, o, c) {
      const f = (v + p.amp * Math.sin(TAU * (u * p.freq - t * p.speed))) * p.count - t * p.flow;
      return lutAt(p._stops, wrapF(f, true, p.mirror), o);
    },
  });
  layer('zigzag', {
    name: 'Zigzag', cat: 'Patterns', desc: 'Chevron stripes.',
    params: [P.color('a', 'Colour A', '#ffd54f'), P.color('b', 'Colour B', '#1b1b1b'), P.int('freq', 'Zigs', 1, 24, 8), P.range('amp', 'Depth', 0, 0.5, 0.01, 0.15), P.range('count', 'Stripes', 1, 16, 1, 3), P.range('duty', 'Width', 0.05, 0.95, 0.01, 0.5), P.speed(1)],
    prepare: colorPrep(['a', 'b']),
    sample(u, v, t, p, o, c) {
      const z = tri(u * p.freq) * p.amp;
      const g = fract((v + z) * p.count - t * p.speed);
      return setC(o, g < p.duty ? p._a : p._b, 1);
    },
  });
  layer('grid', {
    name: 'Grid lines', cat: 'Patterns', desc: 'Glowing grid, like a synthwave floor.',
    params: [P.color('line', 'Line colour', '#ff2bd6'), P.color('bg', 'Background', '#12002b'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.int('cols', 'Columns', 1, 64, 16), P.int('rows', 'Rows', 1, 16, 3), P.range('width', 'Line width', 0.01, 0.4, 0.01, 0.06), P.range('glow', 'Glow', 0, 1, 0.01, 0.4), P.speed(1, 'sx', 'Scroll x'), P.speed(0, 'sy', 'Scroll y')],
    prepare: colorPrep(['line', 'bg']),
    sample(u, v, t, p, o, c) {
      const fx = fract(u * p.cols - t * p.sx), fy = fract(v * p.rows - t * p.sy);
      const d = Math.min(Math.min(fx, 1 - fx), Math.min(fy, 1 - fy));
      const m = clamp(1 - smoothstep(p.width * 0.5, p.width * 0.5 + 0.01, d) + p.glow * Math.exp(-d * 12) * 0.8);
      return mixC(o, p._bg, p._line, m, mix(p.bgAlpha, 1, m));
    },
  });
  layer('carbon', {
    name: 'Carbon fibre', cat: 'Materials', desc: 'Woven fibre with a moving sheen.',
    params: [P.color('dark', 'Dark', '#0e0f12'), P.color('light', 'Light', '#4a4d57'), P.int('cols', 'Weave columns', 2, 80, 30), P.int('rows', 'Weave rows', 1, 20, 6), P.range('sheen', 'Sheen', 0, 1, 0.01, 0.5), P.speed(1)],
    prepare: colorPrep(['dark', 'light']),
    sample(u, v, t, p, o, c) {
      const gx = u * p.cols, gy = v * p.rows, ix = Math.floor(gx), iy = Math.floor(gy), fx = gx - ix, fy = gy - iy;
      const s = (ix + iy) & 1 ? Math.sin(Math.PI * fy) : Math.sin(Math.PI * fx);
      const band = Math.exp(-Math.pow((fract(u - t * p.speed) - 0.5) * 4, 2)) * p.sheen;
      const f = clamp(s * 0.55 + band * ((ix + iy) & 1 ? 0.6 : 1));
      return mixC(o, p._dark, p._light, f);
    },
  });

  // =====================================================================================
  // Noise and fluids
  // =====================================================================================
  // Per-frame churn offset, cached on p (t is constant for a whole frame): read p._fx, p._fy afterwards.
  const flowXY = (t, p) => { if (p._ft !== t) { const a = TAU * t * p.speed; p._ft = t; p._fx = Math.cos(a) * p.flow; p._fy = Math.sin(a) * p.flow; } };
  layer('clouds', {
    name: 'Clouds', cat: 'Noise and fluids', desc: 'Soft fractal noise that drifts and churns.',
    params: [P.pal('galaxy'), P.int('sx', 'Scale x', 1, 32, 8), P.int('sy', 'Scale y', 1, 16, 2), P.int('oct', 'Detail', 1, 6, 4), P.range('contrast', 'Contrast', 0.2, 4, 0.05, 1.4), P.range('flow', 'Churn', 0, 3, 0.05, 0.8), P.speed(1), P.speed(0, 'drift', 'Drift'), P.seed(1)],
    sample(u, v, t, p, o, c) {
      flowXY(t, p); const ox = p._fx, oy = p._fy;
      const n = fbm((u - t * p.drift) * p.sx + ox, v * p.sy + oy, p.sx, p.sy, p.oct, p.seed);
      return lutAt(p._stops, clamp((n - 0.5) * p.contrast + 0.5), o);
    },
  });
  layer('smoke', {
    name: 'Smoke', cat: 'Noise and fluids', desc: 'Ridged noise: wisps, veins and electric threads.',
    params: [P.pal('mono'), P.int('sx', 'Scale x', 1, 32, 6), P.int('sy', 'Scale y', 1, 16, 2), P.int('oct', 'Detail', 1, 6, 4), P.range('contrast', 'Contrast', 0.2, 4, 0.05, 1.6), P.range('flow', 'Churn', 0, 3, 0.05, 0.7), P.speed(1), P.speed(1, 'drift', 'Drift'), P.seed(7)],
    sample(u, v, t, p, o, c) {
      flowXY(t, p); const ox = p._fx, oy = p._fy;
      const n = ridged((u - t * p.drift) * p.sx + ox, v * p.sy + oy, p.sx, p.sy, p.oct, p.seed);
      return lutAt(p._stops, clamp((n - 0.5) * p.contrast + 0.5), o);
    },
  });
  layer('plasma', {
    name: 'Plasma', cat: 'Noise and fluids', desc: 'Classic demoscene plasma.',
    params: [P.pal('vapor', true), P.range('freq', 'Frequency', 0.5, 8, 0.25, 2), P.range('cycles', 'Colour repeats', 0.5, 6, 0.25, 1), P.speed(1), P.speed(1, 'shift', 'Colour shift')],
    sample(u, v, t, p, o, c) {
      const x = u * TAU * p.freq * Math.min(c.A, 4) / 2, y = v * TAU * p.freq / 2, ph = TAU * t * p.speed;
      const n = Math.sin(x + ph) + Math.sin(y * 1.7 - ph) + Math.sin(x * 0.7 + y + ph) + Math.sin(Math.hypot(x - 3 + Math.sin(ph), y - 1.5 + Math.cos(ph)) * 1.3);
      return lutAt(p._stops, fract((n / 8 + 0.5) * p.cycles - t * p.shift), o);
    },
  });
  layer('marble', {
    name: 'Marble', cat: 'Noise and fluids', desc: 'Veined stone, gently flowing.',
    params: [P.pal([{ at: 0, color: '#f4f1ea' }, { at: 0.6, color: '#d8d2c6' }, { at: 0.85, color: '#6b6355' }, { at: 1, color: '#1f1b16' }]), P.range('veins', 'Veins', 0.5, 12, 0.5, 3), P.range('turb', 'Turbulence', 0, 6, 0.1, 2.5), P.int('sx', 'Scale x', 1, 16, 4), P.int('sy', 'Scale y', 1, 8, 2), P.range('flow', 'Flow', 0, 2, 0.05, 0.5), P.speed(1), P.seed(3)],
    sample(u, v, t, p, o, c) {
      flowXY(t, p); const ox = p._fx, oy = p._fy;
      const n = fbm(u * p.sx + ox, v * p.sy + oy, p.sx, p.sy, 5, p.seed);
      return lutAt(p._stops, 0.5 + 0.5 * Math.sin(TAU * (u * p.veins + v * 0.5 + p.turb * n)), o);
    },
  });
  layer('aurora', {
    name: 'Aurora', cat: 'Noise and fluids', desc: 'Northern-light curtains with vertical rays.',
    params: [P.pal('aurora'), P.range('height', 'Height', 0, 1, 0.01, 0.55), P.range('wave', 'Waviness', 0, 1, 0.01, 0.45), P.range('spread', 'Spread', 0.05, 1, 0.01, 0.5), P.range('rays', 'Rays', 0, 1, 0.01, 0.6), P.int('sx', 'Scale', 1, 16, 4), P.range('intensity', 'Intensity', 0.2, 2, 0.05, 1.1), P.speed(1), P.seed(5)],
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed;
      const w = fbm(u * p.sx + Math.cos(a) * 0.7, Math.sin(a) * 0.7, p.sx, 4, 3, p.seed);
      const line = p.height + (w - 0.5) * p.wave;
      const d = v - line;
      const prof = d > 0 ? Math.exp(-d * d * 220) : Math.exp(-(d * d) / (p.spread * p.spread * 0.18));
      const r = 1 - p.rays + p.rays * vnoise(u * p.sx * 6 - Math.cos(a) * 2, Math.sin(a) * 2, p.sx * 6, 4, p.seed + 9);
      lutAt(p._stops, clamp(w * 0.9 + (line - v) * 0.6), o);
      o[3] *= clamp(prof * r * p.intensity);
      return o;
    },
  });
  layer('fire', {
    name: 'Fire', cat: 'Noise and fluids', desc: 'Rising flames; transparent above the fire line.',
    params: [P.pal('fire'), P.range('height', 'Flame height', 0.1, 1.5, 0.01, 0.8), P.range('turb', 'Turbulence', 0, 2, 0.05, 0.9), P.int('sx', 'Scale x', 1, 32, 10), P.int('sy', 'Scale y', 1, 8, 2), P.speed(2, 'speed', 'Rise speed', 1, 8), P.seed(11)],
    sample(u, v, t, p, o, c) {
      const n = fbm(u * p.sx + Math.sin(TAU * t) * 0.3, (v + t * p.speed) * p.sy, p.sx, p.sy, 5, p.seed);
      const f = clamp((v - (1 - p.height)) / p.height + (n - 0.5) * p.turb * 1.6);
      return lutAt(p._stops, f, o);
    },
  });
  layer('lava', {
    name: 'Lava lamp', cat: 'Noise and fluids', desc: 'Metaballs that drift and merge.',
    params: [P.pal('lava'), P.color('bg', 'Background', '#1a0000'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.int('count', 'Blobs', 2, 16, 7), P.range('size', 'Size', 0.05, 0.6, 0.01, 0.22), P.range('threshold', 'Threshold', 0.3, 3, 0.05, 1), P.range('soft', 'Softness', 0.01, 1, 0.01, 0.15), P.speed(1), P.seed(2)],
    prepare(p) {
      p._bg = rgb01(p.bg);
      const r = PL.rng(p.seed * 7919);
      p._b = Array.from({ length: p.count }, () => ({ x: r(), y: r(), ax: 0.1 + r() * 0.35, ay: 0.15 + r() * 0.35, kx: 1 + ((r() * 2) | 0), ky: 1 + ((r() * 2) | 0), px: r(), py: r(), s: 0.6 + r() * 0.8 }));
    },
    sample(u, v, t, p, o, c) {
      let f = 0;
      for (const b of p._b) {
        const bx = b.x + b.ax * Math.sin(TAU * (t * p.speed * b.kx + b.px));
        const by = b.y + b.ay * Math.sin(TAU * (t * p.speed * b.ky + b.py));
        let dx = Math.abs(u - fract(bx)); dx = Math.min(dx, 1 - dx);
        const dy = (v - by), r = p.size * b.s;
        f += (r * r) / ((dx * c.A) * (dx * c.A) * 0.15 + dy * dy + 1e-4) * 0.25;
      }
      const m = smoothstep(p.threshold - p.soft, p.threshold + p.soft, f);
      lutAt(p._stops, clamp(f / (p.threshold * 3)), o);
      o[0] = mix(p._bg[0], o[0], m); o[1] = mix(p._bg[1], o[1], m); o[2] = mix(p._bg[2], o[2], m);
      o[3] = mix(p.bgAlpha, 1, m);
      return o;
    },
  });
  layer('voronoi', {
    name: 'Cells', cat: 'Noise and fluids', desc: 'Voronoi cells: crystals, ice, scales, stained glass.',
    params: [P.pal('icyLake'), P.select('mode', 'Style', [['crystal', 'Crystal facets'], ['cells', 'Flat cells'], ['distance', 'Distance glow'], ['edges', 'Edges only']], 'crystal'), P.int('cx', 'Columns', 1, 48, 14), P.int('cy', 'Rows', 1, 16, 3), P.range('jitter', 'Jitter', 0, 1, 0.01, 0.9), P.range('edge', 'Edge width', 0, 0.3, 0.005, 0.04), P.color('edgeColor', 'Edge colour', '#ffffff'), P.speed(1), P.speed(0, 'shift', 'Colour shift'), P.seed(4)],
    prepare: colorPrep(['edgeColor']),
    sample(u, v, t, p, o, c) {
      const gx = u * p.cx, gy = v * p.cy, ix = Math.floor(gx), iy = Math.floor(gy);
      const kx = c.W / p.cx, ky = c.H / p.cy, k = Math.min(kx, ky);
      let f1 = 1e9, f2 = 1e9, h1 = 0;
      for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
        const cx = ix + i, cy = iy + j, wx = wrap(cx, p.cx), wy = wrap(cy, p.cy);
        const h = hash2(wx, wy, p.seed), h2 = hash2(wx, wy, p.seed + 1), h3 = hash2(wx, wy, p.seed + 2);
        const px = cx + 0.5 + p.jitter * 0.5 * Math.sin(TAU * (t * p.speed + h)) * (h2 > 0.5 ? 1 : -1) * 0.9 + p.jitter * (h2 - 0.5) * 0.5;
        const py = cy + 0.5 + p.jitter * 0.5 * Math.cos(TAU * (t * p.speed + h3)) * 0.9;
        const dx = (gx - px) * kx / k, dy = (gy - py) * ky / k, d = Math.hypot(dx, dy);
        if (d < f1) { f2 = f1; f1 = d; h1 = h; } else if (d < f2) f2 = d;
      }
      const e = 1 - smoothstep(p.edge, p.edge + 0.03, f2 - f1);
      let f;
      if (p.mode === 'cells') f = h1;
      else if (p.mode === 'distance') f = clamp(f1);
      else if (p.mode === 'edges') f = clamp(1 - (f2 - f1) * 2);
      else f = clamp(h1 * 0.55 + (f2 - f1) * 0.6 + (1 - f1) * 0.15);
      lutAt(p._stops, fract(f - t * p.shift + (p.shift ? 1 : 0)), o);
      if (p.edge > 0 && p.mode !== 'edges') { o[0] = mix(o[0], p._edgeColor[0], e); o[1] = mix(o[1], p._edgeColor[1], e); o[2] = mix(o[2], p._edgeColor[2], e); o[3] = mix(o[3], 1, e); }
      return o;
    },
  });
  layer('caustics', {
    name: 'Caustics', cat: 'Noise and fluids', desc: 'Light rippling on the bottom of a pool.',
    params: [P.color('bg', 'Water', '#04536b'), P.color('light', 'Light', '#bff8ff'), P.range('scale', 'Scale', 0.5, 6, 0.1, 2), P.range('sharp', 'Sharpness', 1, 12, 0.5, 5), P.range('intensity', 'Intensity', 0, 2, 0.05, 1), P.speed(1)],
    prepare: colorPrep(['bg', 'light']),
    sample(u, v, t, p, o, c) {
      const x = u * TAU * p.scale * Math.min(c.A, 6) / 2, y = v * TAU * p.scale, ph = TAU * t * p.speed;
      const a = Math.sin(x + 1.6 * Math.sin(y * 0.9 + ph)), b = Math.sin(y * 1.3 + 1.6 * Math.sin(x * 0.8 - ph)), d = Math.sin((x + y) * 0.6 + Math.sin(x * 0.5 + ph));
      const l = Math.pow(clamp(1 - Math.abs(a + b + d) / 3), p.sharp) * p.intensity;
      return mixC(o, p._bg, p._light, clamp(l));
    },
  });

  // =====================================================================================
  // Particles and light
  // =====================================================================================
  // Cell-based particles: one candidate point per grid cell, neighbours checked so points can cross edges.
  function particles(u, v, p, c, cell, fn) {
    const nx = Math.max(1, Math.round(c.W / cell)), ny = Math.max(1, Math.round(c.H / cell));
    const gx = u * nx, gy = v * ny, ix = Math.floor(gx), iy = Math.floor(gy);
    const cw = c.W / nx, ch = c.H / ny;
    let acc = 0;
    for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
      const cx = ix + i, cy = iy + j, wx = wrap(cx, nx), wy = wrap(cy, ny);
      const h = hash2(wx, wy, p.seed);
      if (h > p.density) continue;
      const px = cx + hash2(wx, wy, p.seed + 1), py = cy + hash2(wx, wy, p.seed + 2);
      acc = Math.max(acc, fn((gx - px) * cw, (gy - py) * ch, h / p.density, hash2(wx, wy, p.seed + 3)));
    }
    return acc;
  }
  const starShape = (dx, dy, s, rays) => {
    const d = Math.hypot(dx, dy);
    let m = Math.max(0, 1 - d / (s + 1e-3)); m *= m;
    if (rays) {
      const ax = Math.abs(dx), ay = Math.abs(dy), L = s * 3.2, W = Math.max(0.35, s * 0.18);
      m = Math.max(m, Math.max(0, 1 - ax / L) * Math.max(0, 1 - ay / W), Math.max(0, 1 - ay / L) * Math.max(0, 1 - ax / W));
    }
    return m;
  };
  layer('sparkles', {
    name: 'Sparkles', cat: 'Particles and light', desc: 'Twinkling four-point glints. Layer over anything.',
    params: [P.color('color', 'Colour', '#ffffff'), P.range('cell', 'Spacing (px)', 4, 60, 1, 14), P.range('density', 'Density', 0.02, 1, 0.01, 0.35), P.range('size', 'Size (px)', 0.5, 8, 0.1, 2.2), P.bool('rays', 'Star rays', true), P.int('twinkle', 'Twinkles per loop', 1, 8, 2), P.range('variety', 'Size variety', 0, 1, 0.01, 0.6), P.seed(12)],
    prepare(p) {
      colorPrep(['color'])(p);
      p._fn = (dx, dy, h, h2) => { // reads the frame time from p._t (set by sample, constant per frame)
        const t = p._t, tw = Math.pow(Math.max(0, Math.sin(TAU * (t * p.twinkle + h2))), 3);
        return starShape(dx, dy, p.size * (1 - p.variety * h) * tw, p.rays) * tw;
      };
    },
    sample(u, v, t, p, o, c) {
      p._t = t;
      const a = particles(u, v, p, c, p.cell, p._fn);
      return setC(o, p._color, clamp(a));
    },
  });
  layer('stars', {
    name: 'Starfield', cat: 'Particles and light', desc: 'Drifting stars at several depths.',
    params: [P.color('color', 'Colour', '#e8f0ff'), P.range('cell', 'Spacing (px)', 3, 40, 1, 9), P.range('density', 'Density', 0.02, 1, 0.01, 0.4), P.range('size', 'Size (px)', 0.3, 4, 0.1, 1.1), P.int('depth', 'Depth layers', 1, 3, 2), P.speed(1, 'speed', 'Drift'), P.int('twinkle', 'Twinkles per loop', 0, 8, 1), P.seed(21)],
    prepare(p) {
      colorPrep(['color'])(p);
      // per-depth seed/density holders and shape callbacks, built once (particles() reads only seed and density)
      p._q = []; p._fns = [];
      for (let l = 0; l < p.depth; l++) {
        p._q.push({ seed: p.seed + l * 101, density: p.density });
        p._fns.push((dx, dy, h, h2) => {
          const t = p._t, tw = p.twinkle ? 0.55 + 0.45 * Math.sin(TAU * (t * p.twinkle + h2)) : 1;
          return starShape(dx, dy, p.size * (0.6 + l * 0.35) * (0.5 + h), false) * tw * (0.5 + 0.5 * (l + 1) / p.depth);
        });
      }
    },
    sample(u, v, t, p, o, c) {
      let a = 0;
      p._t = t;
      for (let l = 0; l < p.depth; l++) {
        const sp = p.speed * (l + 1);
        a = Math.max(a, particles(fract(u - t * sp), v, p._q[l], c, p.cell * (1 + l * 0.6), p._fns[l]));
      }
      return setC(o, p._color, clamp(a));
    },
  });
  layer('particles', {
    name: 'Snow, rain, bubbles', cat: 'Particles and light', desc: 'Falling or rising particles.',
    params: [P.select('mode', 'Type', [['snow', 'Snow'], ['rain', 'Rain'], ['bubbles', 'Bubbles'], ['embers', 'Embers']], 'snow'), P.color('color', 'Colour', '#ffffff'), P.range('cell', 'Spacing (px)', 4, 60, 1, 12), P.range('density', 'Density', 0.02, 1, 0.01, 0.5), P.range('size', 'Size (px)', 0.5, 8, 0.1, 1.6), P.speed(1, 'speed', 'Fall speed', 1, 8), P.speed(0, 'wind', 'Wind'), P.seed(31)],
    prepare(p) {
      colorPrep(['color'])(p);
      p._fn = (dx, dy, h, h2) => {
        const s = p.size * (0.6 + 0.8 * h2);
        if (p.mode === 'rain') return Math.max(0, 1 - Math.abs(dx) / (s * 0.45)) * Math.max(0, 1 - Math.abs(dy) / (s * 4));
        if (p.mode === 'bubbles') { const d = Math.hypot(dx, dy); return Math.max(0, 1 - Math.abs(d - s * 1.6) / 0.9) * 0.9 + (d < s * 1.6 ? 0.12 : 0); }
        const m = Math.max(0, 1 - Math.hypot(dx, dy) / s);
        return p.mode === 'embers' ? m * (0.5 + 0.5 * Math.sin(TAU * (p._t * 3 + h2))) : m;
      };
    },
    sample(u, v, t, p, o, c) {
      p._t = t;
      const up = p.mode === 'bubbles' || p.mode === 'embers';
      const vv = fract(v + (up ? 1 : -1) * t * p.speed), uu = fract(u - t * p.wind);
      const a = particles(uu, vv, p, c, p.cell, p._fn);
      return setC(o, p._color, clamp(a));
    },
  });
  const shineBand = (f, c0, w, soft) => { const d = Math.abs(f - c0); return 1 - smoothstep(w * (1 - soft), w + 1e-4, d); };
  layer('shine', {
    name: 'Shine sweep', cat: 'Particles and light', desc: 'A highlight band that sweeps across, then rests.',
    params: [P.color('color', 'Colour', '#ffffff'), P.range('angle', 'Angle', 0, 360, 1, 70), P.range('width', 'Width', 0.01, 0.6, 0.01, 0.12), P.range('soft', 'Softness', 0, 1, 0.01, 0.7), P.range('intensity', 'Intensity', 0, 1, 0.01, 0.85), P.int('sweeps', 'Sweeps per loop', 1, 6, 1), P.range('hold', 'Rest time', 0, 0.9, 0.01, 0.45), P.bool('double', 'Double band', false)],
    prepare: colorPrep(['color']),
    sample(u, v, t, p, o, c) {
      const f = proj(u, v, p.angle, c.A), local = fract(t * p.sweeps), act = 1 - p.hold;
      if (local >= act) return setC(o, p._color, 0);
      const cpos = mix(-p.width * 1.5, 1 + p.width * 1.5, local / act);
      let a = shineBand(f, cpos, p.width, p.soft);
      if (p.double) a = Math.max(a, shineBand(f, cpos - p.width * 2.2, p.width * 0.45, p.soft) * 0.8);
      return setC(o, p._color, a * p.intensity);
    },
  });
  layer('rays', {
    name: 'Light rays', cat: 'Particles and light', desc: 'Rotating sunburst rays from a point.',
    params: [P.color('color', 'Colour', '#fff3c4'), P.range('cx', 'Centre x', -0.5, 1.5, 0.01, 0.5), P.range('cy', 'Centre y', -1, 2, 0.01, 1.2), P.int('count', 'Rays', 2, 48, 12), P.range('duty', 'Ray width', 0.05, 0.95, 0.01, 0.4), P.range('soft', 'Softness', 0, 0.5, 0.01, 0.25), P.range('falloff', 'Falloff', 0, 1, 0.01, 0.4), P.range('intensity', 'Intensity', 0, 1, 0.01, 0.6), P.speed(1, 'speed', 'Spin')],
    prepare: colorPrep(['color']),
    sample(u, v, t, p, o, c) {
      const dx = (u - p.cx) * c.A, dy = v - p.cy;
      const g = fract((Math.atan2(dy, dx) / TAU + 0.5) * p.count - t * p.speed);
      const m = smoothstep(0, p.soft + 1e-3, g) * (1 - smoothstep(p.duty, p.duty + p.soft + 1e-3, g));
      const fall = mix(1, clamp(1 - Math.hypot(dx / c.A, dy) * 0.8), p.falloff);
      return setC(o, p._color, m * fall * p.intensity);
    },
  });
  layer('glints', {
    name: 'Glitter', cat: 'Particles and light', desc: 'Dense fine glitter that flickers like foil.',
    params: [P.color('color', 'Colour', '#fff6d6'), P.range('density', 'Density', 0.05, 1, 0.01, 0.6), P.range('cell', 'Grain (px)', 1, 8, 1, 2), P.int('rate', 'Flickers per loop', 1, 12, 4), P.range('sharp', 'Sharpness', 1, 16, 0.5, 6), P.seed(41)],
    prepare: colorPrep(['color']),
    sample(u, v, t, p, o, c) {
      const ix = Math.floor(u * c.W / p.cell), iy = Math.floor(v * c.H / p.cell);
      const h = hash2(ix, iy, p.seed);
      if (h > p.density) return setC(o, p._color, 0);
      const s = Math.pow(Math.max(0, Math.sin(TAU * (t * p.rate + hash2(ix, iy, p.seed + 1)))), p.sharp);
      return setC(o, p._color, s);
    },
  });
  layer('matrix', {
    name: 'Digital rain', cat: 'Particles and light', desc: 'Falling code columns.',
    params: [P.color('head', 'Head colour', '#e6ffe6'), P.color('trail', 'Trail colour', '#00ff55'), P.color('bg', 'Background', '#001a08'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.int('cols', 'Columns', 4, 120, 40), P.int('rows', 'Glyph rows', 2, 24, 8), P.range('trailLen', 'Trail length', 0.1, 1, 0.01, 0.6), P.speed(1, 'speed', 'Fall speed', 1, 6), P.seed(51)],
    prepare: colorPrep(['head', 'trail', 'bg']),
    sample(u, v, t, p, o, c) {
      const col = Math.floor(u * p.cols), lx = fract(u * p.cols);
      const h = hash2(col, 0, p.seed), k = 1 + ((hash2(col, 1, p.seed) * 2) | 0);
      const row = Math.floor(v * p.rows), qv = (row + 0.5) / p.rows;
      const head = fract(t * p.speed * k + h);
      const d = fract(head - qv);
      let a = d < p.trailLen ? 1 - d / p.trailLen : 0;
      const glyph = hash2(col, row, p.seed + Math.floor(t * 12)) > 0.25 ? 1 : 0.35;
      const inCell = lx > 0.15 && lx < 0.85 && fract(v * p.rows) > 0.1 && fract(v * p.rows) < 0.9 ? 1 : 0;
      a *= glyph * inCell;
      const isHead = d < 1 / p.rows ? 1 : 0;
      const cc = isHead ? p._head : p._trail;
      return mixC(o, p._bg, cc, a, mix(p.bgAlpha, 1, a));
    },
  });

  // =====================================================================================
  // Materials
  // =====================================================================================
  layer('chrome', {
    name: 'Chrome', cat: 'Materials', desc: 'Mirror metal: sky above, dark horizon, ground below.',
    params: [P.stops('chrome'), P.range('horizon', 'Horizon', 0.1, 0.9, 0.01, 0.5), P.range('warp', 'Warp', 0, 0.4, 0.005, 0.08), P.int('freq', 'Ripples', 1, 12, 3), P.speed(1), P.range('tilt', 'Tilt', -0.5, 0.5, 0.01, 0.1)],
    sample(u, v, t, p, o, c) {
      const f = v + (0.5 - p.horizon) + p.warp * Math.sin(TAU * (u * p.freq - t * p.speed)) + (u - 0.5) * p.tilt;
      return lutAt(p._stops, clamp(f), o);
    },
  });
  layer('brushed', {
    name: 'Brushed metal', cat: 'Materials', desc: 'Fine horizontal grain with a sliding highlight.',
    params: [P.color('color', 'Metal colour', '#a9b1bb'), P.range('grain', 'Grain', 0, 1, 0.01, 0.45), P.int('lines', 'Grain lines', 4, 120, 50), P.range('hw', 'Highlight width', 0.05, 1, 0.01, 0.25), P.range('hi', 'Highlight', 0, 1, 0.01, 0.6), P.speed(1), P.seed(61)],
    prepare: colorPrep(['color']),
    sample(u, v, t, p, o, c) {
      const n = fbm(u * 2, v * p.lines, 2, p.lines, 3, p.seed);
      let hp = fract(u - t * p.speed) - 0.5;
      const h = Math.exp(-(hp * hp) / (p.hw * p.hw * 0.25)) * p.hi;
      const k = 1 - p.grain * 0.5 + p.grain * n * 0.8;
      o[0] = clamp(p._color[0] * k + h); o[1] = clamp(p._color[1] * k + h); o[2] = clamp(p._color[2] * k + h); o[3] = 1;
      return o;
    },
  });
  layer('holo', {
    name: 'Holographic', cat: 'Materials', desc: 'Iridescent foil that shifts hue as it moves.',
    params: [P.pal('holo', true), P.range('angle', 'Angle', 0, 360, 1, 120), P.range('scale', 'Colour repeats', 0.25, 6, 0.25, 1.5), P.range('noise', 'Noise warp', 0, 2, 0.05, 0.6), P.speed(1), P.seed(71)],
    sample(u, v, t, p, o, c) {
      const a = TAU * t;
      const n = fbm(u * 6 + Math.cos(a) * 0.5, v * 2 + Math.sin(a) * 0.5, 6, 2, 3, p.seed);
      return lutAt(p._stops, fract(proj(u, v, p.angle, c.A) * p.scale + n * p.noise - t * p.speed), o);
    },
  });
  layer('opal', {
    name: 'Opal', cat: 'Materials', desc: 'Milky stone with flashes of pastel colour.',
    params: [P.pal('opal', true), P.color('milk', 'Milk colour', '#f4f1f8'), P.range('milkAmt', 'Milkiness', 0, 1, 0.01, 0.35), P.int('sx', 'Scale x', 1, 24, 8), P.int('sy', 'Scale y', 1, 8, 2), P.range('spread', 'Colour spread', 0.2, 5, 0.05, 2.2), P.range('flash', 'Flash', 0, 1, 0.01, 0.5), P.speed(1), P.seed(81)],
    prepare: colorPrep(['milk']),
    sample(u, v, t, p, o) {
      const a = TAU * t * p.speed;
      const n = fbm(u * p.sx + Math.cos(a) * 0.6, v * p.sy + Math.sin(a) * 0.6, p.sx, p.sy, 4, p.seed);
      const n2 = fbm(u * p.sx * 2 - Math.cos(a) * 0.6, v * p.sy * 2, p.sx * 2, p.sy * 2, 3, p.seed + 3);
      lutAt(p._stops, fract(n * p.spread + t * p.speed), o);
      const m = p.milkAmt * (1 - n2), f = Math.pow(n2, 3) * p.flash * 2;
      for (let k = 0; k < 3; k++) o[k] = clamp(mix(o[k], p._milk[k], m) + f);
      return o;
    },
  });
  layer('wood', {
    name: 'Wood grain', cat: 'Materials', desc: 'Growth rings with natural wobble.',
    params: [P.color('dark', 'Dark', '#5a3416'), P.color('light', 'Light', '#c98b4f'), P.range('rings', 'Rings', 1, 40, 0.5, 12), P.range('turb', 'Turbulence', 0, 2, 0.05, 0.6), P.speed(0)],
    prepare: colorPrep(['dark', 'light']),
    sample(u, v, t, p, o, c) {
      const n = fbm(u * 4, v * 2, 4, 2, 4, 5);
      const d = Math.hypot((u - 0.5) * c.A * 0.15, (v + 1.5) * 1);
      const f = Math.pow(fract(d * p.rings + n * p.turb - t * p.speed), 0.6);
      return mixC(o, p._light, p._dark, f);
    },
  });
  layer('image', {
    name: 'Image', cat: 'Materials', desc: 'Your own picture (PNG, JPG), stretched or tiled.',
    params: [P.image(), P.int('tx', 'Tiles x', 1, 16, 1), P.int('ty', 'Tiles y', 1, 8, 1), P.speed(0, 'sx', 'Scroll x'), P.speed(0, 'sy', 'Scroll y')],
    prepare(p) { p._img = PL.imageCache && PL.imageCache[p.src]; },
    sample(u, v, t, p, o, c) {
      const im = p._img;
      if (!im) { o[0] = 0; o[1] = 0; o[2] = 0; o[3] = 0; return o; }
      const x = fract(u * p.tx - t * p.sx) * im.w, y = fract(v * p.ty - t * p.sy) * im.h;
      const i = ((Math.min(im.h - 1, y | 0) * im.w) + Math.min(im.w - 1, x | 0)) * 4, d = im.data;
      o[0] = d[i] / 255; o[1] = d[i + 1] / 255; o[2] = d[i + 2] / 255; o[3] = d[i + 3] / 255;
      return o;
    },
  });

  // =====================================================================================
  // Effects (post-processing on the composited image)
  // =====================================================================================
  effect('hue', {
    name: 'Hue shift', cat: 'Colour', desc: 'Rotates every colour round the wheel; cycles make it loop.',
    params: [P.range('deg', 'Shift (degrees)', -180, 180, 1, 0), P.speed(1, 'cycles', 'Full turns per loop')],
    run(buf, W, H, t, p) {
      const m = hueMatrix(p.deg + 360 * p.cycles * t);
      return eachPx(buf, W, H, (i) => { const r = buf[i], g = buf[i + 1], b = buf[i + 2]; buf[i] = clamp(m[0] * r + m[1] * g + m[2] * b); buf[i + 1] = clamp(m[3] * r + m[4] * g + m[5] * b); buf[i + 2] = clamp(m[6] * r + m[7] * g + m[8] * b); });
    },
  });
  effect('adjust', {
    name: 'Adjust', cat: 'Colour', desc: 'Brightness, contrast, saturation and gamma.',
    params: [P.range('bright', 'Brightness', -1, 1, 0.01, 0), P.range('contrast', 'Contrast', 0, 3, 0.01, 1), P.range('sat', 'Saturation', 0, 3, 0.01, 1), P.range('gamma', 'Gamma', 0.2, 3, 0.01, 1)],
    run(buf, W, H, t, p) {
      const ig = 1 / p.gamma;
      return eachPx(buf, W, H, (i) => {
        let r = buf[i], g = buf[i + 1], b = buf[i + 2];
        const l = luma(r, g, b);
        r = l + (r - l) * p.sat; g = l + (g - l) * p.sat; b = l + (b - l) * p.sat;
        r = (r - 0.5) * p.contrast + 0.5 + p.bright; g = (g - 0.5) * p.contrast + 0.5 + p.bright; b = (b - 0.5) * p.contrast + 0.5 + p.bright;
        buf[i] = Math.pow(clamp(r), ig); buf[i + 1] = Math.pow(clamp(g), ig); buf[i + 2] = Math.pow(clamp(b), ig);
      });
    },
  });
  effect('gradientMap', {
    name: 'Gradient map', cat: 'Colour', desc: 'Recolours by brightness using a palette.',
    params: [P.pal('gold'), P.range('amount', 'Amount', 0, 1, 0.01, 1), P.speed(0, 'shift', 'Palette shift')],
    run(buf, W, H, t, p) {
      const o = [0, 0, 0, 0];
      return eachPx(buf, W, H, (i) => {
        const l = luma(buf[i], buf[i + 1], buf[i + 2]);
        lutAt(p._stops, p.shift ? fract(l - t * p.shift) : l, o);
        buf[i] = mix(buf[i], o[0], p.amount); buf[i + 1] = mix(buf[i + 1], o[1], p.amount); buf[i + 2] = mix(buf[i + 2], o[2], p.amount);
      });
    },
  });
  effect('duotone', {
    name: 'Duotone', cat: 'Colour', desc: 'Two-colour print look.',
    params: [P.color('dark', 'Shadows', '#1b0036'), P.color('light', 'Highlights', '#00f0ff'), P.range('amount', 'Amount', 0, 1, 0.01, 1)],
    prepare: colorPrep(['dark', 'light']),
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i) => { const l = luma(buf[i], buf[i + 1], buf[i + 2]); for (let k = 0; k < 3; k++) buf[i + k] = mix(buf[i + k], mix(p._dark[k], p._light[k], l), p.amount); });
    },
  });
  effect('posterize', {
    name: 'Posterize', cat: 'Colour', desc: 'Limits each channel to a few levels: flat, cel-shaded colour.',
    params: [P.int('levels', 'Levels', 2, 16, 5)],
    run(buf, W, H, t, p) { const n = p.levels - 1; return eachPx(buf, W, H, (i) => { for (let k = 0; k < 3; k++) buf[i + k] = Math.round(buf[i + k] * n) / n; }); },
  });
  effect('invert', {
    name: 'Invert', cat: 'Colour', desc: 'Negative image; can flash on and off.',
    params: [P.range('amount', 'Amount', 0, 1, 0.01, 1), P.int('flash', 'Flashes per loop', 0, 8, 0)],
    run(buf, W, H, t, p) {
      const a = p.flash ? (fract(t * p.flash) < 0.5 ? p.amount : 0) : p.amount;
      return eachPx(buf, W, H, (i) => { for (let k = 0; k < 3; k++) buf[i + k] = mix(buf[i + k], 1 - buf[i + k], a); });
    },
  });
  effect('pulse', {
    name: 'Pulse', cat: 'Colour', desc: 'Breathing brightness, saturation or opacity.',
    params: [P.select('what', 'Pulse', [['bright', 'Brightness'], ['sat', 'Saturation'], ['alpha', 'Opacity']], 'bright'), P.range('amount', 'Amount', 0, 1, 0.01, 0.35), P.speed(2, 'speed', 'Pulses per loop', 1, 8)],
    run(buf, W, H, t, p) {
      const s = 0.5 + 0.5 * Math.sin(TAU * t * p.speed);
      return eachPx(buf, W, H, (i) => {
        if (p.what === 'alpha') buf[i + 3] *= 1 - p.amount * s;
        else if (p.what === 'sat') { const l = luma(buf[i], buf[i + 1], buf[i + 2]), k = 1 - p.amount * s; for (let c = 0; c < 3; c++) buf[i + c] = l + (buf[i + c] - l) * k; }
        else { const k = 1 + p.amount * (s * 2 - 1) * 0.6; for (let c = 0; c < 3; c++) buf[i + c] = clamp(buf[i + c] * k); }
      });
    },
  });

  // ---- shape and lighting ("modelling") ----
  effect('cylinder', {
    name: '3D tube', cat: 'Lighting and shape', desc: 'Shades the paint like a rounded tube with a specular line.',
    params: [P.select('axis', 'Tube runs', [['h', 'Horizontally'], ['v', 'Vertically']], 'h'), P.range('strength', 'Shading', 0, 1, 0.01, 0.6), P.range('ambient', 'Ambient', 0, 1, 0.01, 0.35), P.range('pos', 'Highlight position', 0, 1, 0.01, 0.3), P.range('width', 'Highlight width', 0.01, 0.5, 0.01, 0.08), P.range('hi', 'Highlight', 0, 1, 0.01, 0.65), P.color('hiColor', 'Highlight colour', '#ffffff')],
    prepare: colorPrep(['hiColor']),
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i, u, v) => {
        const q = p.axis === 'h' ? v : u;
        const sh = mix(1, p.ambient + (1 - p.ambient) * Math.sin(Math.PI * q), p.strength);
        const s = Math.exp(-Math.pow((q - p.pos) / p.width, 2)) * p.hi;
        for (let k = 0; k < 3; k++) buf[i + k] = clamp(mix(buf[i + k] * sh, p._hiColor[k], s));
      });
    },
  });
  effect('bevel', {
    name: 'Bevel and emboss', cat: 'Lighting and shape', desc: 'Treats brightness as height and lights it; the light can orbit.',
    params: [P.range('strength', 'Strength', 0, 2, 0.01, 0.7), P.range('depth', 'Depth', 0.5, 12, 0.1, 4), P.range('angle', 'Light angle', 0, 360, 1, 135), P.speed(0, 'spin', 'Light orbits per loop'), P.int('soft', 'Smoothing (px)', 0, 6, 1), P.bool('invert', 'Invert height', false)],
    run(buf, W, H, t, p) {
      const hgt = new Float32Array(W * H);
      eachPx(buf, W, H, (i, u, v, x, y) => { hgt[y * W + x] = luma(buf[i], buf[i + 1], buf[i + 2]) * (p.invert ? -1 : 1); });
      let hb = hgt;
      if (p.soft > 0) {
        const tmp = new Float32Array(W * H * 4); for (let k = 0; k < W * H; k++) { tmp[k * 4] = hgt[k]; tmp[k * 4 + 3] = 1; }
        const bl = boxBlur(tmp, W, H, p.soft); hb = new Float32Array(W * H); for (let k = 0; k < W * H; k++) hb[k] = bl[k * 4];
      }
      const a = ((p.angle + 360 * p.spin * t) * Math.PI) / 180, lx = Math.cos(a), ly = -Math.sin(a), lz = 0.6;
      const ln = Math.hypot(lx, ly, lz);
      const h = (xx, yy) => hb[wrap(yy, H) * W + wrap(xx, W)];
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const nx = (h(x - 1, y) - h(x + 1, y)) * p.depth, ny = (h(x, y - 1) - h(x, y + 1)) * p.depth;
        const nl = Math.hypot(nx, ny, 1);
        const d = (nx * lx + ny * -ly + lz) / (nl * ln) - lz / ln;
        for (let k = 0; k < 3; k++) buf[i + k] = clamp(buf[i + k] + d * p.strength);
      });
    },
  });
  effect('gloss', {
    name: 'Gloss', cat: 'Lighting and shape', desc: 'Glassy highlight over the top half, like a jelly button.',
    params: [P.range('pos', 'Edge', 0.1, 0.9, 0.01, 0.5), P.range('strength', 'Strength', 0, 1, 0.01, 0.35), P.range('curve', 'Curve', -0.5, 0.5, 0.01, 0.15), P.range('soft', 'Softness', 0, 0.3, 0.01, 0.04)],
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i, u, v) => {
        const edge = p.pos + p.curve * (Math.pow((u - 0.5) * 2, 2) - 0.5);
        const m = (1 - smoothstep(edge - p.soft, edge + p.soft + 1e-3, v)) * p.strength * (1 - v * 0.5);
        for (let k = 0; k < 3; k++) buf[i + k] = clamp(mix(buf[i + k], 1, m));
      });
    },
  });
  effect('vignette', {
    name: 'Vignette', cat: 'Lighting and shape', desc: 'Darkens or fades the edges.',
    params: [P.range('amount', 'Amount', 0, 1, 0.01, 0.4), P.range('size', 'Size', 0.1, 1.5, 0.01, 0.75), P.bool('toAlpha', 'Fade to transparent', false)],
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i, u, v) => {
        const d = Math.hypot((u - 0.5) * 2, (v - 0.5) * 2) / Math.SQRT2;
        const m = smoothstep(p.size * 0.5, p.size, d) * p.amount;
        if (p.toAlpha) buf[i + 3] *= 1 - m; else for (let k = 0; k < 3; k++) buf[i + k] *= 1 - m;
      });
    },
  });
  effect('edgeFade', {
    name: 'Edge fade', cat: 'Lighting and shape', desc: 'Makes the left or right end transparent so the name colour shows.',
    params: [P.range('left', 'Left', 0, 0.9, 0.01, 0), P.range('right', 'Right', 0, 0.9, 0.01, 0.3), P.range('top', 'Top', 0, 0.9, 0.01, 0), P.range('bottom', 'Bottom', 0, 0.9, 0.01, 0)],
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i, u, v) => {
        let a = 1;
        if (p.left) a *= smoothstep(0, p.left, u);
        if (p.right) a *= smoothstep(0, p.right, 1 - u);
        if (p.top) a *= smoothstep(0, p.top, v);
        if (p.bottom) a *= smoothstep(0, p.bottom, 1 - v);
        buf[i + 3] *= a;
      });
    },
  });
  effect('glow', {
    name: 'Bloom', cat: 'Lighting and shape', desc: 'Bright areas bleed light into their surroundings.',
    params: [P.range('threshold', 'Threshold', 0, 1, 0.01, 0.6), P.int('radius', 'Radius (px)', 1, 12, 3), P.range('strength', 'Strength', 0, 3, 0.05, 1)],
    run(buf, W, H, t, p) {
      const br = new Float32Array(buf.length);
      eachPx(buf, W, H, (i) => { const l = luma(buf[i], buf[i + 1], buf[i + 2]), k = smoothstep(p.threshold, 1, l); br[i] = buf[i]; br[i + 1] = buf[i + 1]; br[i + 2] = buf[i + 2]; br[i + 3] = k * buf[i + 3]; });
      const bl = boxBlur(br, W, H, p.radius);
      return eachPx(buf, W, H, (i) => { const a = bl[i + 3] * p.strength; for (let k = 0; k < 3; k++) buf[i + k] = clamp(buf[i + k] + bl[i + k] * a); });
    },
  });

  // ---- distortion ----
  effect('wave', {
    name: 'Wave distort', cat: 'Distortion', desc: 'Ripples the image sideways or up and down.',
    params: [P.select('axis', 'Shift', [['x', 'Sideways'], ['y', 'Up and down']], 'y'), P.range('amp', 'Amplitude (px)', 0, 20, 0.1, 3), P.int('freq', 'Waves', 1, 16, 3), P.speed(1)],
    run(buf, W, H, t, p) {
      return remap(buf, W, H, (x, y, s) => {
        if (p.axis === 'x') { s[0] = x + p.amp * Math.sin(TAU * ((y / H) * p.freq - t * p.speed)); s[1] = y; }
        else { s[0] = x; s[1] = y + p.amp * Math.sin(TAU * ((x / W) * p.freq - t * p.speed)); }
      });
    },
  });
  effect('swirl', {
    name: 'Swirl', cat: 'Distortion', desc: 'Twists the image round a centre point.',
    params: [P.range('strength', 'Strength (turns)', -2, 2, 0.01, 0.5), P.range('radius', 'Radius', 0.05, 1.5, 0.01, 0.6), P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Wobbles per loop', 0, 8)],
    run(buf, W, H, t, p) {
      const s = p.strength * (p.speed ? Math.sin(TAU * t * p.speed) : 1), R = p.radius * H;
      return remap(buf, W, H, (x, y, out) => {
        const dx = x - p.cx * W, dy = y - p.cy * H, d = Math.hypot(dx, dy);
        const k = Math.max(0, 1 - d / R), a = s * TAU * k * k, c = Math.cos(a), sn = Math.sin(a);
        out[0] = p.cx * W + dx * c - dy * sn; out[1] = p.cy * H + dx * sn + dy * c;
      });
    },
  });
  effect('mirror', {
    name: 'Mirror', cat: 'Distortion', desc: 'Mirror or kaleidoscope the image.',
    params: [P.select('mode', 'Mode', [['x', 'Left onto right'], ['y', 'Top onto bottom'], ['quad', 'Four-way'], ['kaleido', 'Kaleidoscope']], 'x'), P.int('segments', 'Kaleidoscope segments', 2, 16, 6), P.speed(0, 'spin', 'Spin')],
    run(buf, W, H, t, p) {
      if (p.mode === 'kaleido') {
        const seg = TAU / p.segments, cx = W / 2, cy = H / 2;
        return remap(buf, W, H, (x, y, s) => {
          const dx = x - cx, dy = y - cy;
          let a = Math.atan2(dy, dx) + TAU * t * p.spin;
          a = ((a % seg) + seg) % seg; if (a > seg / 2) a = seg - a;
          const r = Math.hypot(dx, dy);
          s[0] = cx + Math.cos(a) * r; s[1] = cy + Math.sin(a) * r;
        });
      }
      return remap(buf, W, H, (x, y, s) => {
        s[0] = p.mode === 'x' || p.mode === 'quad' ? (x > W / 2 ? W - x : x) : x;
        s[1] = p.mode === 'y' || p.mode === 'quad' ? (y > H / 2 ? H - y : y) : y;
      });
    },
  });
  effect('pixelate', {
    name: 'Pixelate', cat: 'Distortion', desc: 'Chunky pixel blocks.',
    params: [P.int('size', 'Block size (px)', 1, 24, 4)],
    run(buf, W, H, t, p) {
      const s = p.size; if (s <= 1) return buf;
      const out = new Float32Array(buf.length);
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const sx = Math.min(W - 1, Math.floor(x / s) * s + (s >> 1)), sy = Math.min(H - 1, Math.floor(y / s) * s + (s >> 1));
        const i = (y * W + x) * 4, j = (sy * W + sx) * 4;
        out[i] = buf[j]; out[i + 1] = buf[j + 1]; out[i + 2] = buf[j + 2]; out[i + 3] = buf[j + 3];
      }
      return out;
    },
  });
  effect('blur', {
    name: 'Blur', cat: 'Distortion', desc: 'Soft blur.',
    params: [P.int('radius', 'Radius (px)', 1, 12, 2)],
    run(buf, W, H, t, p) { return boxBlur(buf, W, H, p.radius); },
  });
  effect('chroma', {
    name: 'Chromatic split', cat: 'Distortion', desc: 'Red and blue channels drift apart.',
    params: [P.range('offset', 'Offset (px)', 0, 10, 0.1, 1.5), P.range('angle', 'Angle', 0, 360, 1, 0), P.speed(0, 'wobble', 'Wobbles per loop')],
    run(buf, W, H, t, p) {
      const k = p.wobble ? Math.sin(TAU * t * p.wobble) : 1, a = (p.angle * Math.PI) / 180;
      const dx = Math.cos(a) * p.offset * k, dy = Math.sin(a) * p.offset * k;
      const out = new Float32Array(buf), o = [0, 0, 0, 0];
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        bilinear(buf, W, H, x + 0.5 + dx, y + 0.5 + dy, o); out[i] = o[0]; const ar = o[3];
        bilinear(buf, W, H, x + 0.5 - dx, y + 0.5 - dy, o); out[i + 2] = o[2];
        out[i + 3] = Math.max(buf[i + 3], ar, o[3]);
      }
      return out;
    },
  });
  effect('glitch', {
    name: 'Glitch', cat: 'Distortion', desc: 'Random slices jump sideways with colour tearing.',
    params: [P.range('amount', 'Amount', 0, 1, 0.01, 0.5), P.int('slices', 'Slices', 2, 32, 10), P.int('rate', 'Changes per loop', 1, 30, 8), P.range('chance', 'How often', 0, 1, 0.01, 0.4), P.seed(91)],
    run(buf, W, H, t, p) {
      const step = Math.floor(t * p.rate);
      if (hash2(step, 0, p.seed) > p.chance) return buf;
      const out = new Float32Array(buf.length);
      for (let y = 0; y < H; y++) {
        const sl = Math.floor((y / H) * p.slices);
        const h = hash2(sl, step, p.seed + 1), on = h < p.amount;
        const sh = on ? Math.round((hash2(sl, step, p.seed + 2) - 0.5) * W * 0.3 * p.amount) : 0;
        const split = on ? Math.round(p.amount * 3) : 0;
        for (let x = 0; x < W; x++) {
          const i = (y * W + x) * 4, j = (y * W + wrap(x - sh, W)) * 4, jr = (y * W + wrap(x - sh + split, W)) * 4, jb = (y * W + wrap(x - sh - split, W)) * 4;
          out[i] = buf[jr]; out[i + 1] = buf[j + 1]; out[i + 2] = buf[jb + 2]; out[i + 3] = Math.max(buf[j + 3], buf[jr + 3], buf[jb + 3]);
        }
      }
      return out;
    },
  });
  effect('scanlines', {
    name: 'Scanlines', cat: 'Texture', desc: 'CRT lines, optionally rolling.',
    params: [P.range('spacing', 'Spacing (px)', 2, 12, 1, 3), P.range('dark', 'Darkness', 0, 1, 0.01, 0.35), P.speed(0, 'speed', 'Roll')],
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i, u, v, x, y) => { const f = fract(y / p.spacing - t * p.speed * (H / p.spacing)); const m = f < 0.5 ? p.dark : 0; for (let k = 0; k < 3; k++) buf[i + k] *= 1 - m; });
    },
  });
  effect('grain', {
    name: 'Film grain', cat: 'Texture', desc: 'Fine animated noise.',
    params: [P.range('amount', 'Amount', 0, 0.5, 0.01, 0.08), P.bool('animated', 'Animated', true), P.bool('mono', 'Monochrome', true)],
    run(buf, W, H, t, p, c) {
      const f = p.animated ? c.frame : 0;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        for (let k = 0; k < 3; k++) { const n = (hash2(x, y * 3 + (p.mono ? 0 : k), f + 1) - 0.5) * 2 * p.amount; buf[i + k] = clamp(buf[i + k] + n); if (p.mono) { buf[i + 1] = clamp(buf[i + 1] + n); buf[i + 2] = clamp(buf[i + 2] + n); break; } }
      });
    },
  });
  effect('halftone', {
    name: 'Halftone', cat: 'Texture', desc: 'Comic-print dots sized by brightness.',
    params: [P.int('size', 'Dot spacing (px)', 2, 16, 4), P.range('angle', 'Angle', 0, 90, 1, 45), P.color('ink', 'Ink', '#000000'), P.range('amount', 'Amount', 0, 1, 0.01, 0.8)],
    prepare: colorPrep(['ink']),
    run(buf, W, H, t, p) {
      const a = (p.angle * Math.PI) / 180, c = Math.cos(a), s = Math.sin(a);
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const rx = (x * c - y * s) / p.size, ry = (x * s + y * c) / p.size;
        const d = Math.hypot(fract(rx) - 0.5, fract(ry) - 0.5);
        const l = luma(buf[i], buf[i + 1], buf[i + 2]);
        const m = (1 - smoothstep((1 - l) * 0.6 - 0.05, (1 - l) * 0.6 + 0.05, d)) * p.amount;
        for (let k = 0; k < 3; k++) buf[i + k] = mix(buf[i + k], p._ink[k], m);
      });
    },
  });

  // ---------- blending ----------
  const BLENDS = [['normal', 'Normal'], ['multiply', 'Multiply'], ['screen', 'Screen'], ['overlay', 'Overlay'], ['add', 'Add'], ['softlight', 'Soft light'], ['hardlight', 'Hard light'], ['difference', 'Difference'], ['dodge', 'Colour dodge'], ['burn', 'Colour burn'], ['lighten', 'Lighten'], ['darken', 'Darken'], ['hue', 'Hue'], ['color', 'Colour'], ['mask', 'Mask (keep below where this is)'], ['erase', 'Erase (cut out)']];
  const sep = {
    normal: (b, s) => s, multiply: (b, s) => b * s, screen: (b, s) => b + s - b * s,
    overlay: (b, s) => (b < 0.5 ? 2 * b * s : 1 - 2 * (1 - b) * (1 - s)), hardlight: (b, s) => (s < 0.5 ? 2 * b * s : 1 - 2 * (1 - b) * (1 - s)),
    add: (b, s) => Math.min(1, b + s), softlight: (b, s) => (1 - 2 * s) * b * b + 2 * s * b, difference: (b, s) => Math.abs(b - s),
    dodge: (b, s) => (s >= 1 ? 1 : Math.min(1, b / (1 - s))), burn: (b, s) => (s <= 0 ? 0 : 1 - Math.min(1, (1 - b) / s)),
    lighten: Math.max, darken: Math.min,
  };
  PL.BLENDS = BLENDS;
  // Composite one straight-alpha pixel s (alpha as) onto buffer at i.
  function blendPx(buf, i, s, as, mode) {
    const ab = buf[i + 3];
    if (mode === 'erase') { buf[i + 3] = ab * (1 - as); return; }
    if (mode === 'mask') { buf[i + 3] = ab * as; return; }
    if (as <= 0) return;
    let c0, c1, c2;
    if (mode === 'hue' || mode === 'color') {
      const hb = rgbToHsl(buf[i], buf[i + 1], buf[i + 2]), hs = rgbToHsl(s[0], s[1], s[2]);
      [c0, c1, c2] = mode === 'hue' ? hslToRgb(hs[0], hb[1], hb[2]) : hslToRgb(hs[0], hs[1], hb[2]);
    } else {
      const f = sep[mode] || sep.normal;
      c0 = f(buf[i], s[0]); c1 = f(buf[i + 1], s[1]); c2 = f(buf[i + 2], s[2]);
    }
    // W3C compositing: Cs' = (1 - ab) * Cs + ab * B(Cb, Cs)
    const r = (1 - ab) * s[0] + ab * c0, g = (1 - ab) * s[1] + ab * c1, b = (1 - ab) * s[2] + ab * c2;
    const ao = as + ab * (1 - as);
    buf[i] = (as * r + ab * (1 - as) * buf[i]) / ao;
    buf[i + 1] = (as * g + ab * (1 - as) * buf[i + 1]) / ao;
    buf[i + 2] = (as * b + ab * (1 - as) * buf[i + 2]) / ao;
    buf[i + 3] = ao;
  }

  PL.LAYERS = LAYERS;
  PL.EFFECTS = EFFECTS;
  PL.blendPx = blendPx;
  PL.LAYER_CATS = ['Gradients', 'Patterns', 'Noise and fluids', 'Particles and light', 'Materials'];
  PL.EFFECT_CATS = ['Colour', 'Lighting and shape', 'Distortion', 'Texture'];
}
