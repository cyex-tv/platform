// @ts-nocheck
/* Pack: Retro, tech and glitch: screens, circuits, data, signal errors. Same contracts as js/library.js; see js/packs/README.md. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, smoothstep, tri, wrap, hexToRgb, lutAt, hash2, vnoise, fbm, ridged, luma, hsvToRgb } = PL;
  const P = PL.P;
  const { rgb01, colorPrep, eachPx, bilinear, remap, boxBlur } = PL.fx;
  const add = (store, id, def) => { if (store[id]) throw new Error('Duplicate layer or effect id: ' + id); def.id = id; def.pack = 'retro'; store[id] = def; };
  const layer = (id, def) => add(PL.LAYERS, id, def);
  const effect = (id, def) => add(PL.EFFECTS, id, def);
  for (const c of ['Retro and tech']) if (!PL.LAYER_CATS.includes(c)) PL.LAYER_CATS.push(c);
  for (const c of ['Glitch and retro']) if (!PL.EFFECT_CATS.includes(c)) PL.EFFECT_CATS.push(c);

  // ---- definitions go here ----
  const CAT = 'Retro and tech', ECAT = 'Glitch and retro';
  // Step index that wraps with the loop, so random-looking glitches still start and end on the same frame.
  const stepOf = (t, n) => Math.floor(t * n + 1e-9) % n;
  const setC = (o, c, a = 1) => { o[0] = c[0]; o[1] = c[1]; o[2] = c[2]; o[3] = a; return o; };
  const mixC = (o, a, b, f, al = 1) => { o[0] = mix(a[0], b[0], f); o[1] = mix(a[1], b[1], f); o[2] = mix(a[2], b[2], f); o[3] = al; return o; };
  const bayer8 = (x, y) => { x &= 7; y &= 7; const a = x ^ y; return (((a & 1) << 5) | ((x & 1) << 4) | ((a & 2) << 2) | ((x & 2) << 1) | ((a & 4) >> 1) | ((x & 4) >> 2)) / 64 + 1 / 128; };
  // Linear sample along a row at float x (wraps sideways).
  const lerpX = (buf, W, y, xf, k) => { const x0 = Math.floor(xf), f = xf - x0, r = y * W, a = (r + wrap(x0, W)) * 4 + k, b = (r + wrap(x0 + 1, W)) * 4 + k; return buf[a] + (buf[b] - buf[a]) * f; };

  // =====================================================================================
  // Layers
  // =====================================================================================
  const hEdge = (i, j, cols, seed) => hash2(wrap(i, cols), j, seed + 1) < 0.55;
  const vEdge = (i, j, cols, seed) => hash2(wrap(i, cols), j, seed + 2) < 0.4;
  layer('circuitTraces', {
    name: 'Circuit board', cat: CAT, desc: 'Green PCB traces with solder pads and bright pulses running along them.',
    tags: ['circuit', 'pcb', 'tech', 'traces', 'electronics', 'chip', 'board', 'pulse'],
    params: [P.color('bg', 'Board', '#06231a'), P.color('trace', 'Traces', '#1f8f5f'), P.color('pulse', 'Pulses', '#9dffd0'), P.int('cols', 'Cells across', 8, 64, 28), P.range('width', 'Trace width', 0.04, 0.3, 0.01, 0.14), P.range('len', 'Pulse length', 0.03, 0.6, 0.01, 0.14), P.speed(1), P.seed(7)],
    prepare: colorPrep(['bg', 'trace', 'pulse']),
    sample(u, v, t, p, o, c) {
      const cols = p.cols, sd = p.seed, gx = u * cols, gy = (v * cols) / c.A, ci = Math.floor(gx), cj = Math.floor(gy), lx = gx - ci - 0.5, ly = gy - cj - 0.5;
      const R = hEdge(ci, cj, cols, sd), L = hEdge(ci - 1, cj, cols, sd), D = vEdge(ci, cj, cols, sd), U = vEdge(ci, cj - 1, cols, sd);
      const hw = p.width / 2, ax = Math.abs(lx), ay = Math.abs(ly);
      const hor = ay < hw && ((lx >= 0 && R) || (lx < 0 && L)), ver = ax < hw && ((ly >= 0 && D) || (ly < 0 && U));
      const deg = (R ? 1 : 0) + (L ? 1 : 0) + (D ? 1 : 0) + (U ? 1 : 0), rr = Math.hypot(lx, ly);
      const node = deg > 0 && (deg !== 2 || hash2(wrap(ci, cols), cj, sd + 3) < 0.3);
      if (node && rr < hw * 0.55) return setC(o, p._bg, 1);
      const pad = node && rr < hw * 1.7;
      if (!hor && !ver && !pad) return setC(o, p._bg, 1);
      let a = 0;
      if (hor) {
        const dir = hash2(0, cj, sd + 4) < 0.5 ? 1 : -1, sp = p.speed * dir, head = fract(t * sp + hash2(1, cj, sd + 5));
        const d = sp >= 0 ? fract(head - u) : fract(u - head);
        if (d < p.len) a = Math.max(a, (1 - d / p.len) * (1 - d / p.len));
      }
      if (ver) {
        const col = wrap(ci, cols), dir = hash2(col, 0, sd + 6) < 0.5 ? 1 : -1, sp = p.speed * dir, head = fract(t * sp + hash2(col, 1, sd + 7));
        const d = sp >= 0 ? fract(head - v) : fract(v - head);
        if (d < p.len) a = Math.max(a, (1 - d / p.len) * (1 - d / p.len));
      }
      return mixC(o, p._trace, p._pulse, a, 1);
    },
  });

  layer('hexStream', {
    name: 'Hex data stream', cat: CAT, desc: 'Rows of tiny blocky glyphs scrolling past like a memory dump, in opposite directions.',
    tags: ['hex', 'dump', 'data', 'code', 'terminal', 'stream', 'hacker', 'memory'],
    params: [P.color('bg', 'Background', '#03120f'), P.color('ink', 'Glyphs', '#2fd6a0'), P.color('hi', 'Highlight', '#e4fff4'), P.int('rows', 'Rows', 2, 12, 6), P.int('cols', 'Glyphs across', 8, 80, 36), P.range('bright', 'Highlights', 0, 0.3, 0.01, 0.07), P.speed(1), P.seed(11)],
    prepare: colorPrep(['bg', 'ink', 'hi']),
    sample(u, v, t, p, o) {
      const rows = p.rows, cols = p.cols, row = Math.min(rows - 1, Math.floor(v * rows)), ly = v * rows - row;
      const dir = hash2(row, 0, p.seed) < 0.5 ? 1 : -1, k = 1 + ((hash2(row, 1, p.seed) * 2) | 0);
      const gx = u * cols - dir * k * p.speed * t * cols, ci = Math.floor(gx), fx = gx - ci;
      const sx = Math.floor(fx * 4), sy = Math.floor(ly * 6), col = wrap(ci, cols);
      if (sx > 2 || sy > 4 || col % 3 === 2) return setC(o, p._bg, 1);
      const on = hash2(col * 7 + sx, row * 5 + sy, p.seed + 3) > 0.5;
      if (!on) return setC(o, p._bg, 1);
      const hot = hash2(col, row, p.seed + 9) < p.bright;
      return mixC(o, p._bg, hot ? p._hi : p._ink, row % 2 ? 0.6 : 1, 1);
    },
  });

  const binGlyph = (lx, ly, one) => {
    if (one) return (lx > 0.43 && lx < 0.6 && ly > 0.12 && ly < 0.88) || (lx > 0.25 && lx <= 0.43 && ly > 0.12 && ly < 0.28);
    return lx > 0.22 && lx < 0.78 && ly > 0.12 && ly < 0.88 && !(lx > 0.4 && lx < 0.6 && ly > 0.3 && ly < 0.7);
  };
  layer('binaryRain', {
    name: 'Binary rain', cat: CAT, desc: 'Falling streams of ones and zeros in two depth layers, fading into the dark.',
    tags: ['binary', 'rain', 'code', 'digits', 'hacker', 'cyber', 'falling', 'bits'],
    params: [P.color('head', 'Head', '#e8ffff'), P.color('trail', 'Trail', '#19c6d9'), P.color('bg', 'Background', '#02161c'), P.int('cols', 'Columns', 6, 60, 26), P.int('rows', 'Digit rows', 3, 20, 7), P.range('depth', 'Back layer', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Fall speed', 1, 6), P.seed(23)],
    prepare: colorPrep(['head', 'trail', 'bg']),
    sample(u, v, t, p, o) {
      let best = 0, hd = 0, dim = 1;
      for (let layerK = 0; layerK < 2; layerK++) {
        const cols = layerK ? p.cols * 2 : p.cols, rows = layerK ? p.rows * 2 : p.rows, sd = p.seed + layerK * 100;
        const col = Math.floor(u * cols), lx = u * cols - col, row = Math.floor(v * rows), ly = v * rows - row;
        const h = hash2(col, 0, sd), k = 1 + ((hash2(col, 1, sd) * 2) | 0), len = 0.25 + 0.5 * hash2(col, 2, sd);
        const head = fract(t * p.speed * k + h), d = fract(head - (row + 0.5) / rows);
        if (d >= len) continue;
        const flip = hash2(col, row, sd + stepOf(t, 6) * 3) > 0.5;
        if (!binGlyph(lx, ly, flip)) continue;
        const a = (1 - d / len) * (layerK ? p.depth : 1);
        if (a > best) { best = a; hd = d < 1 / rows ? 1 : 0; dim = layerK ? 0.7 : 1; }
      }
      const f = Math.pow(best, 0.8);
      const cc = hd ? p._head : p._trail;
      return mixC(o, p._bg, cc, f * dim, 1);
    },
  });

  layer('radarSweep', {
    name: 'Radar sweep', cat: CAT, desc: 'Round radar screens with a rotating beam and blips that light up as it passes.',
    tags: ['radar', 'sonar', 'sweep', 'scope', 'green', 'military', 'blip', 'tech'],
    params: [P.color('bg', 'Screen', '#04140c'), P.color('grid', 'Grid', '#1b6b3a'), P.color('sweep', 'Beam', '#5dff9a'), P.int('count', 'Radars across', 1, 8, 4), P.int('blips', 'Blips', 0, 6, 3), P.range('trail', 'Beam trail', 0.1, 1, 0.01, 0.45), P.speed(1), P.seed(5)],
    prepare: colorPrep(['bg', 'grid', 'sweep']),
    sample(u, v, t, p, o, c) {
      const n = p.count, ri = Math.min(n - 1, Math.floor(u * n)), cw = c.A / n;
      const x = (u * n - ri - 0.5) * cw, y = v - 0.5, r = Math.hypot(x, y), R = 0.46;
      if (r > R) return setC(o, p._bg, 1);
      const ang = Math.atan2(y, x) / TAU + 0.5, head = fract(t * p.speed + hash2(ri, 0, p.seed) * 0.5);
      const fw = p.speed >= 0 ? 1 : -1;
      const behind = (a) => (fw > 0 ? fract(head - a) : fract(a - head));
      let grid = 0;
      for (let k = 1; k <= 3; k++) if (Math.abs(r - (R * k) / 3) < 0.008) grid = 1;
      if (Math.abs(x) < 0.005 || Math.abs(y) < 0.005) grid = 1;
      const d = behind(ang);
      let beam = d < p.trail ? Math.pow(1 - d / p.trail, 2) : 0;
      if (d < 0.006) beam = 1;
      let blip = 0;
      for (let k = 0; k < p.blips; k++) {
        const ba = hash2(ri, k + 1, p.seed + 11), br = 0.08 + 0.34 * hash2(ri, k + 1, p.seed + 12);
        const bx = Math.cos(TAU * (ba - 0.5)) * br, by = Math.sin(TAU * (ba - 0.5)) * br;
        const dd = Math.hypot(x - bx, y - by);
        if (dd < 0.035) blip = Math.max(blip, Math.pow(1 - behind(ba), 3) * (1 - dd / 0.035));
      }
      let rr = p._bg[0], gg = p._bg[1], bb = p._bg[2];
      const g = grid * 0.55;
      rr = mix(rr, p._grid[0], g); gg = mix(gg, p._grid[1], g); bb = mix(bb, p._grid[2], g);
      const s = clamp(beam * 0.75 + blip);
      o[0] = mix(rr, p._sweep[0], s); o[1] = mix(gg, p._sweep[1], s); o[2] = mix(bb, p._sweep[2], s); o[3] = 1;
      return o;
    },
  });

  layer('oscilloscope', {
    name: 'Oscilloscope', cat: CAT, desc: 'A glowing phosphor waveform sliding across a graticule. Pick sine, square or mixed.',
    tags: ['oscilloscope', 'waveform', 'signal', 'sine', 'audio', 'lab', 'green', 'crt'],
    params: [P.color('bg', 'Screen', '#021208'), P.color('grid', 'Graticule', '#0f5a30'), P.color('trace', 'Trace', '#7dffa8'), P.select('shape', 'Wave', [['sine', 'Sine'], ['square', 'Square'], ['mix', 'Mixed']], 'mix'), P.int('waves', 'Waves across', 1, 12, 3), P.range('amp', 'Height', 0.05, 0.45, 0.01, 0.27), P.range('thick', 'Thickness (px)', 0.4, 4, 0.1, 1.2), P.speed(1)],
    prepare: colorPrep(['bg', 'grid', 'trace']),
    sample(u, v, t, p, o, c) {
      const fn = (uu) => {
        const a = TAU * (p.waves * uu - t * p.speed);
        if (p.shape === 'sine') return p.amp * Math.sin(a);
        if (p.shape === 'square') return p.amp * 0.78 * (Math.sin(a) + Math.sin(3 * a) / 3 + Math.sin(5 * a) / 5);
        return p.amp * (0.62 * Math.sin(a) + 0.3 * Math.sin(3 * a + TAU * t) + 0.12 * Math.sin(7 * a));
      };
      const e = 0.5 / c.W, yv = fn(u), slope = ((fn(u + e) - fn(u - e)) / (2 * e)) * (c.H / c.W);
      const dpx = (Math.abs(v - 0.5 - yv) * c.H) / Math.sqrt(1 + slope * slope), th = p.thick;
      const core = Math.exp(-(dpx * dpx) / (th * th)), glow = 0.3 * Math.exp(-dpx / (th * 3));
      const gl = (Math.abs(fract(u * 8 + 0.5) - 0.5) < 0.012 || Math.abs(fract(v * 4 + 0.5) - 0.5) < 0.03 ? 0.55 : 0) + (Math.abs(v - 0.5) < 0.012 ? 0.3 : 0);
      const s = clamp(core + glow);
      for (let k = 0; k < 3; k++) o[k] = clamp(mix(p._bg[k], p._grid[k], Math.min(1, gl)) + p._trace[k] * s);
      o[3] = 1;
      return o;
    },
  });

  layer('equalizer', {
    name: 'Equalizer bars', cat: CAT, desc: 'Segmented LED level bars bouncing to an invisible beat, with floating peak caps.',
    tags: ['equalizer', 'audio', 'music', 'bars', 'spectrum', 'led', 'level', 'vu'],
    params: [P.pal('toxic'), P.color('bg', 'Background', '#07070d'), P.int('bars', 'Bars', 4, 64, 24), P.int('segments', 'Segments', 4, 24, 10), P.range('gap', 'Gap', 0.1, 0.6, 0.01, 0.28), P.bool('peaks', 'Peak caps', true), P.speed(1), P.seed(3)],
    prepare: colorPrep(['bg']),
    sample(u, v, t, p, o) {
      const bar = Math.min(p.bars - 1, Math.floor(u * p.bars)), lx = u * p.bars - bar, yy = 1 - v, seg = Math.min(p.segments - 1, Math.floor(yy * p.segments)), ly = yy * p.segments - seg;
      const h1 = hash2(bar, 1, p.seed), h2 = hash2(bar, 2, p.seed), k1 = 1 + ((h1 * 2) | 0), k2 = 2 + ((h2 * 3) | 0);
      const lev = clamp(0.5 + 0.5 * (0.6 * Math.sin(TAU * (k1 * t * p.speed + h1)) + 0.4 * Math.sin(TAU * (k2 * t * p.speed + h2))));
      const L = 0.12 + 0.82 * lev, peak = Math.min(1, L + 0.1 + 0.1 * (0.5 + 0.5 * Math.sin(TAU * (t * p.speed + h2))));
      const g = p.gap / 2;
      if (lx < g || lx > 1 - g || ly < 0.12 || ly > 0.88) return setC(o, p._bg, 1);
      lutAt(p._stops, p.segments > 1 ? seg / (p.segments - 1) : 0, o);
      const lit = (seg + 1) / p.segments <= L + 1e-6 || (p.peaks && seg === Math.min(p.segments - 1, Math.floor(peak * p.segments)));
      const f = lit ? 1 : 0.1;
      o[0] = mix(p._bg[0], o[0], f); o[1] = mix(p._bg[1], o[1], f); o[2] = mix(p._bg[2], o[2], f); o[3] = 1;
      return o;
    },
  });

  layer('ledMarquee', {
    name: 'LED marquee', cat: CAT, desc: 'A round-dot LED sign scrolling blocky unreadable text, with unlit dots left glowing faintly.',
    tags: ['led', 'marquee', 'dot matrix', 'sign', 'scroll', 'ticker', 'amber', 'display'],
    params: [P.color('on', 'Lit', '#ff9a1f'), P.color('off', 'Unlit', '#2a1405'), P.color('bg', 'Background', '#0a0503'), P.int('rows', 'Dot rows', 4, 24, 10), P.range('density', 'Fill', 0.2, 0.8, 0.01, 0.5), P.speed(1), P.seed(17)],
    prepare: colorPrep(['on', 'off', 'bg']),
    sample(u, v, t, p, o, c) {
      const rows = p.rows, cols = Math.max(4, Math.round((rows * c.A) / 4) * 4), row = Math.min(rows - 1, Math.floor(v * rows));
      const gx = Math.floor(u * cols), fx = u * cols - gx, fy = v * rows - row, col = wrap(gx + Math.floor(t * p.speed * cols), cols), gi = Math.floor(col / 4);
      const lit = col % 4 !== 3 && gi % 6 !== 5 && hash2(col % 4 + gi * 3, row, p.seed) < p.density;
      const ratio = (c.A * rows) / cols, dx = (fx - 0.5) * ratio, dy = fy - 0.5, d = Math.hypot(dx, dy), s = 1 - smoothstep(0.34, 0.42, d);
      const base = lit ? p._on : p._off;
      mixC(o, p._bg, base, s, 1);
      if (lit) { const gl = Math.exp(-d * 5) * 0.25; o[0] = clamp(o[0] + p._on[0] * gl); o[1] = clamp(o[1] + p._on[1] * gl); o[2] = clamp(o[2] + p._on[2] * gl); }
      return o;
    },
  });

  layer('lcdSubpixels', {
    name: 'LCD subpixels', cat: CAT, desc: 'A macro view of an LCD panel: red, green and blue stripes carrying a slow colour wash.',
    tags: ['lcd', 'subpixel', 'rgb', 'screen', 'pixels', 'monitor', 'macro', 'stripes'],
    params: [P.pal('rainbow', true), P.int('pixels', 'Pixels across', 12, 128, 48), P.range('gap', 'Black gap', 0, 0.5, 0.01, 0.2), P.int('bands', 'Colour bands', 1, 6, 2), P.speed(1)],
    sample(u, v, t, p, o, c) {
      lutAt(p._stops, fract(u * p.bands + v * 0.25 - t * p.speed), o);
      const n = p.pixels, ch = Math.min(2, Math.floor(fract(u * n) * 3)), sx = fract(u * n * 3), sy = fract((v * n) / c.A), g = p.gap / 2;
      const mx = smoothstep(g, g + 0.1, sx) * (1 - smoothstep(1 - g - 0.1, 1 - g, sx)), my = smoothstep(g * 0.6, g * 0.6 + 0.1, sy) * (1 - smoothstep(1 - g * 0.6 - 0.1, 1 - g * 0.6, sy));
      const m = mx * my, r = o[0], gr = o[1], b = o[2], src = [r, gr, b];
      for (let k = 0; k < 3; k++) o[k] = clamp(src[k] * (k === ch ? 1.15 : 0.05) * m);
      o[3] = 1;
      return o;
    },
  });

  layer('dither8bit', {
    name: '8-bit sky', cat: CAT, desc: 'A chunky pixel sky of banded colours with checkered dithering between bands, cycling slowly.',
    tags: ['8-bit', 'sky', 'dither', 'pixel', 'gradient', 'retro', 'game', 'bands'],
    params: [P.pal('sunset', true), P.int('bands', 'Colour bands', 3, 16, 7), P.int('rows', 'Pixel rows', 8, 64, 28), P.range('repeat', 'Repeats', 0.5, 3, 0.5, 1), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const rows = p.rows, cols = Math.round(rows * c.A), gx = Math.floor(u * cols), gy = Math.floor(v * rows), vq = (gy + 0.5) / rows;
      const F = fract(vq * p.repeat - t * p.speed) * p.bands, b = Math.floor(F), fr = F - b, nb = b + (fr > bayer8(gx, gy) ? 1 : 0);
      return lutAt(p._stops, fract((nb % p.bands + 0.5) / p.bands), o);
    },
  });

  layer('synthSun', {
    name: 'Synthwave sun', cat: CAT, desc: 'A yellow to pink sun with horizontal cuts that crawl down it, glowing over a purple dusk.',
    tags: ['synthwave', 'sun', 'outrun', 'vaporwave', '80s', 'retrowave', 'sunset', 'neon'],
    params: [P.color('top', 'Sun top', '#ffe14a'), P.color('bottom', 'Sun bottom', '#ff2d95'), P.color('skyTop', 'Sky top', '#12012b'), P.color('skyBottom', 'Sky bottom', '#4a0a6b'), P.int('count', 'Suns across', 1, 4, 2), P.int('bars', 'Cut bars', 3, 16, 7), P.range('cut', 'Cut depth', 0.2, 0.9, 0.01, 0.6), P.range('glow', 'Glow', 0, 1, 0.01, 0.6), P.speed(1)],
    prepare: colorPrep(['top', 'bottom', 'skyTop', 'skyBottom']),
    sample(u, v, t, p, o, c) {
      const n = p.count, ri = Math.min(n - 1, Math.floor(u * n)), x = (u * n - ri - 0.5) * (c.A / n), y = v - 0.46, R = 0.4, d = Math.hypot(x, y);
      const sky = [mix(p._skyTop[0], p._skyBottom[0], v), mix(p._skyTop[1], p._skyBottom[1], v), mix(p._skyTop[2], p._skyBottom[2], v)];
      const g = Math.exp(-Math.max(0, d - R) * 7) * p.glow * 0.7;
      for (let k = 0; k < 3; k++) sky[k] = clamp(sky[k] + p._bottom[k] * g);
      const sy = clamp((y + R) / (2 * R));
      let inside = 1 - smoothstep(R - 0.012, R, d);
      const cs = 1 - p.cut;
      if (sy > cs) {
        const q = (sy - cs) / p.cut, ph = fract(q * p.bars - t * p.speed);
        if (ph < q * 0.75) inside = 0;
      }
      for (let k = 0; k < 3; k++) o[k] = mix(sky[k], mix(p._top[k], p._bottom[k], sy), inside);
      o[3] = 1;
      return o;
    },
  });

  layer('seventiesStripes', {
    name: '70s stripes', cat: CAT, desc: 'Flat bands of warm orange, brown and cream swaying in slow waves, with thin dark outlines.',
    tags: ['70s', 'retro', 'stripes', 'sunset', 'orange', 'seventies', 'groovy', 'waves'],
    params: [P.pal('sunset', true), P.int('bands', 'Bands', 3, 14, 6), P.range('sway', 'Sway', 0, 0.4, 0.01, 0.12), P.int('waves', 'Waves across', 1, 8, 2), P.color('line', 'Outline', '#2b0d12'), P.range('lineWidth', 'Outline width', 0, 0.3, 0.01, 0.07), P.speed(1)],
    prepare: colorPrep(['line']),
    sample(u, v, t, p, o) {
      const f = v * 1.2 + p.sway * Math.sin(TAU * (u * p.waves + t * p.speed));
      const s = fract(f) * p.bands, i = Math.floor(s), fr = s - i;
      if (Math.min(fr, 1 - fr) < p.lineWidth / 2) return setC(o, p._line, 1);
      // colours roll through the palette once per loop
      lutAt(p._stops, fract((i + 0.5) / p.bands + t * p.speed), o);
      o[3] = 1;
      return o;
    },
  });

  layer('outrunRoad', {
    name: 'Outrun road', cat: CAT, desc: 'A neon grid road rushing toward a glowing horizon, with perspective lanes under a dark sky.',
    tags: ['outrun', 'road', 'perspective', 'grid', 'synthwave', 'neon', 'horizon', '80s', 'highway'],
    params: [P.color('sky', 'Sky', '#0a0020'), P.color('glow', 'Horizon glow', '#ff2bd6'), P.color('line', 'Lines', '#2de2ff'), P.color('ground', 'Ground', '#12002e'), P.range('horizon', 'Horizon', 0.2, 0.7, 0.01, 0.4), P.int('lanes', 'Lanes', 4, 40, 14), P.int('depth', 'Depth lines', 3, 24, 9), P.range('width', 'Line width (px)', 0.5, 3, 0.1, 1.3), P.speed(1)],
    prepare: colorPrep(['sky', 'glow', 'line', 'ground']),
    sample(u, v, t, p, o, c) {
      const hz = p.horizon, hg = Math.exp(-Math.abs(v - hz) * 16);
      if (v <= hz) {
        const f = v / hz;
        for (let k = 0; k < 3; k++) o[k] = clamp(mix(p._sky[k], p._glow[k], f * f * 0.5) + p._glow[k] * hg * 0.6);
        o[3] = 1;
        return o;
      }
      const dv = v - hz, z = (1 - hz) / dv, fade = clamp(dv / (0.2 * (1 - hz)));
      const ph = z * p.depth * 0.25 + t * p.speed, dpv = (p.depth * 0.25 * z * z) / (1 - hz), wph = Math.max(1e-4, (dpv * p.width) / c.H);
      const fp = fract(ph), dist1 = Math.min(fp, 1 - fp), a1 = 1 - smoothstep(0, wph, dist1);
      const q = (u - 0.5) * z * p.lanes, fq = fract(q + 0.5), dist2 = Math.min(fq, 1 - fq), wq = Math.max(1e-4, (p.lanes * z * p.width) / c.W), a2 = 1 - smoothstep(0, wq, dist2);
      const a = clamp(Math.max(Math.min(a1, 1), a2) * fade);
      for (let k = 0; k < 3; k++) o[k] = clamp(mix(p._ground[k], p._line[k], a) + p._glow[k] * hg * 0.7);
      o[3] = 1;
      return o;
    },
  });

  layer('loadingBars', {
    name: 'Loading bars', cat: CAT, desc: 'Stacked progress bars filling up with moving barber-pole stripes, then starting over.',
    tags: ['loading', 'progress', 'bar', 'stripes', 'barber pole', 'ui', 'download', 'installer'],
    params: [P.color('track', 'Track', '#0d1020'), P.color('fill', 'Fill', '#37d67a'), P.color('fill2', 'Stripe', '#1fa85a'), P.int('bars', 'Bars', 1, 6, 3), P.int('stripes', 'Stripes across', 4, 64, 24), P.range('gap', 'Gap', 0, 0.5, 0.01, 0.25), P.int('blocks', 'Blocks (0 = smooth)', 0, 40, 0), P.speed(1)],
    prepare: colorPrep(['track', 'fill', 'fill2']),
    sample(u, v, t, p, o) {
      const bar = Math.min(p.bars - 1, Math.floor(v * p.bars)), lv = v * p.bars - bar, g = p.gap / 2;
      if (lv < g || lv > 1 - g) return setC(o, [p._track[0] * 0.5, p._track[1] * 0.5, p._track[2] * 0.5], 1);
      const pr = fract(t * p.speed + bar * 0.3), prog = pr * pr * (3 - 2 * pr);
      const inside = u < prog && !(p.blocks > 0 && fract(u * p.blocks) < 0.14);
      if (!inside) return setC(o, p._track, 1);
      const ph = fract(u * p.stripes + lv - t * p.speed);
      return setC(o, ph < 0.5 ? p._fill : p._fill2, 1);
    },
  });

  layer('barcodeScroll', {
    name: 'Barcode scroll', cat: CAT, desc: 'Black and white barcode strips sliding sideways, with an optional red scanner laser sweeping over.',
    tags: ['barcode', 'scan', 'laser', 'retail', 'stripes', 'price', 'lines', 'scanner'],
    params: [P.color('ink', 'Bars', '#101018'), P.color('paper', 'Paper', '#f2efe6'), P.color('laser', 'Laser', '#ff2a3a'), P.int('units', 'Units across', 48, 400, 160), P.int('rows', 'Rows', 1, 4, 2), P.bool('showLaser', 'Laser', true), P.speed(1), P.seed(31)],
    prepare(p) {
      colorPrep(['ink', 'paper', 'laser'])(p);
      const n = p.units, bits = new Uint8Array(n), r = PL.rng(p.seed);
      let i = 0, on = 1;
      while (i < n) { const w = 1 + Math.floor(r() * 4); for (let k = 0; k < w && i < n; k++) bits[i++] = on; on ^= 1; }
      p._bits = bits;
    },
    sample(u, v, t, p, o) {
      const rows = p.rows, row = Math.min(rows - 1, Math.floor(v * rows)), ly = v * rows - row;
      if (ly < 0.07 || ly > 0.93) return setC(o, p._paper, 1);
      const dir = row % 2 ? -1 : 1, idx = wrap(Math.floor(u * p.units - dir * t * p.speed * p.units) + row * 37, p.units);
      setC(o, p._bits[idx] ? p._ink : p._paper, 1);
      if (p.showLaser) {
        const yl = tri(t), a = Math.exp(-Math.pow((v - yl) / 0.06, 2) * 3);
        o[0] = mix(o[0], p._laser[0], a * 0.85); o[1] = mix(o[1], p._laser[1], a * 0.85); o[2] = mix(o[2], p._laser[2], a * 0.85);
      }
      return o;
    },
  });

  const HEART = [0x66, 0xff, 0xff, 0xff, 0x7e, 0x3c, 0x18, 0x00], STAR = [0x18, 0x18, 0xff, 0x7e, 0x3c, 0x7e, 0x66, 0x00];
  layer('pixelSprites', {
    name: 'Pixel hearts and stars', cat: CAT, desc: 'A tile of 8-bit hearts and stars in candy colours, rows scrolling in opposite directions.',
    tags: ['pixel', 'hearts', 'stars', '8-bit', 'sprites', 'tile', 'game', 'cute', 'retro'],
    params: [P.pal('candy', true), P.color('bg', 'Background', '#14102a'), P.select('shape', 'Sprites', [['mixed', 'Hearts and stars'], ['hearts', 'Hearts'], ['stars', 'Stars']], 'mixed'), P.int('rows', 'Rows', 1, 8, 3), P.speed(1), P.seed(13)],
    prepare: colorPrep(['bg']),
    sample(u, v, t, p, o, c) {
      const rows = p.rows, cols = Math.max(1, Math.round(rows * c.A)), row = Math.min(rows - 1, Math.floor(v * rows)), dir = row % 2 ? -1 : 1;
      const gx = u * cols - dir * t * p.speed * cols, ci = Math.floor(gx), fx = gx - ci, fy = v * rows - row;
      const sx = Math.floor(fx * 10) - 1, sy = Math.floor(fy * 10) - 1;
      if (sx < 0 || sx > 7 || sy < 0 || sy > 7) return setC(o, p._bg, 1);
      const col = wrap(ci, cols), h = hash2(col, row, p.seed);
      const heart = p.shape === 'hearts' || (p.shape === 'mixed' && h < 0.5);
      const bits = heart ? HEART[sy] : STAR[sy];
      if (!((bits >> (7 - sx)) & 1)) return setC(o, p._bg, 1);
      lutAt(p._stops, fract(hash2(col, row, p.seed + 5)), o);
      if (sy < 2 && sx < 4) { o[0] = mix(o[0], 1, 0.35); o[1] = mix(o[1], 1, 0.35); o[2] = mix(o[2], 1, 0.35); }
      o[3] = 1;
      return o;
    },
  });

  const BARS = [[1, 1, 1], [1, 1, 0], [0, 1, 1], [0, 1, 0], [1, 0, 1], [1, 0, 0], [0, 0, 1]], MID = [[0, 0, 1], [0, 0, 0], [1, 0, 1], [0, 0, 0], [0, 1, 1], [0, 0, 0], [1, 1, 1]];
  layer('testCard', {
    name: 'Colour bars', cat: CAT, desc: 'Broadcast test-card colour bars with a grey ramp underneath, sliding sideways with a bright rolling line.',
    tags: ['test card', 'color bars', 'smpte', 'tv', 'broadcast', 'crt', 'signal', 'retro'],
    params: [P.range('level', 'Brightness', 0.5, 1, 0.01, 0.78), P.int('reps', 'Repeats', 1, 4, 1), P.bool('lower', 'Lower strips', true), P.speed(1), P.speed(1, 'roll', 'Line rolls', 0, 4)],
    sample(u, v, t, p, o) {
      const n = 7 * p.reps, idx = wrap(Math.floor(u * n - t * p.speed * n), 7);
      let r, g, b;
      if (!p.lower || v < 0.66) { [r, g, b] = BARS[idx]; r *= p.level; g *= p.level; b *= p.level; }
      else if (v < 0.8) { [r, g, b] = MID[idx]; r *= p.level; g *= p.level; b *= p.level; }
      else { const ramp = fract(u * p.reps - t * p.speed); r = g = b = ramp; }
      const d = Math.abs(fract(v - t * p.roll + 0.5) - 0.5), l = Math.exp(-Math.pow(d / 0.03, 2)) * 0.28;
      o[0] = clamp(r + l); o[1] = clamp(g + l); o[2] = clamp(b + l); o[3] = 1;
      return o;
    },
  });

  // =====================================================================================
  // Effects
  // =====================================================================================
  effect('rgbWobble', {
    name: 'RGB wobble', cat: ECAT, desc: 'Red and blue slide apart in a wave that ripples down the image, like a failing cable.',
    tags: ['rgb', 'split', 'wobble', 'chromatic', 'glitch', 'aberration', 'wave', 'analog'],
    params: [P.range('amount', 'Split (px)', 0, 12, 0.1, 3), P.range('waves', 'Waves down', 0.5, 8, 0.5, 2), P.speed(1)],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf.length);
      for (let y = 0; y < H; y++) {
        const s = p.amount * Math.sin(TAU * ((y / H) * p.waves - t * p.speed));
        for (let x = 0; x < W; x++) {
          const i = (y * W + x) * 4;
          out[i] = lerpX(buf, W, y, x + s, 0); out[i + 1] = buf[i + 1]; out[i + 2] = lerpX(buf, W, y, x - s, 2); out[i + 3] = buf[i + 3];
        }
      }
      return out;
    },
  });

  effect('pixelSort', {
    name: 'Pixel sort', cat: ECAT, desc: 'Runs of bright pixels are sorted by brightness, smearing light streaks along each row.',
    tags: ['pixel sort', 'smear', 'glitch', 'sort', 'streak', 'datamosh', 'brightness', 'art'],
    params: [P.range('thresh', 'Brightness threshold', 0.1, 0.9, 0.01, 0.4), P.range('sweep', 'Threshold sweep', 0, 0.4, 0.01, 0.2), P.int('maxlen', 'Longest run (px)', 8, 256, 64), P.range('amount', 'Rows sorted', 0, 1, 0.01, 0.75), P.select('dir', 'Order', [['up', 'Dark to bright'], ['down', 'Bright to dark']], 'up'), P.speed(1), P.seed(19)],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf), lum = new Float32Array(W), run = [], sgn = p.dir === 'up' ? 1 : -1;
      for (let y = 0; y < H; y++) {
        if (hash2(y, 0, p.seed) >= p.amount) continue;
        const thr = p.thresh + p.sweep * Math.sin(TAU * (t * p.speed - y / H));
        for (let x = 0; x < W; x++) { const i = (y * W + x) * 4; lum[x] = luma(buf[i], buf[i + 1], buf[i + 2]); }
        let x = 0;
        while (x < W) {
          if (lum[x] < thr) { x++; continue; }
          run.length = 0;
          let e = x;
          while (e < W && lum[e] >= thr && e - x < p.maxlen) run.push(e++);
          run.sort((a, b) => sgn * (lum[a] - lum[b]));
          for (let k = 0; k < run.length; k++) { const d = (y * W + x + k) * 4, s = (y * W + run[k]) * 4; out[d] = buf[s]; out[d + 1] = buf[s + 1]; out[d + 2] = buf[s + 2]; out[d + 3] = buf[s + 3]; }
          x = e;
        }
      }
      return out;
    },
  });

  effect('datamosh', {
    name: 'Datamosh drift', cat: ECAT, desc: 'Random blocks smear off in their own direction, then snap back at the next beat of the loop.',
    tags: ['datamosh', 'blocks', 'drift', 'compression', 'glitch', 'smear', 'video', 'broken'],
    params: [P.int('block', 'Block size (px)', 4, 32, 12), P.range('chance', 'Blocks affected', 0, 1, 0.01, 0.35), P.range('reach', 'Smear (px)', 0, 64, 1, 18), P.int('steps', 'Changes per loop', 1, 24, 6), P.seed(37)],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf.length), st = stepOf(t, p.steps), ph = fract(t * p.steps), B = p.block;
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const bx = Math.floor(x / B), by = Math.floor(y / B), sd = p.seed + st * 7, i = (y * W + x) * 4;
        let sx = x, sy = y;
        if (hash2(bx, by, sd) < p.chance) { sx = x - Math.round((hash2(bx, by, sd + 1) - 0.5) * 2 * p.reach * ph); sy = y - Math.round((hash2(bx, by, sd + 2) - 0.5) * p.reach * 0.5 * ph); }
        const j = (wrap(sy, H) * W + wrap(sx, W)) * 4;
        out[i] = buf[j]; out[i + 1] = buf[j + 1]; out[i + 2] = buf[j + 2]; out[i + 3] = buf[j + 3];
      }
      return out;
    },
  });

  effect('bitcrush', {
    name: 'Bitcrush', cat: ECAT, desc: 'Cuts colour depth and holds pixels in uneven runs, like a low sample rate turned image.',
    tags: ['bitcrush', 'lofi', 'sample hold', 'crush', 'low bit', 'digital', 'glitch', 'noise'],
    params: [P.int('bits', 'Levels per channel', 2, 32, 6), P.int('hold', 'Hold (px)', 1, 32, 6), P.range('jitter', 'Uneven holds', 0, 1, 0.01, 0.6), P.speed(1, 'wobble', 'Wobble'), P.seed(41)],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf.length), n = p.bits, q = (v) => clamp(Math.floor(v * n) / (n - 1));
      for (let y = 0; y < H; y++) {
        let x = 0, r = 0;
        while (x < W) {
          const len = Math.max(1, Math.round(p.hold * (1 + p.jitter * (hash2(r++, y, p.seed) - 0.5) * 2) * (1 + 0.3 * Math.sin(TAU * (p.wobble * t + y / H))))), j = (y * W + x) * 4;
          const cr = q(buf[j]), cg = q(buf[j + 1]), cb = q(buf[j + 2]), ca = buf[j + 3];
          for (let k = 0; k < len && x < W; k++, x++) { const i = (y * W + x) * 4; out[i] = cr; out[i + 1] = cg; out[i + 2] = cb; out[i + 3] = ca; }
        }
      }
      return out;
    },
  });

  effect('interlaceShift', {
    name: 'Interlace shift', cat: ECAT, desc: 'Even and odd lines pull sideways in opposite directions, with the odd field dimmed.',
    tags: ['interlace', 'comb', 'field', 'tv', 'lines', 'shift', 'broadcast', 'video'],
    params: [P.range('shift', 'Shift (px)', 0, 12, 0.1, 3), P.int('height', 'Line height (px)', 1, 4, 1), P.range('dim', 'Dim odd lines', 0, 0.6, 0.01, 0.2), P.speed(1, 'speed', 'Sway')],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf.length), s = p.shift * (0.6 + 0.4 * Math.sin(TAU * t * p.speed));
      for (let y = 0; y < H; y++) {
        const odd = Math.floor(y / p.height) & 1, sh = odd ? s : -s, m = odd ? 1 - p.dim : 1;
        for (let x = 0; x < W; x++) { const i = (y * W + x) * 4; out[i] = lerpX(buf, W, y, x + sh, 0) * m; out[i + 1] = lerpX(buf, W, y, x + sh, 1) * m; out[i + 2] = lerpX(buf, W, y, x + sh, 2) * m; out[i + 3] = buf[i + 3]; }
      }
      return out;
    },
  });

  effect('signalBands', {
    name: 'Signal noise bands', cat: ECAT, desc: 'Horizontal bands of static roll through the picture, brightening and tearing the lines inside them.',
    tags: ['noise', 'static', 'bands', 'signal', 'interference', 'tv', 'tracking', 'glitch'],
    params: [P.int('bands', 'Bands', 1, 8, 3), P.range('height', 'Band height', 0.02, 0.4, 0.01, 0.12), P.range('noise', 'Noise', 0, 1, 0.01, 0.6), P.speed(1, 'speed', 'Roll'), P.seed(43)],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf), nf = stepOf(t, 12);
      for (let y = 0; y < H; y++) {
        let m = 0;
        for (let b = 0; b < p.bands; b++) {
          const cy = b / p.bands + hash2(b, 0, p.seed) * 0.1 + t * p.speed * (1 + (b % 2)), d = Math.abs(fract((y + 0.5) / H - cy + 0.5) - 0.5);
          if (d < p.height / 2) m = Math.max(m, 1 - d / (p.height / 2));
        }
        if (m <= 0) continue;
        const sh = Math.round((hash2(y, nf, p.seed + 1) - 0.5) * 8 * m);
        for (let x = 0; x < W; x++) {
          const i = (y * W + x) * 4, j = (y * W + wrap(x - sh, W)) * 4, n = hash2(x, y, p.seed + nf * 5), w = m * p.noise;
          for (let k = 0; k < 3; k++) out[i + k] = clamp(mix(buf[j + k], n, w) + 0.1 * m);
        }
      }
      return out;
    },
  });

  effect('jpegBlocks', {
    name: 'JPEG artefacts', cat: ECAT, desc: 'Heavy compression: flat blocks, crushed detail and colour that bleeds across each square.',
    tags: ['jpeg', 'compression', 'blocks', 'artifacts', 'lossy', 'low quality', 'glitch', 'pixelated'],
    params: [P.int('block', 'Block size (px)', 4, 16, 8), P.range('amount', 'Damage', 0, 1, 0.01, 0.75), P.speed(1, 'speed', 'Breathing')],
    run(buf, W, H, t, p) {
      const B = p.block, bw = Math.ceil(W / B), bh = Math.ceil(H / B), mean = new Float32Array(bw * bh * 3), cnt = new Float32Array(bw * bh);
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { const bi = Math.floor(y / B) * bw + Math.floor(x / B), i = (y * W + x) * 4; mean[bi * 3] += buf[i]; mean[bi * 3 + 1] += buf[i + 1]; mean[bi * 3 + 2] += buf[i + 2]; cnt[bi]++; }
      for (let b = 0; b < cnt.length; b++) for (let k = 0; k < 3; k++) mean[b * 3 + k] /= cnt[b];
      const q = p.amount * (0.75 + 0.25 * Math.sin(TAU * t * p.speed)), steps = mix(24, 2.5, q);
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const bi = (Math.floor(y / B) * bw + Math.floor(x / B)) * 3, mr = mean[bi], mg = mean[bi + 1], mb = mean[bi + 2], l = luma(buf[i], buf[i + 1], buf[i + 2]), lm = luma(mr, mg, mb);
        const l2 = lm + Math.round((l - lm) * steps) / steps;
        buf[i] = clamp(l2 + mix(buf[i] - l, mr - lm, q)); buf[i + 1] = clamp(l2 + mix(buf[i + 1] - l, mg - lm, q)); buf[i + 2] = clamp(l2 + mix(buf[i + 2] - l, mb - lm, q));
      });
    },
  });

  effect('colorBleed', {
    name: 'Colour bleed', cat: ECAT, desc: 'Colour smears sideways past the edges of shapes while brightness stays sharp, like composite video.',
    tags: ['bleed', 'chroma', 'smear', 'composite', 'ntsc', 'vhs', 'color', 'analog'],
    params: [P.range('length', 'Smear (px)', 1, 40, 0.5, 10), P.range('gain', 'Saturation', 0.5, 2, 0.01, 1.3), P.range('amount', 'Amount', 0, 1, 0.01, 1), P.speed(1, 'speed', 'Wobble')],
    run(buf, W, H, t, p) {
      const k = Math.exp(-1 / Math.max(0.5, p.length * (1 + 0.3 * Math.sin(TAU * t * p.speed))));
      for (let y = 0; y < H; y++) {
        let ci = 0, cq = 0;
        const row = y * W;
        for (let pass = 0; pass < 2; pass++) for (let x = 0; x < W; x++) {
          const i = (row + x) * 4, r = buf[i], g = buf[i + 1], b = buf[i + 2];
          const I = 0.596 * r - 0.274 * g - 0.322 * b, Q = 0.211 * r - 0.523 * g + 0.312 * b;
          if (x === 0 && pass === 0) { ci = I; cq = Q; }
          ci = ci * k + I * (1 - k); cq = cq * k + Q * (1 - k);
          if (pass === 1) {
            const Y = 0.299 * r + 0.587 * g + 0.114 * b, I2 = mix(I, ci * p.gain, p.amount), Q2 = mix(Q, cq * p.gain, p.amount);
            buf[i] = clamp(Y + 0.956 * I2 + 0.621 * Q2); buf[i + 1] = clamp(Y - 0.272 * I2 - 0.647 * Q2); buf[i + 2] = clamp(Y - 1.106 * I2 + 1.703 * Q2);
          }
        }
      }
      return buf;
    },
  });

  effect('crtTube', {
    name: 'CRT tube', cat: ECAT, desc: 'Curves the picture like a glass tube, rounds and darkens the corners, adds a phosphor mask and a faint flicker.',
    tags: ['crt', 'tube', 'curve', 'vignette', 'tv', 'monitor', 'phosphor', 'retro'],
    params: [P.range('curve', 'Curve', 0, 0.5, 0.01, 0.2), P.range('vignette', 'Vignette', 0, 1, 0.01, 0.5), P.range('corner', 'Corner rounding', 0, 1, 0.01, 0.5), P.range('mask', 'Phosphor mask', 0, 0.6, 0.01, 0.2), P.range('flicker', 'Flicker', 0, 0.2, 0.005, 0.05), P.speed(2, 'speed', 'Flicker rate')],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf.length), o = [0, 0, 0, 0], fl = 1 + p.flicker * Math.sin(TAU * t * p.speed);
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const nx = ((x + 0.5) / W - 0.5) * 2, ny = ((y + 0.5) / H - 0.5) * 2, sx = nx * (1 + p.curve * 0.5 * ny * ny), sy = ny * (1 + p.curve * 0.5 * nx * nx);
        const met = Math.pow(Math.pow(Math.abs(sx), 6) + Math.pow(Math.abs(sy), 6), 1 / 6), edge = 1 - p.corner * 0.2;
        const m = 1 - smoothstep(edge, 1.0, met) * 1, vg = 1 - p.vignette * 0.5 * (sx * sx + sy * sy) * 0.5 - 0;
        bilinear(buf, W, H, (sx / 2 + 0.5) * W, (sy / 2 + 0.5) * H, o, false);
        const i = (y * W + x) * 4, ch = x % 3;
        for (let k = 0; k < 3; k++) out[i + k] = clamp(o[k] * m * vg * fl * (k === ch ? 1 : 1 - p.mask));
        out[i + 3] = o[3];
      }
      return out;
    },
  });

  effect('rollingSync', {
    name: 'Rolling sync bar', cat: ECAT, desc: 'A bright bar rolls up the picture, pushing the lines sideways and cutting a dark gap behind it.',
    tags: ['sync', 'roll', 'vertical hold', 'tv', 'bar', 'interference', 'analog', 'glitch'],
    params: [P.range('height', 'Bar height', 0.05, 0.5, 0.01, 0.2), P.range('push', 'Push (px)', 0, 40, 0.5, 12), P.range('lift', 'Brightness', 0, 1, 0.01, 0.35), P.speed(1, 'speed', 'Rolls')],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf);
      for (let y = 0; y < H; y++) {
        const d = fract((y + 0.5) / H - t * p.speed), q = d / p.height;
        if (q >= 1) continue;
        const dx = p.push * Math.sin(Math.PI * q) * (1 + 0.3 * Math.sin(y * 1.7)), lift = p.lift * Math.pow(1 - q, 1.5), dark = d < 0.02 ? 0.2 : 1;
        for (let x = 0; x < W; x++) { const i = (y * W + x) * 4; for (let k = 0; k < 3; k++) out[i + k] = clamp((lerpX(buf, W, y, x - dx, k) + lift) * dark); }
      }
      return out;
    },
  });

  effect('tapeTear', {
    name: 'Tape tear', cat: ECAT, desc: 'The picture splits at a few tear lines and each strip sways sideways on its own, with a bright seam.',
    tags: ['tear', 'tape', 'slice', 'sway', 'frame tear', 'vhs', 'glitch', 'shift'],
    params: [P.int('tears', 'Tear lines', 1, 8, 4), P.range('amount', 'Sway (px)', 0, 40, 0.5, 14), P.range('fringe', 'Colour fringe (px)', 0, 4, 0.1, 1), P.speed(1, 'speed', 'Sway speed'), P.seed(47)],
    run(buf, W, H, t, p) {
      const n = p.tears, bounds = [];
      for (let i = 0; i < n; i++) bounds.push((i + 0.2 + 0.6 * hash2(i, 0, p.seed)) / n + 0.02 * Math.sin(TAU * (t * p.speed + hash2(i, 1, p.seed))));
      bounds.sort((a, b) => a - b);
      const out = new Float32Array(buf.length);
      for (let y = 0; y < H; y++) {
        const yy = (y + 0.5) / H;
        let seg = 0, near = 1;
        for (let i = 0; i < n; i++) { if (bounds[i] < yy) seg++; near = Math.min(near, Math.abs(bounds[i] - yy) * H); }
        const sw = p.amount * Math.sin(TAU * (t * p.speed + hash2(seg, 2, p.seed))) * (hash2(seg, 3, p.seed) < 0.5 ? 1 : -1), seam = near < 0.6 ? 0.3 : 0;
        for (let x = 0; x < W; x++) { const i = (y * W + x) * 4; out[i] = clamp(lerpX(buf, W, y, x - sw + p.fringe, 0) + seam); out[i + 1] = clamp(lerpX(buf, W, y, x - sw, 1) + seam); out[i + 2] = clamp(lerpX(buf, W, y, x - sw - p.fringe, 2) + seam); out[i + 3] = buf[i + 3]; }
      }
      return out;
    },
  });

  effect('bayer1bit', {
    name: '1-bit Bayer', cat: ECAT, desc: 'Turns the picture into two colours using an ordered Bayer dither, with a brightness wave sweeping across.',
    tags: ['1-bit', 'bayer', 'dither', 'monochrome', 'mac', 'black and white', 'pixel', 'ordered'],
    params: [P.color('ink', 'Dark', '#0b0b12'), P.color('paper', 'Light', '#e9f5e0'), P.int('scale', 'Dot size (px)', 1, 6, 2), P.range('bias', 'Brightness', -0.3, 0.3, 0.01, 0), P.range('sweep', 'Brightness wave', 0, 0.3, 0.01, 0.12), P.select('mode', 'Colours', [['two', 'Two colours'], ['keep', 'Keep colour']], 'two'), P.speed(1, 'speed', 'Wave speed')],
    prepare: colorPrep(['ink', 'paper']),
    run(buf, W, H, t, p) {
      const s = p.scale;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const th = bayer8(Math.floor(x / s), Math.floor(y / s)), off = p.bias + p.sweep * Math.sin(TAU * (t * p.speed + u));
        if (p.mode === 'keep') { for (let k = 0; k < 3; k++) buf[i + k] = buf[i + k] + off > th ? 1 : 0; return; }
        const on = luma(buf[i], buf[i + 1], buf[i + 2]) + off > th, c = on ? p._paper : p._ink;
        buf[i] = c[0]; buf[i + 1] = c[1]; buf[i + 2] = c[2];
      });
    },
  });

  effect('gameBoy', {
    name: 'Game Boy', cat: ECAT, desc: 'Maps the picture to four olive-green shades with light dithering and chunky pixels.',
    tags: ['game boy', 'gameboy', 'green', '4 colour', 'handheld', 'lcd', 'pixel', 'retro', 'palette'],
    params: [P.color('c0', 'Darkest', '#0f380f'), P.color('c1', 'Dark', '#306230'), P.color('c2', 'Light', '#8bac0f'), P.color('c3', 'Lightest', '#9bbc0f'), P.int('size', 'Pixel size (px)', 1, 6, 2), P.range('dither', 'Dither', 0, 1, 0.01, 0.5), P.range('contrast', 'Contrast', 0.5, 2.5, 0.01, 1.3), P.speed(1, 'speed', 'Breathing')],
    prepare: colorPrep(['c0', 'c1', 'c2', 'c3']),
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf.length), s = p.size, pal = [p._c0, p._c1, p._c2, p._c3], ct = p.contrast * (1 + 0.12 * Math.sin(TAU * t * p.speed));
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const sx = Math.min(W - 1, Math.floor(x / s) * s), sy = Math.min(H - 1, Math.floor(y / s) * s), j = (sy * W + sx) * 4, i = (y * W + x) * 4;
        const l = clamp((luma(buf[j], buf[j + 1], buf[j + 2]) - 0.5) * ct + 0.5), idx = clamp(Math.round(l * 3 + (bayer8(Math.floor(x / s), Math.floor(y / s)) - 0.5) * p.dither), 0, 3), c = pal[idx];
        out[i] = c[0]; out[i + 1] = c[1]; out[i + 2] = c[2]; out[i + 3] = buf[j + 3];
      }
      return out;
    },
  });

  effect('tvGhosting', {
    name: 'TV ghosting', cat: ECAT, desc: 'Faint echoes of bright areas trail off to the right, like a badly tuned aerial.',
    tags: ['ghost', 'echo', 'tv', 'aerial', 'antenna', 'double image', 'analog', 'retro'],
    params: [P.range('distance', 'Echo distance (px)', 1, 40, 0.5, 9), P.range('strength', 'Strength', 0, 1, 0.01, 0.6), P.int('echoes', 'Echoes', 1, 4, 2), P.speed(1, 'speed', 'Sway')],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf), d = p.distance * (1 + 0.3 * Math.sin(TAU * t * p.speed));
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        for (let e = 1; e <= p.echoes; e++) {
          const w = Math.pow(p.strength, e);
          for (let k = 0; k < 3; k++) out[i + k] = Math.max(out[i + k], lerpX(buf, W, y, x - d * e, k) * w);
        }
      }
      return out;
    },
  });

}
