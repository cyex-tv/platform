// @ts-nocheck
/* Pack: Stylise, lens and light effects. Same contracts as js/library.js; see js/packs/README.md. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, smoothstep, tri, wrap, hexToRgb, lutAt, hash2, vnoise, fbm, ridged, luma, hsvToRgb } = PL;
  const P = PL.P;
  const { rgb01, colorPrep, eachPx, bilinear, remap, boxBlur } = PL.fx;
  const add = (store, id, def) => { if (store[id]) throw new Error('Duplicate layer or effect id: ' + id); def.id = id; def.pack = 'stylise'; store[id] = def; };
  const layer = (id, def) => add(PL.LAYERS, id, def);
  const effect = (id, def) => add(PL.EFFECTS, id, def);
  for (const c of []) if (!PL.LAYER_CATS.includes(c)) PL.LAYER_CATS.push(c);
  for (const c of ['Stylise', 'Lens and warp', 'Light']) if (!PL.EFFECT_CATS.includes(c)) PL.EFFECT_CATS.push(c);

  // ---- definitions go here ----
  // ---- helpers ----
  const lumArr = (buf, W, H) => { const L = new Float32Array(W * H); for (let k = 0; k < W * H; k++) L[k] = luma(buf[k * 4], buf[k * 4 + 1], buf[k * 4 + 2]); return L; };
  // Sobel on a W x H field (wraps sideways, clamps vertically): gradient and magnitude (about 4x the per-pixel slope).
  const sobel = (L, W, H) => {
    const gx = new Float32Array(W * H), gy = new Float32Array(W * H), m = new Float32Array(W * H);
    for (let y = 0; y < H; y++) {
      const ya = (y > 0 ? y - 1 : 0) * W, yc = y * W, yb = (y < H - 1 ? y + 1 : H - 1) * W;
      for (let x = 0; x < W; x++) {
        const xa = x > 0 ? x - 1 : W - 1, xb = x < W - 1 ? x + 1 : 0;
        const a = L[ya + xa], b = L[ya + x], c = L[ya + xb], d = L[yc + xa], f = L[yc + xb], g = L[yb + xa], h = L[yb + x], k = L[yb + xb];
        const sx = c + 2 * f + k - a - 2 * d - g, sy = g + 2 * h + k - a - 2 * b - c, i = yc + x;
        gx[i] = sx; gy[i] = sy; m[i] = Math.hypot(sx, sy);
      }
    }
    return { gx, gy, m };
  };
  // Bilinear sample of a one-channel W x H field (wraps sideways, clamps vertically).
  const sample1 = (A, W, H, x, y) => {
    x -= 0.5; y -= 0.5;
    const x0 = Math.floor(x), y0 = Math.floor(y), fx = x - x0, fy = y - y0;
    const xa = wrap(x0, W), xb = wrap(x0 + 1, W), ya = Math.min(H - 1, Math.max(0, y0)) * W, yb = Math.min(H - 1, Math.max(0, y0 + 1)) * W;
    const m1 = A[ya + xa] + (A[ya + xb] - A[ya + xa]) * fx, m2 = A[yb + xa] + (A[yb + xb] - A[yb + xa]) * fx;
    return m1 + (m2 - m1) * fy;
  };
  const reflect = (v, n) => { const m = ((v % (2 * n)) + 2 * n) % (2 * n); return m > n ? 2 * n - m : m; };
  const wd = (d, W) => d - Math.round(d / W) * W; // shortest signed distance around the width
  // Adds light to a pixel; where the pixel is see-through the light raises its alpha too.
  const addLight = (b, i, r, g, bl) => {
    const a = b[i + 3], k = Math.max(r, g, bl);
    if (k <= 0) return;
    if (a >= 0.999) { b[i] = clamp(b[i] + r); b[i + 1] = clamp(b[i + 1] + g); b[i + 2] = clamp(b[i + 2] + bl); return; }
    const a2 = clamp(a + (1 - a) * clamp(k));
    b[i] = clamp((b[i] * a + r) / a2); b[i + 1] = clamp((b[i + 1] * a + g) / a2); b[i + 2] = clamp((b[i + 2] * a + bl) / a2); b[i + 3] = a2;
  };
  // Horizontal wrap-around box blur of a three-channel field.
  const hBlur3 = (src, W, H, r) => {
    const out = new Float32Array(src.length), n = 2 * r + 1;
    for (let y = 0; y < H; y++) {
      const row = y * W; let s0 = 0, s1 = 0, s2 = 0;
      for (let k = -r; k <= r; k++) { const i = (row + wrap(k, W)) * 3; s0 += src[i]; s1 += src[i + 1]; s2 += src[i + 2]; }
      for (let x = 0; x < W; x++) {
        const j = (row + x) * 3; out[j] = s0 / n; out[j + 1] = s1 / n; out[j + 2] = s2 / n;
        const a = (row + wrap(x + r + 1, W)) * 3, b = (row + wrap(x - r, W)) * 3;
        s0 += src[a] - src[b]; s1 += src[a + 1] - src[b + 1]; s2 += src[a + 2] - src[b + 2];
      }
    }
    return out;
  };
  // Voronoi cells over an nx x ny grid (one point per cell, wraps sideways): fn(i, cell, d1, d2, x, y) per pixel.
  const voroEach = (W, H, nx, ny, px, py, fn) => {
    const cw = W / nx, ch = H / ny;
    for (let y = 0; y < H; y++) {
      const gy = Math.min(ny - 1, Math.floor((y + 0.5) / ch));
      for (let x = 0; x < W; x++) {
        const gx = Math.min(nx - 1, Math.floor((x + 0.5) / cw));
        let d1 = 1e9, d2 = 1e9, id = 0;
        for (let dj = -1; dj <= 1; dj++) {
          const cj = gy + dj; if (cj < 0 || cj >= ny) continue;
          for (let di = -1; di <= 1; di++) {
            let ci = gx + di, off = 0;
            if (ci < 0) { ci += nx; off = -W; } else if (ci >= nx) { ci -= nx; off = W; }
            const k = cj * nx + ci, dx = px[k] + off - (x + 0.5), dy = py[k] - (y + 0.5), d = dx * dx + dy * dy;
            if (d < d1) { d2 = d1; d1 = d; id = k; } else if (d < d2) d2 = d;
          }
        }
        fn((y * W + x) * 4, id, Math.sqrt(d1), Math.sqrt(d2), x, y);
      }
    }
  };

  const pickOv = (a, b) => (a < 0.5 ? 2 * a * b : 1 - 2 * (1 - a) * (1 - b)); // overlay blend

  // ======== Stylise ========
  effect('oilPaint', {
    name: 'Oil paint', cat: 'Stylise', desc: 'Smears the image into flat brush patches with crisp edges and a shimmering bristle texture.',
    tags: ['oil', 'painting', 'kuwahara', 'brush', 'painterly', 'smear', 'canvas'],
    params: [P.int('radius', 'Brush size (px)', 1, 6, 3), P.range('punch', 'Colour punch', 0, 1, 0.01, 0.25), P.range('bristle', 'Bristle texture', 0, 1, 0.01, 0.4), P.speed(1, 'speed', 'Shimmer')],
    run(buf, W, H, t, p) {
      const W1 = W + 1, N = W1 * (H + 1), S = [0, 1, 2, 3, 4, 5].map(() => new Float64Array(N)), acc = new Float64Array(6);
      for (let y = 0; y < H; y++) {
        acc.fill(0);
        for (let x = 0; x < W; x++) {
          const i = (y * W + x) * 4, a = buf[i + 3], l = luma(buf[i], buf[i + 1], buf[i + 2]);
          acc[0] += buf[i] * a; acc[1] += buf[i + 1] * a; acc[2] += buf[i + 2] * a; acc[3] += a; acc[4] += l * a; acc[5] += l * l * a;
          const k = (y + 1) * W1 + x + 1, kk = y * W1 + x + 1;
          for (let c = 0; c < 6; c++) S[c][k] = S[c][kk] + acc[c];
        }
      }
      const rect = (c, x0, y0, x1, y1) => S[c][(y1 + 1) * W1 + x1 + 1] - S[c][y0 * W1 + x1 + 1] - S[c][(y1 + 1) * W1 + x0] + S[c][y0 * W1 + x0];
      const r = p.radius, out = new Float32Array(buf.length), q = [[-r, -r, 0, 0], [0, -r, r, 0], [-r, 0, 0, r], [0, 0, r, r]];
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        let best = 1e9, br = 0, bg = 0, bb = 0, found = false;
        for (let k = 0; k < 4; k++) {
          const x0 = Math.max(0, x + q[k][0]), y0 = Math.max(0, y + q[k][1]), x1 = Math.min(W - 1, x + q[k][2]), y1 = Math.min(H - 1, y + q[k][3]);
          const a = rect(3, x0, y0, x1, y1); if (a < 1e-6) continue;
          const ml = rect(4, x0, y0, x1, y1) / a, v = rect(5, x0, y0, x1, y1) / a - ml * ml;
          if (v < best) { best = v; br = rect(0, x0, y0, x1, y1) / a; bg = rect(1, x0, y0, x1, y1) / a; bb = rect(2, x0, y0, x1, y1) / a; found = true; }
        }
        if (!found) { br = buf[i]; bg = buf[i + 1]; bb = buf[i + 2]; }
        const l = luma(br, bg, bb), k = 1 + p.punch, hv = hash2(x, y >> 1, 3);
        const m = 1 + p.bristle * (0.1 * (hv - 0.5) * 2 + 0.1 * Math.sin(TAU * (t * p.speed + hv)));
        out[i] = clamp((l + (br - l) * k) * m); out[i + 1] = clamp((l + (bg - l) * k) * m); out[i + 2] = clamp((l + (bb - l) * k) * m); out[i + 3] = buf[i + 3];
      }
      return out;
    },
  });

  effect('sketch', {
    name: 'Pencil sketch', cat: 'Stylise', desc: 'Turns the image into graphite lines and light hatching on paper; the lines boil slightly.',
    tags: ['pencil', 'sketch', 'drawing', 'line art', 'edges', 'paper', 'graphite', 'hand drawn'],
    params: [P.color('paper', 'Paper', '#f4efe3'), P.color('ink', 'Pencil', '#2a2622'), P.range('strength', 'Line strength', 0.5, 14, 0.1, 5), P.range('shade', 'Shading', 0, 1, 0.01, 0.4), P.range('colour', 'Keep colour', 0, 1, 0.01, 0.2), P.speed(1, 'speed', 'Line boil')],
    prepare: colorPrep(['paper', 'ink']),
    run(buf, W, H, t, p) {
      const L = lumArr(buf, W, H), { m } = sobel(L, W, H), pa = p._paper, ik = p._ink;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const ex = x + 0.5 + 0.8 * Math.sin(TAU * (v * 2 + t * p.speed)), ey = y + 0.5 + 0.8 * Math.cos(TAU * (u * 3 + t * p.speed));
        const e = clamp(sample1(m, W, H, ex, ey) * p.strength);
        const hatch = (0.5 + 0.5 * Math.sin(TAU * (x + y) * 0.25)) * clamp((1 - L[y * W + x]) * 1.4 - 0.2) * p.shade;
        const g = 0.97 + 0.03 * hash2(x, y, 9), ink = clamp(Math.max(e, hatch));
        for (let k = 0; k < 3; k++) { const base = mix(pa[k] * g, buf[i + k] * (0.7 + 0.3 * g), p.colour); buf[i + k] = clamp(mix(base, ik[k], ink)); }
      });
    },
  });

  effect('crossHatch', {
    name: 'Cross-hatch', cat: 'Stylise', desc: 'Draws the image as engraver’s hatching: darker areas get more crossing lines, and the lines slide along.',
    tags: ['hatch', 'engraving', 'etching', 'lines', 'ink', 'drawing', 'comic'],
    params: [P.color('paper', 'Paper', '#f2ecdd'), P.color('ink', 'Ink', '#1d1a2b'), P.range('spacing', 'Line spacing (px)', 2, 10, 0.25, 4), P.range('weight', 'Line weight', 0.1, 0.9, 0.01, 0.4), P.range('colour', 'Keep colour', 0, 1, 0.01, 0.3), P.speed(1, 'speed', 'Lines slide')],
    prepare: colorPrep(['paper', 'ink']),
    run(buf, W, H, t, p) {
      const L = lumArr(buf, W, H), pa = p._paper, ik = p._ink, sp = p.spacing, w = p.weight, ph = t * p.speed;
      const line = (s) => clamp((w - Math.abs(fract(s) - 0.5) * 2) * 2 + 0.5);
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const D = 1 - L[y * W + x];
        let ink = clamp((D - 0.12) * 7) * line((x + y) / sp / 1.414 + ph);
        ink = Math.max(ink, clamp((D - 0.35) * 7) * line((x - y) / sp / 1.414 - ph));
        ink = Math.max(ink, clamp((D - 0.55) * 7) * line(y / sp + ph));
        ink = Math.max(ink, clamp((D - 0.75) * 7) * line(x / sp - ph));
        for (let k = 0; k < 3; k++) buf[i + k] = clamp(mix(mix(pa[k], buf[i + k], p.colour), ik[k], ink));
      });
    },
  });

  effect('neonOutline', {
    name: 'Neon outline', cat: 'Stylise', desc: 'Dims the image and traces its edges with glowing tubes whose colours travel round a palette.',
    tags: ['neon', 'outline', 'edges', 'glow', 'tube', 'sign', 'cyberpunk', 'light'],
    params: [P.pal('neon', true), P.range('strength', 'Edge strength', 0.5, 14, 0.1, 5), P.range('dark', 'Darken the rest', 0, 1, 0.01, 0.7), P.range('glow', 'Glow', 0, 3, 0.05, 1.2), P.int('radius', 'Glow radius (px)', 1, 8, 3), P.range('repeat', 'Colour repeats', 0.25, 4, 0.25, 1), P.speed(1, 'speed', 'Colour cycle')],
    run(buf, W, H, t, p) {
      const L = lumArr(buf, W, H), { m } = sobel(L, W, H), N = W * H, col = new Float32Array(N * 4), o = [0, 0, 0, 0];
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const k = y * W + x, i = k * 4, e = clamp(m[k] * p.strength);
        lutAt(p._stops, fract((x + 0.5) / W * p.repeat + L[k] * 0.35 - t * p.speed), o);
        col[i] = o[0]; col[i + 1] = o[1]; col[i + 2] = o[2]; col[i + 3] = e;
      }
      const bl = boxBlur(col, W, H, p.radius);
      return eachPx(buf, W, H, (i) => {
        const e = col[i + 3], g = bl[i + 3] * p.glow;
        for (let k = 0; k < 3; k++) buf[i + k] = clamp(buf[i + k] * (1 - p.dark) + col[i + k] * e + bl[i + k] * g);
      });
    },
  });

  effect('emboss', {
    name: 'Emboss relief', cat: 'Stylise', desc: 'Presses the image into a raised relief lit from a light that turns once per loop.',
    tags: ['emboss', 'relief', 'bump', 'stone', 'metal', 'carved', '3d', 'light'],
    params: [P.range('depth', 'Depth', 0.5, 20, 0.1, 6), P.range('angle', 'Light angle', 0, 360, 1, 135), P.range('colour', 'Keep colour', 0, 1, 0.01, 0.4), P.speed(1, 'speed', 'Light turns')],
    run(buf, W, H, t, p) {
      const L = lumArr(buf, W, H), { gx, gy } = sobel(L, W, H), a = (p.angle * Math.PI) / 180 + TAU * t * p.speed, dx = Math.cos(a), dy = Math.sin(a);
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const k = y * W + x, s = clamp(0.5 + (gx[k] * dx + gy[k] * dy) * p.depth * 0.25);
        for (let c = 0; c < 3; c++) buf[i + c] = clamp(mix(s, buf[i + c] * s * 2, p.colour));
      });
    },
  });

  effect('stainedGlass', {
    name: 'Stained glass', cat: 'Stylise', desc: 'Splits the image into jagged glass panes with dark lead lines; panes drift and glow in turn.',
    tags: ['stained glass', 'voronoi', 'cells', 'church', 'lead', 'mosaic', 'panes', 'crystal'],
    params: [P.int('cols', 'Panes across', 4, 64, 22), P.int('rows', 'Panes down', 1, 16, 4), P.range('lead', 'Lead width (px)', 0, 4, 0.1, 1.4), P.color('leadc', 'Lead colour', '#14110f'), P.range('light', 'Backlight', 0, 1, 0.01, 0.5), P.range('orbit', 'Pane drift', 0, 0.5, 0.01, 0.15), P.speed(1, 'speed', 'Cycles'), P.seed(5)],
    prepare: colorPrep(['leadc']),
    run(buf, W, H, t, p) {
      const nx = p.cols, ny = p.rows, cw = W / nx, ch = H / ny, n = nx * ny, px = new Float32Array(n), py = new Float32Array(n), ph = new Float32Array(n), cc = new Float32Array(n * 3);
      const bl = boxBlur(buf, W, H, clamp(Math.round(Math.min(cw, ch) / 3), 1, 6)), o = new Float64Array(4), lc = p._leadc;
      for (let j = 0; j < ny; j++) for (let i = 0; i < nx; i++) {
        const k = j * nx + i, ang = TAU * (t * p.speed + hash2(i, j, p.seed + 1));
        px[k] = (i + 0.2 + 0.6 * hash2(i, j, p.seed)) * cw + Math.cos(ang) * p.orbit * cw;
        py[k] = (j + 0.2 + 0.6 * hash2(i, j, p.seed + 3)) * ch + Math.sin(ang) * p.orbit * ch;
        ph[k] = hash2(i, j, p.seed + 2);
        bilinear(bl, W, H, px[k], clamp(py[k], 0.5, H - 0.5), o);
        cc[k * 3] = o[0]; cc[k * 3 + 1] = o[1]; cc[k * 3 + 2] = o[2];
      }
      const rad = 0.6 * Math.max(cw, ch), lw = p.lead * 0.5;
      voroEach(W, H, nx, ny, px, py, (i, id, d1, d2) => {
        const e = (d2 - d1) * 0.5, lead = 1 - smoothstep(lw, lw + 0.9, e);
        const g = 1 + p.light * (0.45 * Math.sin(TAU * (t * p.speed + ph[id])) + 0.4 * (1 - Math.min(1, d1 / rad)));
        for (let k = 0; k < 3; k++) buf[i + k] = clamp(mix(cc[id * 3 + k] * g, lc[k], lead));
      });
      return buf;
    },
  });

  effect('mosaic', {
    name: 'Mosaic tiles', cat: 'Stylise', desc: 'Rebuilds the image from bevelled square tiles with grout; tiles twinkle one after another.',
    tags: ['mosaic', 'tiles', 'grout', 'pixel', 'ceramic', 'bathroom', 'squares', 'bevel'],
    params: [P.int('size', 'Tile size (px)', 3, 24, 8), P.range('grout', 'Grout', 0, 0.5, 0.01, 0.15), P.color('groutc', 'Grout colour', '#1d1a17'), P.range('bevel', 'Bevel', 0, 1, 0.01, 0.5), P.range('shine', 'Twinkle', 0, 1, 0.01, 0.45), P.speed(1, 'speed', 'Twinkles'), P.seed(8)],
    prepare: colorPrep(['groutc']),
    run(buf, W, H, t, p) {
      const s = p.size, nx = Math.ceil(W / s), ny = Math.ceil(H / s), acc = new Float64Array(nx * ny * 5), gc = p._groutc;
      for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4, a = buf[i + 3], k = (Math.floor(y / s) * nx + Math.floor(x / s)) * 5;
        acc[k] += buf[i] * a; acc[k + 1] += buf[i + 1] * a; acc[k + 2] += buf[i + 2] * a; acc[k + 3] += a; acc[k + 4] += 1;
      }
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const tx = Math.floor(x / s), ty = Math.floor(y / s), k = (ty * nx + tx) * 5, sa = acc[k + 3], cnt = acc[k + 4];
        const lx = x - tx * s + 0.5, ly = y - ty * s + 0.5, fx = lx / s, fy = ly / s, e = Math.min(lx, s - lx, ly, s - ly);
        const hv = hash2(tx, ty, p.seed), tone = 1 + 0.08 * (hv - 0.5) + p.shine * 0.4 * Math.sin(TAU * (t * p.speed + hv));
        const bev = p.bevel * 0.3 * ((1 - smoothstep(0.1, 0.4, fx)) + (1 - smoothstep(0.1, 0.4, fy)) - smoothstep(0.6, 0.9, fx) - smoothstep(0.6, 0.9, fy));
        const gm = 1 - smoothstep(p.grout * s * 0.5 - 0.5, p.grout * s * 0.5 + 0.5, e);
        for (let c = 0; c < 3; c++) { const col = sa > 1e-6 ? acc[k + c] / sa : 0; buf[i + c] = clamp(mix(clamp(col * tone + bev), gc[c], gm)); }
        buf[i + 3] = clamp(sa / cnt);
      });
    },
  });

  effect('thermal', {
    name: 'Thermal camera', cat: 'Stylise', desc: 'Maps brightness to heat-camera false colour, with warm air shimmering through it.',
    tags: ['thermal', 'heat', 'infrared', 'false colour', 'predator', 'camera', 'fire', 'scanner'],
    params: [P.pal([{ at: 0, color: '#0b0033' }, { at: 0.2, color: '#4b0a8c' }, { at: 0.4, color: '#c01c5a' }, { at: 0.6, color: '#ff7a12' }, { at: 0.8, color: '#ffd23c' }, { at: 1, color: '#fffbe8' }]), P.range('contrast', 'Contrast', 0.3, 3, 0.01, 1.3), P.range('heat', 'Heat shimmer', 0, 0.5, 0.01, 0.12), P.int('smooth', 'Smoothing (px)', 0, 3, 1), P.speed(1)],
    run(buf, W, H, t, p) {
      const src = p.smooth > 0 ? boxBlur(buf, W, H, p.smooth) : buf, o = [0, 0, 0, 0];
      return eachPx(buf, W, H, (i, u, v) => {
        const l = luma(src[i], src[i + 1], src[i + 2]);
        const f = (l - 0.5) * p.contrast + 0.5 + p.heat * Math.sin(TAU * (t * p.speed + u * 2 + v * 3 + l * 1.5));
        lutAt(p._stops, f, o); buf[i] = o[0]; buf[i + 1] = o[1]; buf[i + 2] = o[2];
      });
    },
  });

  effect('nightVision', {
    name: 'Night vision', cat: 'Stylise', desc: 'Green phosphor goggles: amplified brightness, flickering noise, a dark lens edge and a bright scan band.',
    tags: ['night vision', 'green', 'goggles', 'military', 'phosphor', 'noise', 'scope', 'infrared'],
    params: [P.color('tint', 'Phosphor', '#46ff6a'), P.range('gain', 'Gain', 0.5, 4, 0.01, 1.7), P.range('noise', 'Noise', 0, 1, 0.01, 0.35), P.range('vignette', 'Lens edge', 0, 1, 0.01, 0.6), P.speed(1, 'speed', 'Scan band'), P.speed(3, 'flick', 'Noise flicker'), P.seed(4)],
    prepare: colorPrep(['tint']),
    run(buf, W, H, t, p) {
      const tn = p._tint;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const l = luma(buf[i], buf[i + 1], buf[i + 2]);
        const nz = Math.sin(TAU * (t * p.flick + hash2(x, y, p.seed))) * p.noise * 0.25;
        const bd = fract(u - t * p.speed + 0.5) - 0.5, band = Math.exp(-(bd * bd) / 0.004) * 0.3;
        const d = Math.hypot((u - 0.5) * 1.1, (v - 0.5) * 1.7), vig = 1 - p.vignette * smoothstep(0.35, 0.8, d);
        const g = clamp((Math.pow(clamp(l), 0.8) * p.gain + nz + band) * vig);
        buf[i] = clamp(tn[0] * g + g * g * 0.25); buf[i + 1] = clamp(tn[1] * g + g * g * 0.25); buf[i + 2] = clamp(tn[2] * g + g * g * 0.25);
      });
    },
  });

  effect('splitTone', {
    name: 'Split tone', cat: 'Stylise', desc: 'Tints shadows and highlights in two different colours; the split point swings back and forth.',
    tags: ['split tone', 'grade', 'teal orange', 'cinematic', 'toning', 'shadows', 'highlights', 'duotone'],
    params: [P.color('shadow', 'Shadows', '#1d4f9c'), P.color('light', 'Highlights', '#ffa940'), P.range('amount', 'Amount', 0, 1, 0.01, 0.75), P.range('balance', 'Balance', 0, 1, 0.01, 0.5), P.range('swing', 'Balance swing', 0, 0.5, 0.01, 0.25), P.speed(1, 'speed', 'Swings')],
    prepare: colorPrep(['shadow', 'light']),
    run(buf, W, H, t, p) {
      const sh = p._shadow, hi = p._light, bal = p.balance + p.swing * Math.sin(TAU * t * p.speed);
      return eachPx(buf, W, H, (i) => {
        const l = luma(buf[i], buf[i + 1], buf[i + 2]), w = smoothstep(bal - 0.35, bal + 0.35, l);
        const r = mix(sh[0], hi[0], w), g = mix(sh[1], hi[1], w), b = mix(sh[2], hi[2], w), k = l / Math.max(0.05, luma(r, g, b));
        buf[i] = clamp(mix(buf[i], r * k, p.amount)); buf[i + 1] = clamp(mix(buf[i + 1], g * k, p.amount)); buf[i + 2] = clamp(mix(buf[i + 2], b * k, p.amount));
      });
    },
  });

  effect('palCycle', {
    name: 'Palette cycle', cat: 'Stylise', desc: 'Recolours the image by brightness from a palette and rotates the palette round, like old colour cycling.',
    tags: ['palette', 'colour cycle', 'rotate', 'gradient map', 'psychedelic', 'retro', 'rainbow', 'loop'],
    params: [P.pal('rainbowLoop', true), P.range('bands', 'Bands', 0.25, 6, 0.25, 1.5), P.range('amount', 'Amount', 0, 1, 0.01, 1), P.range('keep', 'Keep brightness', 0, 1, 0.01, 0.5), P.speed(1)],
    run(buf, W, H, t, p) {
      const o = [0, 0, 0, 0];
      return eachPx(buf, W, H, (i) => {
        const l = luma(buf[i], buf[i + 1], buf[i + 2]);
        lutAt(p._stops, fract(l * p.bands - t * p.speed), o);
        const k = mix(1, (l + 0.05) / (luma(o[0], o[1], o[2]) + 0.05), p.keep);
        for (let c = 0; c < 3; c++) buf[i + c] = clamp(mix(buf[i + c], o[c] * k, p.amount));
      });
    },
  });

  effect('contours', {
    name: 'Contour lines', cat: 'Stylise', desc: 'Draws topographic lines along equal brightness; the lines flow uphill once per loop.',
    tags: ['contour', 'topographic', 'map', 'lines', 'elevation', 'level', 'iso', 'terrain'],
    params: [P.int('levels', 'Lines', 3, 24, 10), P.range('width', 'Line width', 0.05, 1, 0.01, 0.35), P.color('line', 'Line colour', '#ffffff'), P.range('dim', 'Dim the image', 0, 1, 0.01, 0.45), P.int('smooth', 'Smoothing (px)', 0, 4, 2), P.speed(1, 'speed', 'Flow')],
    prepare: colorPrep(['line']),
    run(buf, W, H, t, p) {
      const src = p.smooth > 0 ? boxBlur(buf, W, H, p.smooth) : buf, L = lumArr(src, W, H), { m } = sobel(L, W, H), lc = p._line;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const k = y * W + x, s = L[k] * p.levels - t * p.speed, f = fract(s), dd = Math.min(f, 1 - f);
        const aa = Math.min(0.25, Math.max(0.05, m[k] * p.levels * 0.3)), w = p.width * 0.25, ln = 1 - smoothstep(w, w + aa, dd);
        for (let c = 0; c < 3; c++) buf[i + c] = clamp(mix(buf[i + c] * (1 - p.dim), lc[c], ln));
      });
    },
  });

  // ======== Lens and warp ========
  effect('kaleido', {
    name: 'Kaleidoscope', cat: 'Lens and warp', desc: 'Folds the image into spinning mirror wedges, several side by side, while the source slowly orbits.',
    tags: ['kaleidoscope', 'mirror', 'wedges', 'symmetry', 'mandala', 'psychedelic', 'spin'],
    params: [P.int('wedges', 'Wedges', 2, 16, 6), P.int('centres', 'Kaleidoscopes across', 1, 6, 3), P.range('zoom', 'Zoom', 0.2, 3, 0.01, 0.8), P.range('orbitr', 'Source orbit size', 0, 0.5, 0.01, 0.2), P.speed(1, 'speed', 'Wedge spin'), P.speed(1, 'orbit', 'Source orbit')],
    run(buf, W, H, t, p) {
      const seg = TAU / p.wedges, cw = W / p.centres, rot = TAU * t * p.speed, oa = TAU * t * p.orbit;
      const ox = W / 2 + Math.cos(oa) * p.orbitr * W * 0.5, oy = H / 2 + Math.sin(oa) * p.orbitr * H;
      return remap(buf, W, H, (x, y, s) => {
        const dx = x - (Math.floor(x / cw) + 0.5) * cw, dy = y - H / 2;
        let a = Math.atan2(dy, dx) + rot; a = ((a % seg) + seg) % seg; if (a > seg / 2) a = seg - a;
        const r = Math.hypot(dx, dy) * p.zoom;
        s[0] = ox + Math.cos(a) * r; s[1] = reflect(oy + Math.sin(a) * r, H);
      });
    },
  });

  effect('fisheye', {
    name: 'Glass lens', cat: 'Lens and warp', desc: 'A round magnifying lens with a shiny rim glides across the image and wraps round.',
    tags: ['lens', 'fisheye', 'magnify', 'glass', 'sphere', 'zoom', 'bubble', 'loupe'],
    params: [P.range('size', 'Lens size', 0.4, 2, 0.01, 1.1), P.range('power', 'Magnify', 0.1, 0.95, 0.01, 0.6), P.range('rim', 'Shine', 0, 1, 0.01, 0.6), P.range('cy', 'Height', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Passes')],
    run(buf, W, H, t, p) {
      const R = p.size * H * 0.5, cx = fract(0.25 + t * p.speed) * W, cy = p.cy * H;
      const out = remap(buf, W, H, (x, y, s) => {
        const dx = wd(x - cx, W), dy = y - cy, d = Math.hypot(dx, dy);
        if (d >= R) { s[0] = x; s[1] = y; return; }
        const k = 1 - p.power * (1 - (d / R) * (d / R));
        s[0] = cx + dx * k; s[1] = cy + dy * k;
      });
      return eachPx(out, W, H, (i, u, v, x, y) => {
        const dx = wd(x + 0.5 - cx, W) / R, dy = (y + 0.5 - cy) / R, nd = Math.hypot(dx, dy);
        if (nd >= 1) return;
        const spec = Math.exp(-((dx + 0.35) * (dx + 0.35) + (dy + 0.4) * (dy + 0.4)) * 18), ring = smoothstep(0.86, 1, nd) * (0.5 + 0.5 * (dx + dy) * 0.7);
        const k = p.rim * (0.6 * spec + 0.3 * ring);
        out[i] = clamp(out[i] + k); out[i + 1] = clamp(out[i + 1] + k); out[i + 2] = clamp(out[i + 2] + k);
      });
    },
  });

  effect('polarRing', {
    name: 'Polar rings', cat: 'Lens and warp', desc: 'Wraps the image around a centre like a ring, one copy per ring, and turns it.',
    tags: ['polar', 'ring', 'circle', 'wrap', 'radial', 'round', 'coordinates', 'wheel'],
    params: [P.int('rings', 'Rings across', 1, 6, 3), P.range('scale', 'Ring size', 0.5, 3, 0.01, 1.2), P.range('hole', 'Centre hole', 0, 0.8, 0.01, 0.15), P.select('fill', 'Outside the ring', [['tile', 'Repeat'], ['clear', 'Clear']], 'tile'), P.speed(1, 'speed', 'Turns')],
    run(buf, W, H, t, p) {
      const cw = W / p.rings, Rm = H * 0.5 * p.scale;
      const out = remap(buf, W, H, (x, y, s) => {
        const dx = x - (Math.floor(x / cw) + 0.5) * cw, dy = y - H / 2, nr = Math.hypot(dx, dy) / Rm;
        s[0] = fract(Math.atan2(dy, dx) / TAU + t * p.speed) * W; s[1] = reflect((nr - p.hole) / (1 - p.hole) * H, H);
      });
      if (p.fill === 'clear') eachPx(out, W, H, (i, u, v, x, y) => { const dx = x + 0.5 - (Math.floor(x / cw) + 0.5) * cw, dy = y + 0.5 - H / 2; out[i + 3] *= 1 - smoothstep(Rm - 1, Rm, Math.hypot(dx, dy)); });
      return out;
    },
  });

  effect('tunnel', {
    name: 'Zoom tunnel', cat: 'Lens and warp', desc: 'Pulls the image into an endless spiral tunnel that zooms forward in a seamless loop.',
    tags: ['tunnel', 'droste', 'zoom', 'spiral', 'infinite', 'vortex', 'warp', 'hyperspace'],
    params: [P.int('arms', 'Image repeats round', 1, 8, 2), P.range('twist', 'Twist', -2, 2, 0.01, 0.35), P.range('depth', 'Ring density', 0.3, 4, 0.01, 1.4), P.range('dark', 'Dark centre', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Zoom cycles')],
    run(buf, W, H, t, p) {
      const out = remap(buf, W, H, (x, y, s) => {
        const dx = x - W / 2, dy = y - H / 2, r = Math.hypot(dx, dy) + 1e-3, lr = Math.log(r / (H * 0.5));
        s[0] = fract(Math.atan2(dy, dx) / TAU * p.arms + p.twist * lr) * W; s[1] = fract(lr * p.depth - t * p.speed) * H;
      });
      return eachPx(out, W, H, (i, u, v, x, y) => {
        const k = 1 - p.dark * (1 - smoothstep(0, H * 0.45, Math.hypot(x + 0.5 - W / 2, y + 0.5 - H / 2)));
        out[i] *= k; out[i + 1] *= k; out[i + 2] *= k;
      });
    },
  });

  effect('tileGrid', {
    name: 'Tile grid', cat: 'Lens and warp', desc: 'Repeats the whole image as a grid of small copies, with brick offsets and rows that slide in turn.',
    tags: ['tile', 'repeat', 'grid', 'brick', 'pattern', 'copies', 'wallpaper', 'scroll'],
    params: [P.int('cols', 'Columns', 1, 16, 4), P.int('rows', 'Rows', 1, 6, 2), P.range('brick', 'Brick offset', 0, 1, 0.01, 0.5), P.speed(1, 'speed', 'Row slide')],
    run(buf, W, H, t, p) {
      return remap(buf, W, H, (x, y, s) => {
        const v = y / H * p.rows, row = Math.floor(v), odd = row & 1;
        const tu = x / W * p.cols + (odd ? p.brick : 0) + (odd ? -1 : 1) * t * p.speed;
        s[0] = fract(tu) * W; s[1] = fract(v) * H;
      });
    },
  });

  effect('shatter', {
    name: 'Shatter', cat: 'Lens and warp', desc: 'Breaks the image into glass shards that shift and tilt in turn, with dark cracks between them.',
    tags: ['shatter', 'broken glass', 'shards', 'crack', 'fragments', 'pieces', 'voronoi', 'smash'],
    params: [P.int('cols', 'Shards across', 4, 40, 14), P.int('rows', 'Shards down', 1, 10, 3), P.range('push', 'Shift (px)', 0, 16, 0.1, 4), P.range('tilt', 'Tilt', 0, 1, 0.01, 0.3), P.range('crack', 'Cracks', 0, 1, 0.01, 0.6), P.speed(1, 'speed', 'Cycles'), P.seed(11)],
    run(buf, W, H, t, p) {
      const nx = p.cols, ny = p.rows, n = nx * ny, cw = W / nx, ch = H / ny;
      const px = new Float32Array(n), py = new Float32Array(n), ox = new Float32Array(n), oy = new Float32Array(n), rot = new Float32Array(n);
      for (let j = 0; j < ny; j++) for (let i = 0; i < nx; i++) {
        const k = j * nx + i, h3 = hash2(i, j, p.seed + 2), ang = hash2(i, j, p.seed + 1) * TAU, amp = p.push * Math.sin(TAU * (t * p.speed + h3));
        px[k] = (i + 0.15 + 0.7 * hash2(i, j, p.seed)) * cw; py[k] = (j + 0.15 + 0.7 * hash2(i, j, p.seed + 3)) * ch;
        ox[k] = Math.cos(ang) * amp; oy[k] = Math.sin(ang) * amp * 0.6; rot[k] = p.tilt * 0.3 * Math.sin(TAU * (t * p.speed + h3 + 0.25));
      }
      const out = new Float32Array(buf.length), o = new Float64Array(4);
      voroEach(W, H, nx, ny, px, py, (i, id, d1, d2, x, y) => {
        const dx = wd(x + 0.5 - px[id], W), dy = y + 0.5 - py[id], c = Math.cos(rot[id]), s = Math.sin(rot[id]);
        bilinear(buf, W, H, px[id] + dx * c - dy * s - ox[id], py[id] + dx * s + dy * c - oy[id], o);
        const cr = (1 - smoothstep(0.2, 1.2, (d2 - d1) * 0.5)) * p.crack, k = 1 - cr * 0.75;
        out[i] = o[0] * k; out[i + 1] = o[1] * k; out[i + 2] = o[2] * k; out[i + 3] = o[3];
      });
      return out;
    },
  });

  effect('flutedGlass', {
    name: 'Fluted glass', cat: 'Lens and warp', desc: 'Views the image through ribbed glass: every rib squeezes and flips a slice, and the ribs slide along.',
    tags: ['glass', 'ribbed', 'fluted', 'reeded', 'refraction', 'strips', 'bathroom', 'lens'],
    params: [P.int('ribs', 'Ribs', 4, 80, 28), P.range('strength', 'Refraction', 0, 1, 0.01, 0.3), P.range('shade', 'Rib shading', 0, 1, 0.01, 0.4), P.speed(1, 'speed', 'Ribs slide')],
    run(buf, W, H, t, p) {
      const rw = W / p.ribs, sh = t * p.speed, k = 1 - 2 * p.strength;
      const out = remap(buf, W, H, (x, y, s) => {
        const q = x / rw - sh, cx = (Math.floor(q) + 0.5 + sh) * rw;
        s[0] = cx + (x - cx) * k; s[1] = y;
      });
      return eachPx(out, W, H, (i, u, v, x) => {
        const pos = fract((x + 0.5) / rw - sh), g = 1 + p.shade * (0.5 - pos) * 0.7;
        out[i] = clamp(out[i] * g); out[i + 1] = clamp(out[i + 1] * g); out[i + 2] = clamp(out[i + 2] * g);
      });
    },
  });

  effect('waterReflect', {
    name: 'Water reflection', cat: 'Lens and warp', desc: 'Mirrors the top of the image into rippling, tinted water below a horizon line.',
    tags: ['water', 'reflection', 'lake', 'mirror', 'ripple', 'sea', 'blue', 'calm'],
    params: [P.range('horizon', 'Horizon', 0.2, 0.8, 0.01, 0.5), P.range('ripple', 'Ripple (px)', 0, 6, 0.1, 2), P.int('waves', 'Ripples', 2, 40, 14), P.range('fade', 'Fade', 0, 1, 0.01, 0.4), P.color('water', 'Water tint', '#2a6f9e'), P.range('tint', 'Tint', 0, 1, 0.01, 0.3), P.speed(1)],
    prepare: colorPrep(['water']),
    run(buf, W, H, t, p) {
      const hy = p.horizon * H, wt = p._water;
      const out = remap(buf, W, H, (x, y, s) => {
        if (y < hy) { s[0] = x; s[1] = y; return; }
        const d = (y - hy) / H, w = Math.sin(TAU * (d * p.waves - t * p.speed)) * (0.3 + d * 2);
        s[0] = x + p.ripple * w; s[1] = reflect(hy - (y - hy) + p.ripple * 0.3 * Math.cos(TAU * (x / W * 6 + t * p.speed)), H);
      });
      return eachPx(out, W, H, (i, u, v, x, y) => {
        if (y + 0.5 < hy) return;
        const d = (y + 0.5 - hy) / H, f = 1 - p.fade * Math.min(1, d * 2.2), w = Math.sin(TAU * (d * p.waves - t * p.speed)), glint = smoothstep(0.93, 1, w) * 0.12;
        for (let c = 0; c < 3; c++) out[i + c] = clamp(mix(out[i + c] * f, wt[c] * (0.5 + 0.5 * f), p.tint) + glint);
      });
    },
  });

  effect('melt', {
    name: 'Liquid melt', cat: 'Lens and warp', desc: 'Lets the image sag and drip downwards in ragged columns that rise and fall in a loop.',
    tags: ['melt', 'drip', 'liquid', 'sag', 'dissolve', 'slime', 'drool', 'goo'],
    params: [P.int('cols', 'Drips across', 3, 48, 16), P.range('length', 'Drip length (px)', 0, 40, 0.5, 14), P.range('wobble', 'Sideways wobble (px)', 0, 6, 0.1, 1.5), P.speed(1), P.seed(5)],
    run(buf, W, H, t, p) {
      const n = p.cols, amp = new Float32Array(n);
      for (let i = 0; i < n; i++) amp[i] = (0.35 + 0.65 * hash2(i, 1, p.seed)) * (0.5 + 0.5 * Math.sin(TAU * (t * p.speed + hash2(i, 2, p.seed))));
      return remap(buf, W, H, (x, y, s) => {
        const c = x / W * n - 0.5, i0 = Math.floor(c), a = mix(amp[wrap(i0, n)], amp[wrap(i0 + 1, n)], smoothstep(0, 1, c - i0)), v = y / H;
        s[0] = x + p.wobble * Math.sin(TAU * (v * 2 - t * p.speed)) * a; s[1] = y - p.length * a * smoothstep(0, 1, v);
      });
    },
  });

  // ======== Light ========
  effect('godRays', {
    name: 'God rays', cat: 'Light', desc: 'Bright areas throw soft shafts of light away from a sun that sways across the image.',
    tags: ['god rays', 'sun', 'light shafts', 'crepuscular', 'beams', 'volumetric', 'radiant', 'sunbeams'],
    params: [P.range('cx', 'Light x', 0, 1, 0.01, 0.5), P.range('cy', 'Light y', -0.5, 1.5, 0.01, 0.1), P.range('threshold', 'Threshold', 0, 1, 0.01, 0.5), P.range('length', 'Ray length', 0.1, 1, 0.01, 0.8), P.range('strength', 'Strength', 0, 4, 0.05, 1.6), P.color('tint', 'Ray colour', '#ffe9b0'), P.range('sweep', 'Sweep width', 0, 0.5, 0.01, 0.25), P.speed(1, 'speed', 'Sweeps')],
    prepare: colorPrep(['tint']),
    run(buf, W, H, t, p) {
      const N = W * H, B = new Float32Array(N * 3), tn = p._tint, STEPS = 8;
      for (let k = 0; k < N; k++) { const i = k * 4, m = smoothstep(p.threshold, 1, luma(buf[i], buf[i + 1], buf[i + 2])) * buf[i + 3]; B[k * 3] = buf[i] * m; B[k * 3 + 1] = buf[i + 1] * m; B[k * 3 + 2] = buf[i + 2] * m; }
      const lx = Math.round((p.cx + p.sweep * Math.sin(TAU * t * p.speed)) * W * 100) / 100 + 0.0137, ly = p.cy * H + 0.0211, out = new Float32Array(buf);
      return eachPx(out, W, H, (i, u, v, x, y) => {
        let s0 = 0, s1 = 0, s2 = 0;
        for (let k = 1; k <= STEPS; k++) {
          const f = (k / STEPS) * p.length, w = 1 - k / (STEPS + 1);
          const sx = Math.min(W - 1, Math.max(0, Math.floor(x + 0.5 + (lx - x - 0.5) * f))), sy = Math.min(H - 1, Math.max(0, Math.floor(y + 0.5 + (ly - y - 0.5) * f))), j = (sy * W + sx) * 3;
          s0 += B[j] * w; s1 += B[j + 1] * w; s2 += B[j + 2] * w;
        }
        const g = p.strength / STEPS * 2;
        addLight(out, i, s0 * g * (0.5 + tn[0] * 0.5), s1 * g * (0.5 + tn[1] * 0.5), s2 * g * (0.5 + tn[2] * 0.5));
      });
    },
  });

  effect('lensFlare', {
    name: 'Lens flare', cat: 'Light', desc: 'Bright pixels stretch into anamorphic streaks, and a flare with ghost rings passes over the image.',
    tags: ['lens flare', 'anamorphic', 'streak', 'sun', 'cinema', 'ghost', 'light', 'star'],
    params: [P.range('threshold', 'Streak threshold', 0, 1, 0.01, 0.75), P.int('streak', 'Streak length (px)', 2, 60, 22), P.range('strength', 'Streak strength', 0, 3, 0.05, 0.6), P.color('tint', 'Flare colour', '#7fb8ff'), P.range('ghosts', 'Ghosts', 0, 1, 0.01, 0.6), P.range('cy', 'Flare height', 0, 1, 0.01, 0.45), P.speed(1, 'speed', 'Flare passes')],
    prepare: colorPrep(['tint']),
    run(buf, W, H, t, p) {
      const N = W * H, B = new Float32Array(N * 3), tn = p._tint;
      for (let k = 0; k < N; k++) { const i = k * 4, m = smoothstep(p.threshold, 1, luma(buf[i], buf[i + 1], buf[i + 2])) * buf[i + 3]; B[k * 3] = buf[i] * m; B[k * 3 + 1] = buf[i + 1] * m; B[k * 3 + 2] = buf[i + 2] * m; }
      const S = hBlur3(B, W, H, p.streak), gain = p.strength * (0.5 + 0.1 * (2 * p.streak + 1));
      const cx = fract(0.1 + t * p.speed) * W, cy = p.cy * H, gf = [-0.45, 0.6, 1.35], gx = gf.map((f) => W / 2 + (W / 2 - cx) * f), gy = gf.map((f) => H / 2 + (H / 2 - cy) * f);
      const cr = H * 0.14, gr = H * 0.2, gw = H * 0.09;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const k = (y * W + x) * 3, dx = wd(x + 0.5 - cx, W), dy = y + 0.5 - cy;
        let f = Math.exp(-(dx * dx + dy * dy) / (2 * cr * cr)) + 0.6 * Math.exp(-Math.abs(dx) / (W * 0.1)) * Math.exp(-(dy * dy) / (2 * (H * 0.04) * (H * 0.04)));
        for (let g = 0; g < 3; g++) { const d = Math.hypot(wd(x + 0.5 - gx[g], W), y + 0.5 - gy[g]), e = (d - gr * (0.7 + g * 0.3)) / gw; f += 0.25 * p.ghosts * Math.exp(-e * e); }
        addLight(buf, i, S[k] * gain * (0.4 + tn[0]) + f * tn[0] * 0.8 + f * 0.2, S[k + 1] * gain * (0.4 + tn[1]) + f * tn[1] * 0.8 + f * 0.2, S[k + 2] * gain * (0.4 + tn[2]) + f * tn[2] * 0.8 + f * 0.2);
      });
    },
  });

  effect('glint', {
    name: 'Highlight glint', cat: 'Light', desc: 'A narrow diagonal shine sweeps across and only lights up the bright parts of the image.',
    tags: ['glint', 'shine', 'sweep', 'sparkle', 'highlight', 'metal', 'gleam', 'foil'],
    params: [P.range('angle', 'Angle', 0, 180, 1, 25), P.range('width', 'Width', 0.02, 0.6, 0.01, 0.1), P.range('threshold', 'Highlight threshold', 0, 1, 0.01, 0.35), P.range('strength', 'Strength', 0, 2, 0.01, 1.1), P.color('tint', 'Colour', '#fff6dd'), P.speed(1, 'speed', 'Sweeps')],
    prepare: colorPrep(['tint']),
    run(buf, W, H, t, p) {
      const a = (p.angle * Math.PI) / 180, ca = Math.cos(a), sa = Math.sin(a), span = W * Math.abs(ca) + H * Math.abs(sa) + 1e-3, tn = p._tint;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const d = fract(((x + 0.5) * ca + (y + 0.5) * sa) / span - t * p.speed + 0.5) - 0.5;
        const band = Math.exp(-(d * d) / (p.width * p.width * 0.18)) + 0.8 * Math.exp(-(d * d) / (p.width * p.width * 0.01));
        const k = band * smoothstep(p.threshold, p.threshold + 0.3, luma(buf[i], buf[i + 1], buf[i + 2])) * p.strength;
        addLight(buf, i, tn[0] * k * 0.9, tn[1] * k * 0.9, tn[2] * k * 0.9);
      });
    },
  });

  effect('iridescent', {
    name: 'Iridescent sheen', cat: 'Light', desc: 'Lays a soap-film rainbow sheen over the image that shifts with brightness and slides across.',
    tags: ['iridescent', 'holographic', 'oil slick', 'soap', 'pearl', 'rainbow', 'sheen', 'foil'],
    params: [P.range('freq', 'Film bands', 0.5, 6, 0.1, 2), P.range('spread', 'Spatial shift', 0, 3, 0.05, 1), P.range('amount', 'Amount', 0, 1, 0.01, 0.65), P.speed(1)],
    run(buf, W, H, t, p) {
      return eachPx(buf, W, H, (i, u, v) => {
        const l = luma(buf[i], buf[i + 1], buf[i + 2]), ph = l * p.freq + (u + v * 0.5) * p.spread - t * p.speed;
        const fr = 0.5 + 0.5 * Math.cos(TAU * ph), fg = 0.5 + 0.5 * Math.cos(TAU * (ph + 0.33)), fb = 0.5 + 0.5 * Math.cos(TAU * (ph + 0.67));
        buf[i] = clamp(mix(buf[i], pickOv(buf[i], fr), p.amount)); buf[i + 1] = clamp(mix(buf[i + 1], pickOv(buf[i + 1], fg), p.amount)); buf[i + 2] = clamp(mix(buf[i + 2], pickOv(buf[i + 2], fb), p.amount));
      });
    },
  });

  effect('spotlight', {
    name: 'Spotlight sweep', cat: 'Light', desc: 'A warm spotlight swings across the image while the rest falls into shadow.',
    tags: ['spotlight', 'stage', 'beam', 'dark', 'torch', 'searchlight', 'lamp', 'follow'],
    params: [P.range('size', 'Spot width', 0.1, 1, 0.01, 0.35), P.range('height', 'Spot height', 0.2, 3, 0.05, 1.4), P.range('ambient', 'Ambient light', 0, 1, 0.01, 0.25), P.range('boost', 'Brightness', 0, 2, 0.01, 0.5), P.range('soft', 'Softness', 0.05, 1, 0.01, 0.55), P.color('tint', 'Light colour', '#fff2d6'), P.speed(1, 'speed', 'Sweeps')],
    prepare: colorPrep(['tint']),
    run(buf, W, H, t, p) {
      const cx = W * (0.5 + 0.38 * Math.sin(TAU * t * p.speed)), rx = p.size * W * 0.5, ry = p.height * H * 0.5, tn = p._tint;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const dx = (x + 0.5 - cx) / rx, dy = (y + 0.5 - H / 2) / ry, g = 1 - smoothstep(1 - p.soft, 1, Math.sqrt(dx * dx + dy * dy)), li = p.ambient + (1 - p.ambient) * g;
        for (let c = 0; c < 3; c++) buf[i + c] = clamp(buf[i + c] * li + p.boost * g * tn[c] * (0.2 + buf[i + c] * 0.8));
      });
    },
  });

  effect('bokeh', {
    name: 'Bokeh highlights', cat: 'Light', desc: 'Turns bright spots into soft glowing discs that float gently, like out-of-focus lights.',
    tags: ['bokeh', 'out of focus', 'discs', 'lights', 'dreamy', 'blur', 'night', 'glow'],
    params: [P.range('threshold', 'Threshold', 0, 1, 0.01, 0.5), P.int('radius', 'Disc size (px)', 2, 16, 7), P.range('strength', 'Strength', 0, 3, 0.05, 1.3), P.range('rim', 'Bright rim', 0, 1, 0.01, 0.55), P.range('drift', 'Float (px)', 0, 12, 0.1, 3), P.speed(1, 'speed', 'Float cycles')],
    run(buf, W, H, t, p) {
      const N = W * H, B = new Float32Array(N * 3);
      for (let k = 0; k < N; k++) { const i = k * 4, m = smoothstep(p.threshold, 1, luma(buf[i], buf[i + 1], buf[i + 2])) * buf[i + 3]; B[k * 3] = buf[i] * m; B[k * 3 + 1] = buf[i + 1] * m; B[k * 3 + 2] = buf[i + 2] * m; }
      const R = p.radius, ring = [], inner = [], ox = p.drift * Math.cos(TAU * t * p.speed), oy = p.drift * Math.sin(TAU * t * p.speed) * 0.5;
      for (let k = 0; k < 12; k++) ring.push([Math.round(Math.cos(TAU * k / 12) * R), Math.round(Math.sin(TAU * k / 12) * R)]);
      for (let k = 0; k < 6; k++) inner.push([Math.round(Math.cos(TAU * k / 6) * R * 0.5), Math.round(Math.sin(TAU * k / 6) * R * 0.5)]);
      inner.push([0, 0]);
      const sx = Math.round(ox), sy = Math.round(oy), out = new Float32Array(buf);
      const tap = (x, y, acc) => { const j = (Math.min(H - 1, Math.max(0, y)) * W + wrap(x, W)) * 3; acc[0] += B[j]; acc[1] += B[j + 1]; acc[2] += B[j + 2]; };
      const ra = [0, 0, 0], ia = [0, 0, 0];
      return eachPx(out, W, H, (i, u, v, x, y) => {
        ra[0] = ra[1] = ra[2] = ia[0] = ia[1] = ia[2] = 0;
        for (let k = 0; k < 12; k++) tap(x + sx + ring[k][0], y + sy + ring[k][1], ra);
        for (let k = 0; k < 7; k++) tap(x + sx + inner[k][0], y + sy + inner[k][1], ia);
        const g = p.strength;
        addLight(out, i, (ia[0] / 7 * (1 - p.rim * 0.5) + ra[0] / 12 * p.rim) * g, (ia[1] / 7 * (1 - p.rim * 0.5) + ra[1] / 12 * p.rim) * g, (ia[2] / 7 * (1 - p.rim * 0.5) + ra[2] / 12 * p.rim) * g);
      });
    },
  });

  effect('sparkle', {
    name: 'Highlight sparkle', cat: 'Light', desc: 'Four-point stars twinkle on the brightest parts of the image, each on its own timing.',
    tags: ['sparkle', 'twinkle', 'stars', 'glitter', 'shine', 'bling', 'diamond', 'magic'],
    params: [P.range('threshold', 'Highlight threshold', 0, 1, 0.01, 0.4), P.int('cell', 'Cell size (px)', 4, 32, 9), P.range('density', 'Density', 0, 1, 0.01, 0.8), P.range('strength', 'Strength', 0, 2, 0.01, 1.2), P.color('tint', 'Star colour', '#ffffff'), P.speed(1, 'speed', 'Twinkles'), P.seed(21)],
    prepare: colorPrep(['tint']),
    run(buf, W, H, t, p) {
      const L = lumArr(buf, W, H), nx = Math.ceil(W / p.cell), ny = Math.ceil(H / p.cell), R = Math.max(3, Math.round(p.cell * 0.9)), tn = p._tint;
      const stars = [];
      for (let j = 0; j < ny; j++) for (let i = 0; i < nx; i++) {
        if (hash2(i, j, p.seed + 2) > p.density) continue;
        const sx = Math.min(W - 1, Math.floor((i + hash2(i, j, p.seed)) * p.cell)), sy = Math.min(H - 1, Math.floor((j + hash2(i, j, p.seed + 1)) * p.cell));
        const tw = Math.pow(Math.max(0, Math.sin(TAU * (t * p.speed + hash2(i, j, p.seed + 3)))), 3), amp = smoothstep(p.threshold - 0.1, p.threshold + 0.25, L[sy * W + sx]) * tw * p.strength;
        if (amp > 0.01) stars.push([sx, sy, amp]);
      }
      for (const [sx, sy, amp] of stars) {
        for (let dy = -R; dy <= R; dy++) {
          const y = sy + dy; if (y < 0 || y >= H) continue;
          for (let dx = -R; dx <= R; dx++) {
            const ax = Math.abs(dx), ay = Math.abs(dy), arm = Math.max(Math.exp(-ax / (R * 0.4)) * Math.exp(-ay * 1.7), Math.exp(-ay / (R * 0.4)) * Math.exp(-ax * 1.7)), core = Math.exp(-(ax * ax + ay * ay) / 2);
            const k = (arm + core) * amp; if (k < 0.01) continue;
            addLight(buf, (y * W + wrap(sx + dx, W)) * 4, tn[0] * k, tn[1] * k, tn[2] * k);
          }
        }
      }
      return buf;
    },
  });

  effect('strobe', {
    name: 'Strobe flash', cat: 'Light', desc: 'Bursts of white light wash out the image and fade away, wiping across from the left.',
    tags: ['strobe', 'flash', 'camera', 'lightning', 'flicker', 'burst', 'blink', 'photo'],
    params: [P.range('strength', 'Flash', 0, 1, 0.01, 0.7), P.color('tint', 'Flash colour', '#ffffff'), P.range('decay', 'Decay', 1, 12, 0.1, 5), P.range('bleach', 'Washout', 0, 1, 0.01, 0.5), P.range('dim', 'Dim between flashes', 0, 0.8, 0.01, 0.3), P.range('wipe', 'Wipe across', 0, 1, 0.01, 0.25), P.speed(2, 'speed', 'Flashes')],
    prepare: colorPrep(['tint']),
    run(buf, W, H, t, p) {
      const tn = p._tint;
      return eachPx(buf, W, H, (i, u) => {
        const f = Math.exp(-p.decay * fract(t * p.speed - u * p.wipe)), dm = 1 - p.dim * (1 - f), bl = 1 + f * p.bleach * 2;
        for (let c = 0; c < 3; c++) buf[i + c] = clamp(mix(buf[i + c] * dm * bl, tn[c], f * p.strength));
      });
    },
  });

  effect('rainbowSweep', {
    name: 'Rainbow sweep', cat: 'Light', desc: 'A diagonal band of spectral colour glides across the image like light through a prism.',
    tags: ['rainbow', 'prism', 'spectrum', 'sweep', 'holo', 'light leak', 'refraction', 'colourful'],
    params: [P.range('angle', 'Angle', 0, 180, 1, 20), P.range('width', 'Width', 0.05, 1, 0.01, 0.4), P.range('strength', 'Strength', 0, 1.5, 0.01, 0.8), P.speed(1, 'speed', 'Sweeps')],
    run(buf, W, H, t, p) {
      const a = (p.angle * Math.PI) / 180, ca = Math.cos(a), sa = Math.sin(a), span = W * Math.abs(ca) + H * Math.abs(sa) + 1e-3;
      return eachPx(buf, W, H, (i, u, v, x, y) => {
        const d = fract(((x + 0.5) * ca + (y + 0.5) * sa) / span - t * p.speed + 0.5) - 0.5, ad = Math.abs(d);
        if (ad > p.width * 0.5) return;
        const env = smoothstep(p.width * 0.5, p.width * 0.15, ad) * p.strength, h = d / p.width + 0.5;
        const r = (0.5 + 0.5 * Math.cos(TAU * h)) * env, g = (0.5 + 0.5 * Math.cos(TAU * (h + 0.33))) * env, b = (0.5 + 0.5 * Math.cos(TAU * (h + 0.67))) * env;
        buf[i] = clamp(1 - (1 - buf[i]) * (1 - r)); buf[i + 1] = clamp(1 - (1 - buf[i + 1]) * (1 - g)); buf[i + 2] = clamp(1 - (1 - buf[i + 2]) * (1 - b));
      });
    },
  });

}
