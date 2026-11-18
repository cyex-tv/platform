// @ts-nocheck
/* Pack: Geometric patterns: op-art, tilings, weaves, lattices. Same contracts as js/library.js; see js/packs/README.md. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, smoothstep, tri, wrap, hexToRgb, lutAt, hash2, vnoise, fbm, ridged, luma, hsvToRgb } = PL;
  const P = PL.P;
  const { rgb01, colorPrep, eachPx, bilinear, remap, boxBlur } = PL.fx;
  const add = (store, id, def) => { if (store[id]) throw new Error('Duplicate layer or effect id: ' + id); def.id = id; def.pack = 'geometric'; store[id] = def; };
  const layer = (id, def) => add(PL.LAYERS, id, def);
  const effect = (id, def) => add(PL.EFFECTS, id, def);
  for (const c of ['Geometric']) if (!PL.LAYER_CATS.includes(c)) PL.LAYER_CATS.push(c);
  for (const c of []) if (!PL.EFFECT_CATS.includes(c)) PL.EFFECT_CATS.push(c);

  // ---- definitions go here ----

  // ---- shared helpers ----
  const TA = [0, 0, 0, 1], TB = [0, 0, 0, 1];
  const PI = Math.PI, H3 = Math.sqrt(3) / 2;
  const sat = (x) => (x < 0 ? 0 : x > 1 ? 1 : x);
  // Two palette colours mixed by m (0..1).
  const ink = (p, fa, fb, m, o) => {
    lutAt(p._stops, fa, TA); lutAt(p._stops, fb, TB);
    o[0] = TA[0] + (TB[0] - TA[0]) * m; o[1] = TA[1] + (TB[1] - TA[1]) * m; o[2] = TA[2] + (TB[2] - TA[2]) * m; o[3] = 1; return o;
  };
  // Mix a colour already in o with an rgb triple by m.
  const over = (o, c, m) => { o[0] += (c[0] - o[0]) * m; o[1] += (c[1] - o[1]) * m; o[2] += (c[2] - o[2]) * m; o[3] = 1; return o; };
  const shade = (o, k) => { o[0] = sat(o[0] * k); o[1] = sat(o[1] * k); o[2] = sat(o[2] * k); o[3] = 1; return o; };
  // Antialiased line at integer values of F; gpx = change of F per pixel, wpx = line width in pixels.
  const line = (F, gpx, wpx) => { const d = Math.abs(F - Math.round(F)) / (gpx + 1e-6); return sat(wpx * 0.5 + 0.5 - d); };
  // Two-tone square wave of F (period 1); 1 on the middle half.
  const sq = (F, gpx) => { const w = F - Math.floor(F); return sat(0.5 + (0.25 - Math.abs(w - 0.5)) / (gpx + 1e-6)); };
  const bgPrep = colorPrep(['bg']);

  layer('target', {
    name: 'Target rings', cat: 'Geometric', desc: 'Crisp concentric rings that pulse outward from the middle, with the colour drifting along the radius.',
    tags: ['target', 'rings', 'concentric', 'circles', 'bullseye', 'op art', 'pulse', 'ripple'],
    params: [P.pal('neon', true), P.range('count', 'Rings', 2, 24, 0.5, 8), P.range('shift', 'Colour drift', 0, 1, 0.01, 0.3), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const d = Math.hypot((u - 0.5) * c.A, v - 0.5) * p.count;
      const s = Math.sin(TAU * (d - t * p.speed)), w = Math.min(1, TAU * p.count / c.H * 0.7) + 0.02;
      return ink(p, fract(d * p.shift * 0.25), fract(0.5 + d * p.shift * 0.25), smoothstep(-w, w, s), o);
    },
  });

  layer('moire', {
    name: 'Moiré circles', cat: 'Geometric', desc: 'Two sets of fine rings drifting around each other and throwing shimmering interference fringes.',
    tags: ['moire', 'interference', 'circles', 'rings', 'op art', 'fringes', 'optical', 'illusion'],
    params: [P.pal('icyLake'), P.range('count', 'Density', 4, 30, 0.5, 7), P.range('orbit', 'Orbit', 0, 0.4, 0.01, 0.16), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed, cs = Math.cos(a), sn = Math.sin(a);
      const d1 = Math.hypot((u - 0.5 - p.orbit * cs) * c.A, v - 0.5 - p.orbit * 0.5 * sn);
      const d2 = Math.hypot((u - 0.5 + p.orbit * cs) * c.A, v - 0.5 + p.orbit * 0.5 * sn);
      const s = Math.sin(TAU * d1 * p.count) + Math.sin(TAU * d2 * p.count);
      return lutAt(p._stops, sat(0.5 + 0.28 * s), o);
    },
  });

  layer('warpedChecker', {
    name: 'Warped checker', cat: 'Geometric', desc: 'An op-art chessboard bent by travelling waves so the squares seem to bulge and breathe.',
    tags: ['checker', 'chessboard', 'op art', 'warp', 'bulge', 'illusion', 'riley', 'wave'],
    params: [P.pal('mono'), P.int('cols', 'Squares across', 4, 48, 20), P.range('amp', 'Warp', 0, 1.2, 0.01, 0.45), P.int('waves', 'Waves', 1, 6, 2), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const x0 = u * p.cols, y0 = v * p.cols / c.A, ph = TAU * t * p.speed;
      const x = x0 + p.amp * Math.sin(TAU * (y0 * 0.25) + ph) * Math.cos(TAU * u * p.waves);
      const y = y0 + p.amp * Math.sin(TAU * u * p.waves - ph);
      const w = 1.4 * PI * p.cols / c.W * (1 + p.amp) + 0.02;
      return ink(p, 0.06 + 0.1 * u, 0.88, smoothstep(-w, w, Math.sin(PI * x) * Math.sin(PI * y)), o);
    },
  });

  layer('herringbone', {
    name: 'Herringbone', cat: 'Geometric', desc: 'Diagonal planks laid in a zig-zag parquet, sliding along their length with the joints picked out.',
    tags: ['herringbone', 'parquet', 'planks', 'wood floor', 'zigzag', 'tiles', 'brick', 'chevron'],
    params: [P.pal('gold'), P.int('cols', 'Columns', 8, 64, 24), P.range('gap', 'Joint', 0, 0.25, 0.01, 0.07), P.color('mortar', 'Joint colour', '#14110e'), P.speed(1), P.seed(7)],
    prepare: colorPrep(['mortar']),
    sample(u, v, t, p, o, c) {
      const n = p.cols & ~1, g = n / c.W;
      const x = u * n + 4 * t * p.speed, y = v * n / c.A;
      const q = y + tri(x * 0.5);
      const b = Math.floor(q), fq = q - b, jx = fract((x + b) * 0.5);
      const par = Math.floor((x + b) * 0.5) & 1;
      lutAt(p._stops, 0.1 + 0.8 * hash2(wrap(b, 7), 3, p.seed), o);
      shade(o, par ? 1 : 0.8);
      const dq = Math.min(fq, 1 - fq), dj = Math.min(jx, 1 - jx) * 2;
      const e = sat((Math.min(dq, dj) - p.gap * 0.5) / (1.6 * g) + 0.5);
      return over(o, p._mortar, 1 - e);
    },
  });

  layer('houndstooth', {
    name: 'Houndstooth', cat: 'Geometric', desc: 'A toothy two-tone check with jagged diagonal edges that slowly marches across.',
    tags: ['houndstooth', 'check', 'dogtooth', 'tweed', 'fabric', 'teeth', 'pattern', 'classic'],
    params: [P.pal('mono'), P.int('cols', 'Columns', 8, 96, 32), P.range('teeth', 'Teeth', 0, 1.5, 0.01, 0.8), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const x = u * p.cols, y = v * p.cols / c.A, g = p.cols / c.W;
      const X = x + 4 * t * p.speed + p.teeth * (tri(y * 0.5) * 2 - 1) * 0.5;
      const Y = y + p.teeth * (tri(x * 0.5) * 2 - 1) * 0.5;
      const w = (PI / 2) * g * (1 + p.teeth) + 0.02;
      return ink(p, 0.05, 0.92, smoothstep(-w, w, Math.sin(PI * X / 2) * Math.sin(PI * Y / 2)), o);
    },
  });

  layer('argyle', {
    name: 'Argyle', cat: 'Geometric', desc: 'Four-colour diamonds with fine stitched diagonals that slide across the knit.',
    tags: ['argyle', 'diamonds', 'knit', 'sock', 'lattice', 'preppy', 'rhombus', 'stitch'],
    params: [P.pal('vapor'), P.int('cols', 'Columns', 4, 48, 16), P.range('squash', 'Height', 0.5, 2, 0.01, 1), P.color('stitch', 'Stitch colour', '#f4efe2'), P.speed(1)],
    prepare: colorPrep(['stitch']),
    sample(u, v, t, p, o, c) {
      const n = p.cols & ~1, g = n / c.W;
      const x = u * n, y = v * n / c.A * p.squash;
      const a = (x + y) / 2 - 2 * t * p.speed, b = (x - y) / 2;
      const ia = Math.floor(a), ib = Math.floor(b), fa = a - ia, fb = b - ib;
      lutAt(p._stops, ((ia & 1) * 2 + (ib & 1) + 0.5) / 4, o);
      const pa = 1.414 * g * 0.5;
      const edge = Math.min(Math.min(fa, 1 - fa), Math.min(fb, 1 - fb));
      shade(o, 0.55 + 0.45 * smoothstep(0, pa * 3.2, edge));
      const ln = Math.max(1 - Math.abs(fa - 0.5) / (pa + 1e-6) + 0.2, 1 - Math.abs(fb - 0.5) / (pa + 1e-6) + 0.2);
      return over(o, p._stitch, sat(ln) * 0.9);
    },
  });

  layer('basketWeave', {
    name: 'Basket weave', cat: 'Geometric', desc: 'Woven blocks of three rounded strips, alternating direction, with a soft light sweeping over them.',
    tags: ['basket', 'weave', 'woven', 'wicker', 'strips', 'checker', 'rattan', 'texture'],
    params: [P.pal('sunset'), P.int('cols', 'Blocks across', 4, 40, 16), P.range('spread', 'Colour spread', 0, 1, 0.01, 0.6), P.speed(1), P.seed(21)],
    sample(u, v, t, p, o, c) {
      const x = u * p.cols, y = v * p.cols / c.A, g = p.cols / c.W;
      const bx = Math.floor(x), by = Math.floor(y), fx = x - bx, fy = y - by;
      const ori = (bx + by) & 1, pc = (ori ? fx : fy) * 3, pa = ori ? fy : fx;
      const si = Math.floor(pc), fs = pc - si;
      const h = hash2(wrap(bx, p.cols) * 3 + si, by * 2 + ori, p.seed);
      lutAt(p._stops, sat(0.5 + (h - 0.5) * p.spread + (ori ? 0.12 : -0.12)), o);
      const gl = 0.78 + 0.22 * Math.cos(TAU * (u * 2 + v * 0.5 - t * p.speed));
      const round = 0.3 + 0.7 * Math.pow(Math.sin(PI * fs), 0.6);
      const end = smoothstep(0.015, 0.015 + 2 * g, Math.min(pa, 1 - pa));
      const side = smoothstep(0, 0.15 + 3 * g, Math.min(fs, 1 - fs));
      return shade(o, gl * round * (0.35 + 0.65 * end * (0.5 + 0.5 * side)));
    },
  });

  layer('chevronGlow', {
    name: 'Glowing chevrons', cat: 'Geometric', desc: 'Stacked V-shaped bands of colour that climb the paint, each with a bright glowing edge.',
    tags: ['chevron', 'arrows', 'v shape', 'glow', 'stripes', 'bands', 'sergeant', 'zigzag'],
    params: [P.pal('sunset', true), P.range('bands', 'Bands', 1, 12, 0.5, 3), P.int('peaks', 'Peaks', 1, 16, 4), P.range('glow', 'Glow', 0, 1.5, 0.01, 0.7), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const F = v * p.bands + (tri(u * p.peaks) - 0.5) * 0.9 * Math.max(1, p.bands / 3) - t * p.speed;
      const f = fract(F), dd = Math.min(f, 1 - f);
      lutAt(p._stops, f, o);
      const gl = Math.exp(-dd * dd / 0.006) * p.glow;
      const e = smoothstep(0, 0.02 + p.bands / c.H, dd);
      shade(o, 0.55 + 0.45 * e);
      o[0] = sat(o[0] + gl); o[1] = sat(o[1] + gl); o[2] = sat(o[2] + gl);
      return o;
    },
  });

  // Triangle lattice (rows of height H3, side 1): which rhombus orientation a triangle belongs to.
  const faceUp = (i, j) => wrap(i + 2 * j, 3);
  const faceDown = (i, j) => { const s = wrap(i + 2 * j, 3); return s === 0 ? 0 : s === 2 ? 1 : 2; };

  layer('isoCubes', {
    name: 'Isometric cubes', cat: 'Geometric', desc: 'Tumbling blocks: a field of three-sided cubes with light, mid and dark faces and a ripple of light across them.',
    tags: ['cubes', 'isometric', 'blocks', 'tumbling', '3d', 'rhombus', 'boxes', 'illusion'],
    params: [P.pal('ice'), P.int('cols', 'Cubes across', 3, 48, 18), P.range('ripple', 'Ripple', 0, 0.5, 0.01, 0.2), P.range('edge', 'Edge', 0, 1, 0.01, 0.5), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const n = Math.max(3, Math.round(p.cols / 3) * 3), g = n / c.W;
      const y = v * n / c.A, bb = y / H3, aa = u * n - 0.5 * bb;
      const ia = Math.floor(aa), ib = Math.floor(bb), fa = aa - ia, fb = bb - ib;
      const up = fa + fb < 1;
      let f, ed;
      if (up) {
        f = faceUp(ia, ib);
        ed = Math.min(faceDown(ia, ib) !== f ? 1 - fa - fb : 9, faceDown(ia, ib - 1) !== f ? fb : 9, faceDown(ia - 1, ib) !== f ? fa : 9);
      } else {
        f = faceDown(ia, ib);
        ed = Math.min(faceUp(ia, ib) !== f ? fa + fb - 1 : 9, faceUp(ia, ib + 1) !== f ? 1 - fb : 9, faceUp(ia + 1, ib) !== f ? 1 - fa : 9);
      }
      lutAt(p._stops, f === 0 ? 0.85 : f === 1 ? 0.5 : 0.15, o);
      shade(o, 1 + p.ripple * Math.sin(TAU * (u * 3 + v - t * p.speed)));
      return shade(o, 1 - p.edge * (1 - smoothstep(0, 1.3 * g / H3, ed)));
    },
  });

  layer('triShimmer', {
    name: 'Triangle shimmer', cat: 'Geometric', desc: 'A tiling of small triangles in palette colours, each one shimmering on its own beat.',
    tags: ['triangles', 'tessellation', 'low poly', 'shimmer', 'facets', 'crystal', 'tiles', 'mosaic'],
    params: [P.pal('aurora'), P.int('cols', 'Triangles across', 4, 64, 24), P.range('shimmer', 'Shimmer', 0, 1, 0.01, 0.45), P.range('edge', 'Edge', 0, 1, 0.01, 0.35), P.speed(1), P.seed(5)],
    sample(u, v, t, p, o, c) {
      const n = p.cols, g = n / c.W;
      const y = v * n / c.A, bb = y / H3, aa = u * n - 0.5 * bb;
      const ia = Math.floor(aa), ib = Math.floor(bb), fa = aa - ia, fb = bb - ib;
      const up = fa + fb < 1;
      const h = hash2(wrap(ia + Math.floor(0.5 * bb), n), ib * 2 + (up ? 0 : 1), p.seed);
      lutAt(p._stops, h, o);
      shade(o, 1 - p.shimmer * 0.5 + p.shimmer * 0.5 * Math.sin(TAU * (h * 3 + t * p.speed)));
      const ed = up ? Math.min(fa, fb, 1 - fa - fb) : Math.min(1 - fa, 1 - fb, fa + fb - 1);
      return shade(o, 1 - p.edge * (1 - smoothstep(0, 1.3 * g / H3, ed)));
    },
  });

  layer('diamondLattice', {
    name: 'Diamond lattice', cat: 'Geometric', desc: 'A trellis of crossing diagonal lines with glowing nodes and light pulses running along each strand.',
    tags: ['lattice', 'trellis', 'diamond', 'grid', 'network', 'crosshatch', 'wire', 'pulse'],
    params: [P.pal('neon'), P.int('cols', 'Columns', 4, 48, 16), P.range('width', 'Line width', 0.4, 4, 0.1, 1.6), P.color('bg', 'Background', '#0b0d14'), P.speed(1)],
    prepare: bgPrep,
    sample(u, v, t, p, o, c) {
      const n = p.cols & ~1, g = n / c.W, x = u * n, y = v * n / c.A;
      const a = (x + y) / 2, b = (x - y) / 2, k = 1.414 * g * 0.5;
      const da = Math.abs(a - Math.round(a)), db = Math.abs(b - Math.round(b));
      const ph = TAU * t * p.speed;
      const pa = 0.5 + 0.5 * Math.cos(TAU * (u * 2 - y * 0.125) - ph), pb = 0.5 + 0.5 * Math.cos(TAU * (u * 2 + y * 0.125) + ph);
      const wa = sat(p.width * 0.5 + 0.5 - da / k), wb = sat(p.width * 0.5 + 0.5 - db / k);
      const node = Math.exp(-(da * da + db * db) / (0.02 + 2 * k * k));
      lutAt(p._stops, sat(0.15 + 0.8 * Math.max(wa * pa, wb * pb)), TA);
      o[0] = p._bg[0]; o[1] = p._bg[1]; o[2] = p._bg[2]; o[3] = 1;
      over(o, TA, Math.max(wa * (0.25 + 0.75 * pa), wb * (0.25 + 0.75 * pb)));
      lutAt(p._stops, 0.95, TB);
      return over(o, TB, sat(node * 0.8 * Math.max(wa, wb)));
    },
  });

  layer('quasicrystal', {
    name: 'Quasicrystal', cat: 'Geometric', desc: 'A sum of rotated waves that makes a never-quite-repeating, kaleidoscopic crystal that slowly shifts.',
    tags: ['quasicrystal', 'waves', 'interference', 'kaleidoscope', 'crystal', 'psychedelic', 'symmetry', 'rainbow'],
    params: [P.pal('rainbowLoop', true), P.int('waves', 'Waves', 3, 11, 7), P.range('scale', 'Scale', 2, 14, 0.5, 6), P.range('bands', 'Bands', 0.3, 4, 0.05, 1.4), P.speed(1)],
    prepare(p) {
      const N = p.waves; p._kx = []; p._ky = [];
      for (let i = 0; i < N; i++) { const th = (i * PI) / N; p._kx.push(Math.round(p.scale * 4 * Math.cos(th))); p._ky.push(p.scale * Math.sin(th)); }
    },
    sample(u, v, t, p, o) {
      const ph = TAU * t * p.speed, N = p.waves; let s = 0;
      for (let i = 0; i < N; i++) s += Math.cos(TAU * (p._kx[i] * u + p._ky[i] * v) - ph);
      return lutAt(p._stops, fract(0.5 + (s / N) * p.bands), o);
    },
  });

  layer('guilloche', {
    name: 'Guilloché', cat: 'Geometric', desc: 'Fine interlaced wave lines like a banknote border, weaving over and under as they slide.',
    tags: ['guilloche', 'banknote', 'currency', 'engraving', 'wave lines', 'weave', 'fine lines', 'security'],
    params: [P.pal('gold', true), P.range('lines', 'Lines', 4, 24, 0.5, 12), P.range('amp', 'Swing', 0.2, 6, 0.1, 2.4), P.int('waves', 'Waves', 1, 12, 4), P.range('width', 'Line width', 0.4, 3, 0.1, 1), P.color('bg', 'Background', '#0e0c08'), P.speed(1)],
    prepare: bgPrep,
    sample(u, v, t, p, o, c) {
      const ph = TAU * t * p.speed, k = p.waves;
      const w1 = Math.sin(TAU * u * k - ph) + 0.5 * Math.sin(TAU * u * (k * 2 + 1) + ph);
      const dw = p.amp * TAU * k * 2 / c.W;
      const g = Math.hypot(p.lines / c.H, dw);
      const F1 = v * p.lines + p.amp * 0.5 * w1, F2 = v * p.lines - p.amp * 0.5 * w1;
      const m1 = line(F1, g, p.width), m2 = line(F2 + 0.5, g, p.width);
      o[0] = p._bg[0]; o[1] = p._bg[1]; o[2] = p._bg[2]; o[3] = 1;
      lutAt(p._stops, fract(u + v * 0.5), TA);
      over(o, TA, m1);
      lutAt(p._stops, fract(u + v * 0.5 + 0.5), TA);
      return over(o, TA, m2 * (0.5 + 0.5 * Math.sin(TAU * (u * k * 0.5 + F1 * 0.5))));
    },
  });

  layer('lissajous', {
    name: 'Lissajous lines', cat: 'Geometric', desc: 'Looping contour lines of crossing waves: ovals and figure-eights that swell and merge.',
    tags: ['lissajous', 'contour', 'loops', 'oscilloscope', 'figure eight', 'waves', 'lines', 'curves'],
    params: [P.pal('neon', true), P.int('fx', 'Waves across', 1, 16, 6), P.int('fy', 'Waves down', 1, 6, 2), P.range('lines', 'Contours', 1, 10, 0.5, 4), P.range('width', 'Line width', 0.4, 3, 0.1, 1.2), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const ph = TAU * t * p.speed;
      const X = TAU * p.fx * u + ph, Y = TAU * p.fy * v - ph * 2;
      const F = (Math.cos(X) + Math.cos(Y) + 0.8 * Math.cos(X + Y + ph)) * p.lines * 0.5;
      const gx = (Math.sin(X) + 0.8 * Math.sin(X + Y + ph)) * TAU * p.fx / c.W, gy = (Math.sin(Y) + 0.8 * Math.sin(X + Y + ph)) * TAU * p.fy / c.H;
      const g = Math.hypot(gx, gy) * p.lines * 0.5;
      const m = line(F, g, p.width), idx = Math.round(F);
      lutAt(p._stops, fract(idx * 0.17 + 0.1), o);
      o[0] *= m; o[1] *= m; o[2] *= m; o[3] = 1;
      return o;
    },
  });

  layer('spirograph', {
    name: 'Spirograph rosette', cat: 'Geometric', desc: 'Rings pinched into petals like a spirograph drawing, with the petals turning slowly.',
    tags: ['spirograph', 'rosette', 'petals', 'flower', 'mandala', 'rings', 'hypotrochoid', 'guilloche'],
    params: [P.pal('rainbowLoop', true), P.int('petals', 'Petals', 2, 16, 7), P.range('rings', 'Rings', 2, 16, 0.5, 7), P.range('depth', 'Depth', 0, 0.6, 0.01, 0.28), P.range('width', 'Line width', 0.4, 3, 0.1, 1.1), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const dx = (u - 0.5) * c.A, dy = v - 0.5, r = Math.hypot(dx, dy) + 1e-5, th = Math.atan2(dy, dx);
      const q = p.petals * th - TAU * t * p.speed, co = Math.cos(q), si = Math.sin(q);
      const F = r * p.rings * (1 + p.depth * co);
      const g = p.rings * Math.hypot(1 + p.depth * co, p.depth * p.petals * si) / c.H;
      const m = line(F, g, p.width);
      lutAt(p._stops, fract(th / TAU + r * 0.6), o);
      o[0] = 0.04 + o[0] * m; o[1] = 0.04 + o[1] * m; o[2] = 0.05 + o[2] * m; o[3] = 1;
      return o;
    },
  });

  layer('sunburst', {
    name: 'Sunburst', cat: 'Geometric', desc: 'Alternating wedge rays fanning out from the centre, turning slowly like a retro poster sun.',
    tags: ['sunburst', 'rays', 'radial', 'wedges', 'sun', 'retro', 'poster', 'fan'],
    params: [P.pal('gold'), P.int('rays', 'Rays', 4, 80, 24), P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', -0.5, 1.5, 0.01, 0.5), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const dx = (u - p.cx) * c.A, dy = v - p.cy, r = Math.hypot(dx, dy) + 1e-4;
      const s = Math.sin(p.rays * Math.atan2(dy, dx) - TAU * t * p.speed);
      const w = Math.min(1.5, p.rays / (r * c.H) * 0.8) + 0.02;
      const m = smoothstep(-w, w, s);
      return ink(p, sat(0.12 + 0.25 * r), sat(0.92 - 0.2 * r), m, o);
    },
  });

  layer('polkaWave', {
    name: 'Polka wave', cat: 'Geometric', desc: 'Staggered polka dots that grow and shrink in a travelling wave, shifting colour as they swell.',
    tags: ['polka', 'dots', 'circles', 'spots', 'wave', 'halftone', 'bubbles', 'playful'],
    params: [P.pal('candy', true), P.int('cols', 'Dots across', 4, 64, 24), P.range('size', 'Size', 0.2, 1.2, 0.01, 0.9), P.int('waves', 'Waves', 1, 6, 2), P.color('bg', 'Background', '#15131c'), P.speed(1)],
    prepare: bgPrep,
    sample(u, v, t, p, o, c) {
      const n = p.cols, g = n / c.W, x = u * n, y = v * n / c.A;
      const j = Math.floor(y), xo = x + ((j & 1) ? 0.5 : 0), i = Math.floor(xo);
      const dx = xo - i - 0.5, dy = y - j - 0.5;
      const ph = (i + 0.5 - ((j & 1) ? 0.5 : 0)) / n * p.waves + j * 0.07 - t * p.speed;
      const wv = 0.5 + 0.5 * Math.sin(TAU * ph);
      const rad = 0.47 * p.size * (0.2 + 0.8 * wv);
      const m = smoothstep(rad + g * 0.8, rad - g * 0.8, Math.hypot(dx, dy));
      o[0] = p._bg[0]; o[1] = p._bg[1]; o[2] = p._bg[2]; o[3] = 1;
      lutAt(p._stops, fract(ph), TA);
      return over(o, TA, m);
    },
  });

  layer('staircase', {
    name: 'Pixel staircase', cat: 'Geometric', desc: 'Diagonal colour bands built from chunky square steps, climbing like an 8-bit staircase.',
    tags: ['staircase', 'pixel', 'steps', 'stairs', '8-bit', 'blocky', 'diagonal', 'retro'],
    params: [P.pal('vapor', true), P.int('cols', 'Steps across', 8, 96, 40), P.int('bands', 'Bands', 1, 8, 3), P.range('gap', 'Gap', 0, 0.3, 0.01, 0.06), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const x = u * p.cols, y = v * p.cols / c.A, g = p.cols / c.W;
      const i = Math.floor(x), j = Math.floor(y), fx = x - i, fy = y - j;
      lutAt(p._stops, fract((i + j) / p.cols * p.bands - t * p.speed), o);
      const e = smoothstep(p.gap * 0.5, p.gap * 0.5 + g * 1.2, Math.min(fx, 1 - fx, fy, 1 - fy));
      return shade(o, 0.25 + 0.75 * e);
    },
  });

  layer('squareTunnel', {
    name: 'Square tunnel', cat: 'Geometric', desc: 'Nested squares rushing toward you as if flying down a neon corridor.',
    tags: ['tunnel', 'squares', 'corridor', 'perspective', 'warp', 'depth', 'concentric', 'zoom'],
    params: [P.pal('neon', true), P.range('rings', 'Density', 0.5, 6, 0.1, 2.2), P.range('dark', 'Centre fade', 0, 1, 0.01, 0.6), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const d = Math.max(Math.abs(u - 0.5) * c.A * 0.7, Math.abs(v - 0.5)) + 0.015;
      const F = Math.log(d) * p.rings - t * p.speed, f = fract(F), dd = Math.min(f, 1 - f);
      lutAt(p._stops, f, o);
      const e = smoothstep(0, 1.6 * p.rings / (d * c.H) + 0.01, dd);
      return shade(o, (0.3 + 0.7 * e) * (1 - p.dark * (1 - smoothstep(0.02, 0.3, d))));
    },
  });

  layer('wavyLines', {
    name: 'Wavy stripes', cat: 'Geometric', desc: 'Bridget Riley style stripes rippling in and out of phase so the surface seems to undulate.',
    tags: ['riley', 'op art', 'wavy', 'stripes', 'lines', 'undulate', 'black and white', 'current'],
    params: [P.pal('mono'), P.range('lines', 'Stripes', 2, 20, 0.5, 8), P.range('amp', 'Swing', 0, 2, 0.01, 0.9), P.int('waves', 'Waves', 1, 12, 4), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const ph = TAU * t * p.speed, env = 0.5 + 0.5 * Math.cos(TAU * u);
      const F = v * p.lines + p.amp * (0.4 + 0.6 * env) * Math.sin(TAU * u * p.waves - ph) + 0.5 * p.amp * Math.sin(TAU * u * (p.waves + 2) + ph);
      const g = Math.hypot(p.lines / c.H, p.amp * TAU * (p.waves + 2) * 1.2 / c.W);
      return ink(p, 0.04, 0.95, sq(F, g), o);
    },
  });

  layer('halftone', {
    name: 'Halftone dots', cat: 'Geometric', desc: 'A diagonal grid of dots whose size swells and fades in a wave, like a printed halftone screen.',
    tags: ['halftone', 'dots', 'print', 'comic', 'dot matrix', 'gradient', 'screen', 'pop art'],
    params: [P.pal('sunset'), P.int('cols', 'Dots across', 8, 96, 40), P.range('size', 'Size', 0.3, 1.2, 0.01, 0.9), P.color('bg', 'Paper', '#fff5e6'), P.speed(1)],
    prepare: bgPrep,
    sample(u, v, t, p, o, c) {
      const n = p.cols & ~1, g = n / c.W, x = u * n, y = v * n / c.A;
      const a = (x + y) / 2, b = (x - y) / 2, ia = Math.floor(a), ib = Math.floor(b);
      const xc = ia + ib + 1, yc = ia - ib, uc = xc / n, vc = yc * c.A / n;
      const val = 0.5 + 0.5 * Math.sin(TAU * (uc * 2 - t * p.speed) + vc * PI * 0.8);
      const rad = 0.62 * p.size * Math.sqrt(val);
      const m = smoothstep(rad + g * 0.6, rad - g * 0.6, Math.hypot(a - ia - 0.5, b - ib - 0.5));
      o[0] = p._bg[0]; o[1] = p._bg[1]; o[2] = p._bg[2]; o[3] = 1;
      lutAt(p._stops, sat(0.15 + 0.8 * (uc * 0.5 + val * 0.5)), TA);
      return over(o, TA, m);
    },
  });

  layer('crossStitch', {
    name: 'Cross-stitch', cat: 'Geometric', desc: 'A grid of little stitched crosses on cloth, their colours rolling through in a slow diagonal wave.',
    tags: ['cross stitch', 'embroidery', 'needlework', 'crosses', 'x', 'thread', 'sampler', 'craft'],
    params: [P.pal('neonPop', true), P.int('cols', 'Stitches across', 6, 80, 32), P.range('thick', 'Thread', 0.08, 0.3, 0.01, 0.17), P.color('bg', 'Cloth', '#15171d'), P.speed(1)],
    prepare: bgPrep,
    sample(u, v, t, p, o, c) {
      const n = p.cols, g = n / c.W, x = u * n, y = v * n / c.A;
      const i = Math.floor(x), j = Math.floor(y), dx = x - i - 0.5, dy = y - j - 0.5;
      const d = Math.min(Math.abs(dx - dy), Math.abs(dx + dy)) * 0.7071;
      const m = smoothstep(p.thick + g * 0.8, p.thick - g * 0.8, d);
      const ph = (i + 0.5) / n * 2 + j * 0.06 - t * p.speed;
      lutAt(p._stops, fract(ph), TA);
      shade(TA, 0.65 + 0.35 * Math.cos(PI * Math.min(1, d / (p.thick * 1.2))));
      o[0] = p._bg[0]; o[1] = p._bg[1]; o[2] = p._bg[2]; o[3] = 1;
      return over(o, TA, m);
    },
  });

  layer('brickGlow', {
    name: 'Glowing bricks', cat: 'Geometric', desc: 'A running-bond brick wall in mixed palette colours with a wash of light travelling across it.',
    tags: ['bricks', 'wall', 'masonry', 'running bond', 'highlight', 'tiles', 'blocks', 'sweep'],
    params: [P.pal('ember'), P.int('cols', 'Bricks across', 3, 40, 12), P.range('mortar', 'Mortar', 0, 0.3, 0.01, 0.08), P.range('light', 'Light', 0, 1.5, 0.01, 0.8), P.color('mortarColor', 'Mortar colour', '#18120f'), P.speed(1), P.seed(13)],
    prepare: colorPrep(['mortarColor']),
    sample(u, v, t, p, o, c) {
      const n = p.cols, g = n / c.W, x = u * n, y = v * n / c.A * 2;
      const j = Math.floor(y), xo = x + ((j & 1) ? 0.5 : 0), i = Math.floor(xo);
      lutAt(p._stops, 0.1 + 0.8 * hash2(wrap(i, n), j, p.seed), o);
      const ph = fract(u - v * 0.3 - t * p.speed), dd = Math.min(ph, 1 - ph);
      shade(o, 0.75 * (1 + p.light * Math.exp(-dd * dd / 0.012)));
      const ex = Math.min(xo - i, 1 - (xo - i)), ey = Math.min(y - j, 1 - (y - j)) * 0.5;
      const e = sat((Math.min(ex, ey) - p.mortar * 0.5) / g + 0.5);
      return over(o, p._mortarColor, 1 - e);
    },
  });

  layer('zigzagLines', {
    name: 'Rainbow zigzag lines', cat: 'Geometric', desc: 'Thin zig-zag lines stacked down the paint and sliding upward, coloured by a gradient across the surface.',
    tags: ['zigzag', 'zig zag', 'lines', 'rainbow', 'wiggle', 'stripes', 'waves', 'thin'],
    params: [P.pal('rainbowLoop', true), P.range('lines', 'Lines', 3, 20, 0.5, 8), P.int('teeth', 'Teeth', 2, 40, 16), P.range('tilt', 'Height', 0.1, 2, 0.01, 0.7), P.range('width', 'Line width', 0.4, 4, 0.1, 1.5), P.color('bg', 'Background', '#0d0d14'), P.speed(1)],
    prepare: bgPrep,
    sample(u, v, t, p, o, c) {
      const F = v * p.lines + (tri(u * p.teeth) - 0.5) * p.tilt * 2 - t * p.speed;
      const g = Math.hypot(p.lines / c.H, p.tilt * 4 * p.teeth / c.W);
      const m = line(F, g, p.width);
      o[0] = p._bg[0]; o[1] = p._bg[1]; o[2] = p._bg[2]; o[3] = 1;
      lutAt(p._stops, fract(u + v * 1.5), TA);
      return over(o, TA, m);
    },
  });

  layer('starTiling', {
    name: 'Star tiling', cat: 'Geometric', desc: 'An Islamic-style tiling of eight-point stars with smaller four-point stars between, turning in step.',
    tags: ['islamic', 'star', 'tiling', 'tessellation', 'girih', 'mosaic', 'tile', 'ornament'],
    params: [P.pal('gold'), P.int('cols', 'Cells across', 2, 32, 12), P.range('line', 'Outline', 0, 3, 0.1, 1.2), P.speed(1)],
    sample(u, v, t, p, o, c) {
      const n = p.cols & ~1, g = n / c.W, x = u * n, y = v * n / c.A;
      const ph = TAU * t * p.speed;
      const ci = Math.floor(x), cj = Math.floor(y);
      const dx = x - ci - 0.5, dy = y - cj - 0.5;
      const r1 = Math.hypot(dx, dy), t1 = Math.atan2(dy, dx);
      const R1 = 0.36 * (1 + 0.3 * Math.cos(8 * t1 - ph));
      const ex = x + 0.5, ey = y + 0.5;
      const ex2 = ex - Math.floor(ex) - 0.5, ey2 = ey - Math.floor(ey) - 0.5;
      const r2 = Math.hypot(ex2, ey2), t2 = Math.atan2(ey2, ex2);
      const R2 = 0.2 * (1 + 0.45 * Math.cos(4 * t2 + ph));
      const par = (ci + cj) & 1;
      lutAt(p._stops, 0.06 + 0.04 * par, o);
      const d1 = (r1 - R1) / 1.3, d2 = (r2 - R2) / 1.3;
      lutAt(p._stops, par ? 0.4 : 0.62, TA); over(o, TA, smoothstep(g, -g, d1));
      lutAt(p._stops, 0.85, TA); over(o, TA, smoothstep(g, -g, d2));
      lutAt(p._stops, 0.98, TB);
      const lw = p.line * g;
      const l1 = sat(1 - (Math.abs(d1) - lw * 0.5) / g), l2 = sat(1 - (Math.abs(d2) - lw * 0.5) / g);
      return over(o, TB, Math.max(l1, l2) * (p.line > 0 ? 0.9 : 0));
    },
  });

}
