// @ts-nocheck
/* More layers and effects. Same contracts as library.js. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, smoothstep, tri, wrap, hexToRgb, lutAt, hash2, vnoise, fbm, luma } = PL;
  const P = PL.P;
  const layer = (id, def) => { def.id = id; PL.LAYERS[id] = def; };
  const effect = (id, def) => { def.id = id; PL.EFFECTS[id] = def; };
  const { rgb01, colorPrep, eachPx, bilinear, remap } = PL.fx;
  // Per-frame churn offset, cached on p (t is constant for a whole frame): read p._fx, p._fy afterwards.
  const flow = (t, p) => { if (p._ft !== t) { const a = TAU * t * p.speed; p._ft = t; p._fx = Math.cos(a) * p.flow; p._fy = Math.sin(a) * p.flow; } };

  // =====================================================================================
  // Layers
  // =====================================================================================
  layer('mesh', {
    name: 'Mesh gradient', cat: 'Gradients', desc: 'Soft colour blobs that drift and blend. Each colour\'s strength sets how much space it takes.',
    params: [P.pal('candy'), P.range('soft', 'Softness', 0.5, 6, 0.1, 2.2), P.range('move', 'Movement', 0, 0.6, 0.01, 0.25), P.speed(1), P.seed(17)],
    prepare(p) {
      const r = PL.rng(p.seed * 131), cols = p.stops.colors.filter((c) => c.w > 0);
      p._blobs = cols.map((c) => ({ rgb: rgb01(c.c), a: c.a ?? 1, w: c.w, x: r(), y: r(), px: r(), py: r(), k: 1 + ((r() * 2) | 0) }));
    },
    sample(u, v, t, p, o, c) {
      if (!p._blobs.length) { o[0] = o[1] = o[2] = o[3] = 0; return o; } // all strengths 0: nothing to draw
      let sr = 0, sg = 0, sb = 0, sa = 0, sw = 0;
      for (const b of p._blobs) {
        const bx = b.x + p.move * Math.sin(TAU * (t * p.speed * b.k + b.px)), by = b.y + p.move * Math.cos(TAU * (t * p.speed * b.k + b.py));
        let dx = Math.abs(u - fract(bx)); dx = Math.min(dx, 1 - dx); dx *= c.A;
        const dy = v - by, w = b.w / Math.pow(dx * dx + dy * dy + 0.002, p.soft * 0.5);
        sr += b.rgb[0] * w; sg += b.rgb[1] * w; sb += b.rgb[2] * w; sa += b.a * w; sw += w;
      }
      o[0] = sr / sw; o[1] = sg / sw; o[2] = sb / sw; o[3] = sa / sw;
      return o;
    },
  });
  layer('silk', {
    name: 'Silk', cat: 'Noise and fluids', desc: 'Folded, flowing fabric made from warped noise.',
    params: [P.pal('dusk'), P.int('sx', 'Scale x', 1, 16, 4), P.int('sy', 'Scale y', 1, 8, 1), P.range('warp', 'Warp', 0, 4, 0.05, 1.8), P.range('flow', 'Flow', 0, 2, 0.05, 0.6), P.speed(1), P.range('sheen', 'Sheen', 0, 1, 0.01, 0.4), P.seed(23)],
    sample(u, v, t, p, o) {
      flow(t, p); const ox = p._fx, oy = p._fy;
      const q = fbm(u * p.sx + ox, v * p.sy + oy, p.sx, p.sy, 3, p.seed);
      const n = fbm(u * p.sx + p.warp * q, v * p.sy + p.warp * q * 0.7 - oy, p.sx, p.sy, 4, p.seed + 5);
      lutAt(p._stops, clamp(n * 1.3 - 0.15), o);
      const s = Math.pow(0.5 + 0.5 * Math.sin(TAU * (n * 3 + q)), 6) * p.sheen;
      o[0] = clamp(o[0] + s); o[1] = clamp(o[1] + s); o[2] = clamp(o[2] + s);
      return o;
    },
  });
  layer('spiral', {
    name: 'Spiral', cat: 'Patterns', desc: 'Hypnotic spiral arms that rotate.',
    params: [P.pal('neonPop', true), P.int('arms', 'Arms', 1, 12, 3), P.range('twist', 'Twist', -6, 6, 0.1, 2), P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.speed(1), P.bool('hard', 'Hard bands', false)],
    sample(u, v, t, p, o, c) {
      const dx = (u - p.cx) * c.A, dy = v - p.cy, r = Math.hypot(dx, dy) + 1e-4;
      let f = fract((Math.atan2(dy, dx) / TAU) * p.arms + Math.log(r) * p.twist * 0.5 - t * p.speed);
      if (p.hard) f = f < 0.5 ? 0.25 : 0.75;
      return lutAt(p._stops, f, o);
    },
  });
  layer('tiedye', {
    name: 'Tie-dye', cat: 'Noise and fluids', desc: 'Swirled dye spiralling out from the middle.',
    params: [P.pal('candy', true), P.int('arms', 'Arms', 1, 8, 2), P.range('twist', 'Twist', 0, 20, 0.5, 8), P.range('noise', 'Bleed', 0, 2, 0.05, 0.5), P.speed(1), P.seed(29)],
    sample(u, v, t, p, o, c) {
      const dx = (u - 0.5) * c.A, dy = v - 0.5, r = Math.hypot(dx, dy);
      const n = fbm(u * 6, v * 2, 6, 2, 3, p.seed);
      return lutAt(p._stops, fract((Math.atan2(dy, dx) / TAU) * p.arms + r * p.twist * 0.3 + n * p.noise - t * p.speed), o);
    },
  });
  layer('hexgrid', {
    name: 'Hexagons', cat: 'Patterns', desc: 'Honeycomb tiles with a ripple of colour running through.',
    params: [P.pal('mint'), P.int('cols', 'Columns', 2, 60, 18), P.range('border', 'Border', 0, 0.4, 0.01, 0.08), P.color('borderColor', 'Border colour', '#0b1418'), P.range('random', 'Randomness', 0, 1, 0.01, 0.5), P.speed(1), P.seed(37)],
    prepare: colorPrep(['borderColor']),
    sample(u, v, t, p, o, c) {
      const s = p.cols / c.A, x = u * p.cols, y = v * s * 1.1547;
      const r = Math.floor(y), off = r & 1 ? 0.5 : 0, q = Math.floor(x + off);
      let best = 9, bx = 0, by = 0;
      for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
        const rr = r + j, oo = rr & 1 ? 0.5 : 0, qq = q + i, cx = qq - oo + 0.5, cy = rr + 0.5;
        const d = Math.hypot(x - cx, (y - cy) * 0.866);
        if (d < best) { best = d; bx = qq; by = rr; }
      }
      const h = hash2(wrap(bx, p.cols), by, p.seed);
      lutAt(p._stops, fract(h * p.random + bx / p.cols - t * p.speed), o);
      const e = smoothstep(0.5 - p.border, 0.52 - p.border, best);
      for (let k = 0; k < 3; k++) o[k] = mix(o[k], p._borderColor[k], e);
      return o;
    },
  });
  layer('scales', {
    name: 'Scales', cat: 'Patterns', desc: 'Overlapping fish or dragon scales with shading.',
    params: [P.pal('ocean'), P.int('cols', 'Columns', 2, 60, 20), P.range('shade', 'Shading', 0, 1, 0.01, 0.6), P.speed(1, 'speed', 'Shimmer')],
    sample(u, v, t, p, o, c) {
      const rows = Math.max(1, Math.round(p.cols / c.A * 1.6));
      const y = v * rows, r = Math.floor(y), fy = y - r;
      const x = u * p.cols + (r & 1 ? 0.5 : 0), fx = fract(x) - 0.5;
      const d = Math.hypot(fx, fy - 1) ;
      const inside = d < 0.75;
      const f = inside ? clamp(1 - d / 0.75) : 0;
      lutAt(p._stops, fract((inside ? f * 0.5 : 0.9) + (Math.floor(x) / p.cols) * 0.2 - t * p.speed + 1), o);
      const sh = 1 - p.shade * (1 - fy);
      o[0] *= sh; o[1] *= sh; o[2] *= sh;
      return o;
    },
  });
  layer('truchet', {
    name: 'Maze arcs', cat: 'Patterns', desc: 'Random quarter-circle tiles forming endless winding paths.',
    params: [P.color('line', 'Line colour', '#ffd166'), P.color('bg', 'Background', '#1b1530'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.int('cols', 'Columns', 2, 60, 20), P.range('width', 'Line width', 0.02, 0.4, 0.01, 0.12), P.int('flips', 'Tile flips per loop', 0, 8, 1), P.seed(43)],
    prepare: colorPrep(['line', 'bg']),
    sample(u, v, t, p, o, c) {
      const rows = Math.max(1, Math.round(p.cols / c.A)), x = u * p.cols, y = v * rows;
      const ix = Math.floor(x), iy = Math.floor(y); let fx = x - ix, fy = y - iy;
      const flip = hash2(ix, iy, p.seed + Math.floor(t * p.flips) * (hash2(ix, iy, 7) < 0.3 ? 1 : 0)) > 0.5;
      if (flip) fx = 1 - fx;
      const d = Math.min(Math.abs(Math.hypot(fx, fy) - 0.5), Math.abs(Math.hypot(1 - fx, 1 - fy) - 0.5));
      const m = 1 - smoothstep(p.width * 0.5, p.width * 0.5 + 0.04, d);
      o[0] = mix(p._bg[0], p._line[0], m); o[1] = mix(p._bg[1], p._line[1], m); o[2] = mix(p._bg[2], p._line[2], m); o[3] = mix(p.bgAlpha, 1, m);
      return o;
    },
  });
  layer('plaid', {
    name: 'Plaid', cat: 'Patterns', desc: 'Woven tartan of crossing colour stripes.',
    params: [P.pal('blood'), P.int('cols', 'Stripes across', 1, 24, 6), P.int('rows', 'Stripes down', 1, 12, 2), P.range('mix', 'Weave mix', 0, 1, 0.01, 0.5), P.speed(1, 'sx', 'Scroll'), P.bool('weave', 'Fine weave lines', true)],
    prepare(p) { p._pa = [0, 0, 0, 0]; p._pb = [0, 0, 0, 0]; },
    sample(u, v, t, p, o, c) {
      const a = p._pa, b = p._pb;
      lutAt(p._stops, fract(u * p.cols - t * p.sx), a); lutAt(p._stops, fract(v * p.rows), b);
      let m = p.mix;
      if (p.weave) { const X = Math.floor(u * c.W), Y = Math.floor(v * c.H); m = ((X + Y) & 3) < 2 ? clamp(m + 0.2) : clamp(m - 0.2); }
      for (let k = 0; k < 4; k++) o[k] = mix(a[k], b[k], m);
      return o;
    },
  });
  layer('liquid', {
    name: 'Liquid metal', cat: 'Materials', desc: 'Molten chrome that ripples and reflects.',
    params: [P.stops('chrome'), P.int('sx', 'Scale x', 1, 16, 4), P.int('sy', 'Scale y', 1, 8, 2), P.range('warp', 'Melt', 0, 1.5, 0.01, 0.6), P.range('flow', 'Flow', 0, 2, 0.05, 0.7), P.speed(1), P.seed(53)],
    sample(u, v, t, p, o) {
      flow(t, p); const ox = p._fx, oy = p._fy;
      const n = fbm(u * p.sx + ox, v * p.sy + oy, p.sx, p.sy, 4, p.seed);
      return lutAt(p._stops, clamp(v + (n - 0.5) * p.warp * 1.6), o);
    },
  });
  layer('topo', {
    name: 'Contour lines', cat: 'Noise and fluids', desc: 'Topographic map lines that slowly shift.',
    params: [P.pal('mint'), P.color('lineColor', 'Line colour', '#e6fff4'), P.int('levels', 'Lines', 2, 40, 12), P.range('width', 'Line width', 0.01, 0.5, 0.01, 0.12), P.int('sx', 'Scale x', 1, 16, 4), P.int('sy', 'Scale y', 1, 8, 2), P.range('flow', 'Flow', 0, 2, 0.05, 0.5), P.speed(1), P.seed(61)],
    prepare: colorPrep(['lineColor']),
    sample(u, v, t, p, o) {
      flow(t, p); const ox = p._fx, oy = p._fy;
      const n = fbm(u * p.sx + ox, v * p.sy + oy, p.sx, p.sy, 4, p.seed);
      lutAt(p._stops, n, o);
      const g = fract(n * p.levels), d = Math.min(g, 1 - g);
      const m = 1 - smoothstep(p.width * 0.5, p.width * 0.5 + 0.08, d);
      for (let k = 0; k < 3; k++) o[k] = mix(o[k], p._lineColor[k], m);
      return o;
    },
  });
  layer('galaxyArms', {
    name: 'Galaxy swirl', cat: 'Noise and fluids', desc: 'A rotating spiral galaxy with a bright core.',
    params: [P.pal('galaxy'), P.int('arms', 'Arms', 1, 6, 2), P.range('twist', 'Twist', 0, 12, 0.1, 5), P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.range('size', 'Size', 0.1, 2, 0.01, 0.8), P.speed(1, 'speed', 'Rotation'), P.seed(67)],
    sample(u, v, t, p, o, c) {
      const dx = (u - p.cx) * c.A, dy = v - p.cy, r = Math.hypot(dx, dy) / p.size;
      const a = Math.atan2(dy, dx) - TAU * t * p.speed;
      const arm = 0.5 + 0.5 * Math.cos(a * p.arms - r * p.twist * 2);
      const n = vnoise(Math.cos(a) * r * 8 + 20, Math.sin(a) * r * 8 + 20, 999, 999, p.seed);
      const f = clamp((arm * 0.7 + n * 0.3) * Math.exp(-r * 2.2) * 1.6 + Math.exp(-r * r * 60) * 0.8);
      return lutAt(p._stops, f, o);
    },
  });

  // =====================================================================================
  // Effects
  // =====================================================================================
  effect('ripple', {
    name: 'Ripple', cat: 'Distortion', desc: 'Circular water ripples spreading from a point.',
    params: [P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.int('waves', 'Rings', 1, 30, 8), P.range('amp', 'Strength (px)', 0, 10, 0.1, 2), P.speed(1)],
    run(buf, W, H, t, p) {
      return remap(buf, W, H, (x, y, s) => {
        const dx = x - p.cx * W, dy = y - p.cy * H, d = Math.hypot(dx, dy) + 1e-3;
        const k = p.amp * Math.sin(TAU * ((d / H) * p.waves * 0.5 - t * p.speed));
        s[0] = x + (dx / d) * k; s[1] = y + (dy / d) * k;
      });
    },
  });
  effect('bulge', {
    name: 'Bulge and pinch', cat: 'Distortion', desc: 'Magnifies or pinches a round area; can breathe.',
    params: [P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.range('radius', 'Radius', 0.1, 3, 0.01, 0.8), P.range('strength', 'Strength', -1, 1, 0.01, 0.5), P.speed(0, 'speed', 'Breaths per loop')],
    run(buf, W, H, t, p) {
      const s = p.strength * (p.speed ? Math.sin(TAU * t * p.speed) : 1), R = p.radius * H * 0.5;
      return remap(buf, W, H, (x, y, out) => {
        const dx = x - p.cx * W, dy = y - p.cy * H, d = Math.hypot(dx, dy);
        if (d >= R) { out[0] = x; out[1] = y; return; }
        const k = 1 - s * Math.pow(1 - d / R, 2);
        out[0] = p.cx * W + dx * k; out[1] = p.cy * H + dy * k;
      });
    },
  });
  effect('haze', {
    name: 'Heat haze', cat: 'Distortion', desc: 'Shimmering air, like heat over a road.',
    params: [P.range('amount', 'Strength (px)', 0, 8, 0.1, 1.5), P.int('scale', 'Scale', 1, 16, 6), P.speed(1, 'speed', 'Rise speed', 1, 8)],
    run(buf, W, H, t, p) {
      const sy = Math.max(1, Math.round(p.scale / (W / H) * 2));
      return remap(buf, W, H, (x, y, s) => {
        const n = vnoise((x / W) * p.scale, (y / H + t * p.speed) * sy, p.scale, sy, 5) - 0.5;
        s[0] = x + n * 2 * p.amount; s[1] = y + n * p.amount * 0.5;
      });
    },
  });
  effect('zoomBlur', {
    name: 'Zoom blur', cat: 'Distortion', desc: 'Streaks rushing out from a point, like warp speed.',
    params: [P.range('cx', 'Centre x', 0, 1, 0.01, 0.5), P.range('cy', 'Centre y', 0, 1, 0.01, 0.5), P.range('strength', 'Strength', 0, 0.5, 0.01, 0.12), P.int('pulse', 'Pulses per loop', 0, 8, 0)],
    run(buf, W, H, t, p) {
      const s = p.strength * (p.pulse ? 0.5 + 0.5 * Math.sin(TAU * t * p.pulse) : 1), N = 10, out = new Float32Array(buf.length), o = [0, 0, 0, 0];
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        let r = 0, g = 0, b = 0, a = 0;
        for (let k = 0; k < N; k++) { const f = 1 - (s * k) / N; bilinear(buf, W, H, p.cx * W + (x + 0.5 - p.cx * W) * f, p.cy * H + (y + 0.5 - p.cy * H) * f, o); r += o[0] * o[3]; g += o[1] * o[3]; b += o[2] * o[3]; a += o[3]; }
        const i = (y * W + x) * 4; out[i] = a ? r / a : 0; out[i + 1] = a ? g / a : 0; out[i + 2] = a ? b / a : 0; out[i + 3] = a / N;
      }
      return out;
    },
  });
  effect('motionBlur', {
    name: 'Motion blur', cat: 'Distortion', desc: 'Smears the image in one direction.',
    params: [P.range('angle', 'Angle', 0, 360, 1, 0), P.range('length', 'Length (px)', 0, 30, 0.5, 6)],
    run(buf, W, H, t, p) {
      const a = (p.angle * Math.PI) / 180, dx = Math.cos(a), dy = Math.sin(a), N = Math.max(2, Math.ceil(p.length)), out = new Float32Array(buf.length), o = [0, 0, 0, 0];
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        let r = 0, g = 0, b = 0, al = 0;
        for (let k = 0; k < N; k++) { const s = (k / (N - 1) - 0.5) * p.length; bilinear(buf, W, H, x + 0.5 + dx * s, y + 0.5 + dy * s, o); r += o[0] * o[3]; g += o[1] * o[3]; b += o[2] * o[3]; al += o[3]; }
        const i = (y * W + x) * 4; out[i] = al ? r / al : 0; out[i + 1] = al ? g / al : 0; out[i + 2] = al ? b / al : 0; out[i + 3] = al / N;
      }
      return out;
    },
  });
  effect('edgeGlow', {
    name: 'Neon edges', cat: 'Lighting and shape', desc: 'Finds edges and makes them glow like neon tubes.',
    params: [P.range('strength', 'Strength', 0, 4, 0.05, 1.5), P.range('dark', 'Darken the rest', 0, 1, 0.01, 0.6), P.select('colour', 'Edge colour', [['own', 'Image colour'], ['custom', 'Custom']], 'own'), P.color('edge', 'Custom colour', '#00f0ff')],
    prepare: colorPrep(['edge']),
    run(buf, W, H, t, p) {
      const L = new Float32Array(W * H);
      for (let i = 0; i < W * H; i++) L[i] = luma(buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2]);
      const at = (x, y) => L[wrap(y, H) * W + wrap(x, W)];
      const out = new Float32Array(buf);
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const gx = at(x + 1, y - 1) + 2 * at(x + 1, y) + at(x + 1, y + 1) - at(x - 1, y - 1) - 2 * at(x - 1, y) - at(x - 1, y + 1);
        const gy = at(x - 1, y + 1) + 2 * at(x, y + 1) + at(x + 1, y + 1) - at(x - 1, y - 1) - 2 * at(x, y - 1) - at(x + 1, y - 1);
        const e = clamp(Math.hypot(gx, gy) * p.strength), i = (y * W + x) * 4;
        for (let k = 0; k < 3; k++) { const base = buf[i + k] * (1 - p.dark); const ec = p.colour === 'own' ? clamp(buf[i + k] * 1.5 + 0.2) : p._edge[k]; out[i + k] = clamp(mix(base, ec, e)); }
      }
      return out;
    },
  });
  effect('vhs', {
    name: 'VHS tape', cat: 'Texture', desc: 'Wobbly lines, colour bleed and a rolling noise band.',
    params: [P.range('amount', 'Amount', 0, 1, 0.01, 0.5), P.int('rate', 'Jitter changes per loop', 1, 50, 20), P.speed(1, 'roll', 'Noise band rolls'), P.seed(71)],
    run(buf, W, H, t, p, c) {
      const step = Math.floor(t * p.rate), out = new Float32Array(buf.length), band = fract(t * p.roll);
      for (let y = 0; y < H; y++) {
        const jit = Math.round((hash2(y, step, p.seed) - 0.5) * 3 * p.amount), inBand = Math.abs(y / H - band) < 0.06;
        for (let x = 0; x < W; x++) {
          const i = (y * W + x) * 4, j = (y * W + wrap(x - jit, W)) * 4, jr = (y * W + wrap(x - jit + Math.round(2 * p.amount), W)) * 4;
          let r = buf[jr], g = buf[j + 1], b = buf[j + 2];
          if (inBand) { const n = hash2(x, y, c.frame + 3) * p.amount; r = clamp(r + n); g = clamp(g + n); b = clamp(b + n); }
          const l = luma(r, g, b);
          out[i] = mix(r, l, 0.2 * p.amount); out[i + 1] = mix(g, l, 0.2 * p.amount); out[i + 2] = mix(b, l, 0.2 * p.amount); out[i + 3] = Math.max(buf[j + 3], buf[jr + 3]);
        }
      }
      return out;
    },
  });
  effect('dither', {
    name: 'Pixel-art dither', cat: 'Texture', desc: 'Few colours with a checkered dither, like retro games.',
    params: [P.int('levels', 'Levels per channel', 2, 8, 4), P.int('size', 'Pixel size', 1, 8, 2), P.range('amount', 'Dither strength', 0, 1, 0.01, 0.8)],
    run(buf, W, H, t, p) {
      const B = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5], n = p.levels - 1, s = p.size;
      const out = new Float32Array(buf.length);
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const sx = Math.min(W - 1, Math.floor(x / s) * s), sy = Math.min(H - 1, Math.floor(y / s) * s);
        const i = (y * W + x) * 4, j = (sy * W + sx) * 4, d = (B[((sy / s) & 3) * 4 + ((sx / s) & 3)] / 16 - 0.5) * p.amount / n;
        for (let k = 0; k < 3; k++) out[i + k] = clamp(Math.round((buf[j + k] + d) * n) / n);
        out[i + 3] = buf[j + 3];
      }
      return out;
    },
  });
  effect('vintage', {
    name: 'Vintage', cat: 'Colour', desc: 'Warm, faded old-photo colours.',
    params: [P.range('amount', 'Amount', 0, 1, 0.01, 0.7), P.range('fade', 'Fade', 0, 0.5, 0.01, 0.12)],
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i) => {
        const r = buf[i], g = buf[i + 1], b = buf[i + 2];
        const sr = r * 0.393 + g * 0.769 + b * 0.189, sg = r * 0.349 + g * 0.686 + b * 0.168, sb = r * 0.272 + g * 0.534 + b * 0.131;
        buf[i] = clamp(mix(r, sr, p.amount) * (1 - p.fade) + p.fade); buf[i + 1] = clamp(mix(g, sg, p.amount) * (1 - p.fade) + p.fade * 0.9); buf[i + 2] = clamp(mix(b, sb, p.amount) * (1 - p.fade) + p.fade * 0.7);
      });
    },
  });
  effect('threshold', {
    name: 'Two-tone cut', cat: 'Colour', desc: 'Hard split into two colours at a brightness level; the level can sweep.',
    params: [P.color('dark', 'Dark', '#14002e'), P.color('light', 'Light', '#ffe066'), P.range('level', 'Level', 0, 1, 0.01, 0.5), P.range('soft', 'Softness', 0, 0.5, 0.01, 0.04), P.speed(0, 'sweep', 'Sweeps per loop')],
    prepare: colorPrep(['dark', 'light']),
    run(buf, W, H, t, p) {
      const lv = p.sweep ? 0.5 + 0.4 * Math.sin(TAU * t * p.sweep) : p.level;
      return eachPx(buf, W, H, (i) => { const m = smoothstep(lv - p.soft, lv + p.soft + 1e-3, luma(buf[i], buf[i + 1], buf[i + 2])); for (let k = 0; k < 3; k++) buf[i + k] = mix(p._dark[k], p._light[k], m); });
    },
  });
  effect('sharpen', {
    name: 'Sharpen', cat: 'Texture', desc: 'Crisper details.',
    params: [P.range('amount', 'Amount', 0, 3, 0.05, 0.8)],
    run(buf, W, H, t, p) {
      const out = new Float32Array(buf);
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4, n0 = (y * W + wrap(x - 1, W)) * 4, n1 = (y * W + wrap(x + 1, W)) * 4, n2 = (wrap(y - 1, H) * W + x) * 4, n3 = (wrap(y + 1, H) * W + x) * 4;
        for (let k = 0; k < 3; k++) out[i + k] = clamp(buf[i + k] + p.amount * (4 * buf[i + k] - buf[n0 + k] - buf[n1 + k] - buf[n2 + k] - buf[n3 + k]) * 0.25);
      }
      return out;
    },
  });
  effect('lightLeak', {
    name: 'Light leaks', cat: 'Lighting and shape', desc: 'Soft coloured light drifting across, like film light leaks.',
    params: [P.pal('sunset'), P.int('count', 'Leaks', 1, 6, 3), P.range('size', 'Size', 0.1, 2, 0.01, 0.7), P.range('strength', 'Strength', 0, 1.5, 0.01, 0.5), P.speed(1), P.seed(83)],
    prepare(p) {
      const r = PL.rng(p.seed * 17);
      p._leaks = Array.from({ length: p.count }, () => ({ x: r(), y: r(), ph: r(), col: PL.pickWeighted(p.stops, r()) })).map((l) => Object.assign(l, { rgb: rgb01(l.col.c) }));
    },
    run(buf, W, H, t, p, c) {
      const L = p._leaks.map((l) => ({ x: fract(l.x + t * p.speed), y: l.y + 0.2 * Math.sin(TAU * (t + l.ph)), rgb: l.rgb }));
      return eachPx(buf, W, H, (i, u, v) => {
        for (const l of L) {
          let dx = Math.abs(u - l.x); dx = Math.min(dx, 1 - dx); dx *= c.A;
          const k = Math.exp(-(dx * dx + (v - l.y) * (v - l.y)) / (p.size * p.size * 0.15)) * p.strength;
          for (let q = 0; q < 3; q++) buf[i + q] = clamp(buf[i + q] + l.rgb[q] * k);
        }
      });
    },
  });
  effect('colorBalance', {
    name: 'Colour balance', cat: 'Colour', desc: 'Tint the shadows and the highlights separately.',
    params: [P.color('shadows', 'Shadow tint', '#2b3cff'), P.range('sAmt', 'Shadow amount', 0, 1, 0.01, 0.3), P.color('highs', 'Highlight tint', '#ffb347'), P.range('hAmt', 'Highlight amount', 0, 1, 0.01, 0.3)],
    prepare: colorPrep(['shadows', 'highs']),
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i) => {
        const l = luma(buf[i], buf[i + 1], buf[i + 2]), ws = (1 - l) * p.sAmt, wh = l * p.hAmt;
        for (let k = 0; k < 3; k++) buf[i + k] = clamp(buf[i + k] + (p._shadows[k] - 0.5) * ws + (p._highs[k] - 0.5) * wh);
      });
    },
  });
  effect('shimmer', {
    name: 'Shimmer bands', cat: 'Lighting and shape', desc: 'Bright bands of light rolling across the whole paint.',
    params: [P.range('angle', 'Angle', 0, 360, 1, 60), P.int('count', 'Bands', 1, 8, 2), P.range('width', 'Width', 0.02, 0.5, 0.01, 0.12), P.range('strength', 'Strength', 0, 1, 0.01, 0.45), P.speed(1)],
    run(buf, W, H, t, p, c) {
      const r = (p.angle * Math.PI) / 180, dx = Math.sin(r), dy = -Math.cos(r), L = Math.abs(dx) * c.A + Math.abs(dy);
      return eachPx(buf, W, H, (i, u, v) => {
        const f = ((u - 0.5) * c.A * dx + (v - 0.5) * dy) / L + 0.5;
        const g = fract(f * p.count - t * p.speed), d = Math.min(g, 1 - g) / p.count;
        const k = Math.exp(-(d * d) / (p.width * p.width * 0.05)) * p.strength;
        for (let q = 0; q < 3; q++) buf[i + q] = clamp(buf[i + q] + k);
      });
    },
  });
  effect('outlineFx', {
    name: 'Inner border', cat: 'Lighting and shape', desc: 'A frame around the paint edges, which shows as a rim on the name.',
    params: [P.color('color', 'Colour', '#ffffff'), P.range('width', 'Width', 0, 0.3, 0.005, 0.06), P.range('soft', 'Softness', 0, 0.2, 0.005, 0.04), P.range('alpha', 'Opacity', 0, 1, 0.01, 0.7)],
    prepare: colorPrep(['color']),
    run(buf, W, H, t, p, c) {
      return eachPx(buf, W, H, (i, u, v) => {
        const d = Math.min(u * c.A, (1 - u) * c.A, v, 1 - v);
        const m = (1 - smoothstep(p.width, p.width + p.soft + 1e-3, d)) * p.alpha;
        for (let q = 0; q < 3; q++) buf[i + q] = mix(buf[i + q], p._color[q], m);
      });
    },
  });

  // =====================================================================================
  // Flow: a "stream function" psi built from random sine waves (8 per detail level, each level twice
  // as fine and half as strong). Whole-number x frequencies make it tile across the paint, and each
  // wave's phase wobbles once per speed cycle, so every loop is seamless. Unlike value noise it has
  // no grid, and its gradient is exact. The curl (dpsi/dy, -dpsi/dx) is a swirling flow with no
  // sources or sinks, and the contour lines of psi are exactly its streamlines.
  // Shared params: sx, sy, oct, flow (churn), speed, seed.
  // =====================================================================================
  const FLOW_K = 1; // typical flow speed in "scale" units, used to scale colours and warps
  const PSI_K = 2; // psi spans about 0,5, so Lines x 2 gives roughly that many streamlines
  const FL = new Float64Array(3);
  function flowPrep(p) {
    const r = PL.rng(p.seed * 7919 + 13), waves = [];
    for (let o = 0; o < p.oct; o++) {
      const f = 2 ** o;
      for (let i = 0; i < 8; i++) {
        const kx = Math.round((r() * 2 - 1) * p.sx * f), ky = (r() * 2 - 1) * p.sy * f;
        waves.push({ kx: TAU * kx, ky: TAU * ky, a: 1 / f, ph: TAU * r(), wob: TAU * r(), dir: r() < 0.5 ? -1 : 1 });
      }
    }
    let s = 0; for (const w of waves) s += (w.a * w.a) / 2;
    const n = 0.1 / Math.sqrt(s); for (const w of waves) w.a *= n; // psi has a spread of 0,1 whatever the detail
    p._waves = waves; p._ft = null;
  }
  function flowAt(u, v, t, p, out) {
    const W = p._waves;
    if (p._ft !== t) { p._ft = t; p._ph = W.map((w) => w.ph + p.flow * Math.sin(w.wob + TAU * t * p.speed * w.dir)); }
    let n = 0, dx = 0, dy = 0;
    for (let i = 0; i < W.length; i++) {
      const w = W[i], a = w.kx * u + w.ky * v + p._ph[i], c = w.a * Math.cos(a);
      n += w.a * Math.sin(a); dx += c * w.kx; dy += c * w.ky;
    }
    out[0] = n; out[1] = dx / p.sx; out[2] = dy / p.sy; // psi, then its slope per unit of scale x and y
    return out;
  }
  const flowParams = (sx, sy, oct, churn, seed) => [P.int('sx', 'Scale x', 1, 32, sx), P.int('sy', 'Scale y', 1, 16, sy), P.int('oct', 'Detail', 1, 5, oct), P.range('flow', 'Churn', 0, 3, 0.05, churn), P.speed(1), P.seed(seed)];

  layer('flownoise', {
    name: 'Flow noise', cat: 'Noise and fluids', desc: 'A swirling flow map: colour shows which way the noise flows, or how fast. Flow map gives the red and green texture game engines use.',
    params: [P.pal('rainbowLoop', true), P.select('mode', 'Show', [['angle', 'Direction'], ['speed', 'Speed'], ['map', 'Flow map (red, green)']], 'angle'), P.range('contrast', 'Contrast', 0.2, 4, 0.05, 1), ...flowParams(6, 2, 3, 0.8, 91)],
    prepare: flowPrep,
    sample(u, v, t, p, o, c) {
      const f = flowAt(u, v, t, p, FL);
      const vx = f[2], vy = (-f[1] * p.sx) / (c.A * p.sy); // flow on screen, in units of the paint height
      const m = Math.hypot(vx, vy), k = clamp((m / FLOW_K) * p.contrast);
      if (p.mode === 'map') {
        const s = (p.contrast * 0.5) / FLOW_K;
        o[0] = clamp(0.5 + vx * s); o[1] = clamp(0.5 + vy * s); o[2] = 0; o[3] = 1;
        return o;
      }
      if (p.mode === 'speed') return lutAt(p._stops, k, o);
      lutAt(p._stops, fract(Math.atan2(vy, vx) / TAU + 0.5), o);
      const b = 0.35 + 0.65 * k; // slow areas sink into shadow so the swirls read
      o[0] *= b; o[1] *= b; o[2] *= b;
      return o;
    },
  });

  layer('flowfield', {
    name: 'Flow field', cat: 'Patterns', desc: 'Streamlines that follow a swirling flow, like a wind map or vector-field chart. Dashes can travel along them.',
    params: [P.pal('aurora'), P.color('bg', 'Background', '#0b1020'), P.range('bgAlpha', 'Background opacity', 0, 1, 0.01, 1), P.range('density', 'Lines', 2, 40, 1, 14), P.range('width', 'Line width (px)', 0.5, 6, 0.1, 1.2), P.range('glow', 'Glow', 0, 1, 0.01, 0.3),
      P.int('dashes', 'Dashes', 0, 16, 6, '0 draws solid lines'), P.speed(1, 'dash', 'Dash travel'), ...flowParams(4, 1, 2, 0.4, 97)],
    prepare(p) { colorPrep(['bg'])(p); flowPrep(p); },
    sample(u, v, t, p, o, c) {
      const f = flowAt(u, v, t, p, FL);
      const gx = (f[1] * p.sx) / c.W, gy = (f[2] * p.sy) / c.H; // dpsi per pixel
      const L = p.density * PSI_K, s = f[0] * L, per = Math.hypot(gx, gy) * L + 1e-6; // lines crossed per pixel
      const d = Math.abs(fract(s + 0.5) - 0.5) / per; // pixels to the nearest streamline
      let k = 1 - smoothstep(p.width * 0.5 - 0.5, p.width * 0.5 + 0.5, d);
      if (p.dashes) {
        // dashes ride on a faint line, like particles on a wind map. Distance along the line is the
        // pixel position projected on the local flow direction.
        const m = Math.hypot(gx, gy) + 1e-12, along = (u * c.W * gy - v * c.H * gx) / m;
        const g = fract((along * p.dashes) / c.W - t * p.dash);
        k *= 0.25 + 0.75 * smoothstep(0, 0.06, g) * (1 - smoothstep(0.5, 0.56, g));
      }
      if (p.glow) k = Math.max(k, p.glow * 0.6 * Math.exp(-d / (p.width * 1.5 + 1)));
      lutAt(p._stops, fract(Math.floor(s) * 0.618034), o); // each streamline keeps its own colour
      const ba = p.bgAlpha * (1 - k), a = k + ba;
      if (a <= 0) { o[3] = 0; return o; }
      for (let q = 0; q < 3; q++) o[q] = (o[q] * k + p._bg[q] * ba) / a;
      o[3] = a;
      return o;
    },
  });

  effect('flowwarp', {
    name: 'Flow warp', cat: 'Distortion', desc: 'Pushes the image along a swirling noise flow, like ink in water. More streaks smear it along the flow.',
    params: [P.range('amp', 'Strength (px)', 0, 24, 0.5, 5), P.int('steps', 'Streaks', 1, 8, 1, '1 warps; more smear the image along the flow'), ...flowParams(4, 1, 2, 0.6, 103)],
    prepare: flowPrep,
    run(buf, W, H, t, p) {
      if (!p.amp) return buf;
      const out = new Float32Array(buf.length), s = new Float64Array(4), n = p.steps;
      for (let y = 0, i = 0; y < H; y++) for (let x = 0; x < W; x++, i += 4) {
        const f = flowAt((x + 0.5) / W, (y + 0.5) / H, t, p, FL);
        let dx = f[2] / FLOW_K, dy = (-f[1] / FLOW_K) * (p.sx / p.sy) * (H / W); // flow on screen
        const m = Math.hypot(dx, dy); if (m > 1.5) { dx *= 1.5 / m; dy *= 1.5 / m; }
        dx *= p.amp; dy *= p.amp;
        let r = 0, g = 0, b = 0, a = 0;
        for (let j = 0; j < n; j++) {
          const k = n === 1 ? 1 : j / (n - 1);
          bilinear(buf, W, H, x + 0.5 + dx * k, y + 0.5 + dy * k, s);
          r += s[0] * s[3]; g += s[1] * s[3]; b += s[2] * s[3]; a += s[3];
        }
        if (a > 1e-9) { out[i] = r / a; out[i + 1] = g / a; out[i + 2] = b / a; }
        out[i + 3] = a / n;
      }
      return out;
    },
  });

  PL.LAYER_CATS = ['Scene elements', 'Gradients', 'Patterns', 'Noise and fluids', 'Particles and light', 'Materials'];
}
