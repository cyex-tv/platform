// @ts-nocheck
/* Shared helpers: maths, colour packing, gradient lookup tables, noise. */
export default function install(PL) {
  'use strict';
  const TAU = Math.PI * 2;
  const clamp = (x, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
  const fract = (x) => x - Math.floor(x);
  const mix = (a, b, t) => a + (b - a) * t;
  const smoothstep = (a, b, x) => { const t = clamp((x - a) / (b - a)); return t * t * (3 - 2 * t); };
  const tri = (x) => 1 - Math.abs(fract(x) * 2 - 1); // 0..1..0 triangle wave
  const wrap = (i, n) => ((i % n) + n) % n;

  // ---- colour ----
  function hexToRgb(h) {
    h = String(h || '#000000').replace('#', '').trim();
    if (h.length === 3 || h.length === 4) h = h.split('').map((c) => c + c).join('');
    const n = parseInt(h.slice(0, 6), 16) || 0;
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  const hex2 = (x) => Math.round(clamp(x, 0, 255)).toString(16).padStart(2, '0');
  const rgbToHex = (r, g, b) => '#' + hex2(r) + hex2(g) + hex2(b);

  // A colour packed into one signed 32-bit int, red in the top byte and alpha in the low one
  // (the integer colour format some imported paint JSON uses).
  const packRGBA = (r, g, b, a) => ((r & 255) << 24) | ((g & 255) << 16) | ((b & 255) << 8) | (a & 255);
  const unpackRGBA = (n) => [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255];
  const hexAlphaToDec = (hex, alpha) => { const [r, g, b] = hexToRgb(hex); return packRGBA(r, g, b, Math.round(clamp(alpha) * 255)); };
  const decToHexAlpha = (n) => { const [r, g, b, a] = unpackRGBA(n); return { hex: rgbToHex(r, g, b), alpha: a / 255 }; };

  function hsvToRgb(h, s, v) { // h 0..1, returns 0..1
    h = fract(h) * 6; const i = Math.floor(h), f = h - i;
    const p = v * (1 - s), q = v * (1 - s * f), t = v * (1 - s * (1 - f));
    switch (i % 6) { case 0: return [v, t, p]; case 1: return [q, v, p]; case 2: return [p, v, t]; case 3: return [p, q, v]; case 4: return [t, p, v]; default: return [v, p, q]; }
  }
  function rgbToHsl(r, g, b) {
    const mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2;
    if (mx === mn) return [0, 0, l];
    const d = mx - mn, s = l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn);
    let h = mx === r ? (g - b) / d + (g < b ? 6 : 0) : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return [h / 6, s, l];
  }
  function hslToRgb(h, s, l) {
    if (s === 0) return [l, l, l];
    const hue = (p, q, t) => { t = fract(t); if (t < 1 / 6) return p + (q - p) * 6 * t; if (t < 1 / 2) return q; if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6; return p; };
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q;
    return [hue(p, q, h + 1 / 3), hue(p, q, h), hue(p, q, h - 1 / 3)];
  }
  const luma = (r, g, b) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

  // ---- gradient stops ----
  // Editor stops: [{ at: 0..1, color: '#rrggbb', a: 0..1 }]. CSS semantics: positions are clamped so they
  // never go backwards, colours interpolate premultiplied, ends hold their colour.
  function normStops(stops) {
    const s = (stops || []).map((x) => ({ at: clamp(+x.at || 0), rgb: hexToRgb(x.color), a: x.a == null ? 1 : clamp(+x.a) }));
    if (!s.length) s.push({ at: 0, rgb: [0, 0, 0], a: 0 });
    let mx = 0;
    for (const x of s) { if (x.at < mx) x.at = mx; mx = x.at; }
    return s;
  }
  // Sample normalised stops at t (no repeat). out: [r,g,b,a] 0..1, straight alpha.
  function sampleStops(s, t, out) {
    let i = 0;
    if (t <= s[0].at) return setStop(s[0], out);
    if (t >= s[s.length - 1].at) return setStop(s[s.length - 1], out);
    while (i < s.length - 1 && s[i + 1].at < t) i++;
    const A = s[i], B = s[i + 1];
    const span = B.at - A.at, f = span <= 0 ? 1 : (t - A.at) / span;
    const a = mix(A.a, B.a, f);
    if (a <= 0) { out[0] = out[1] = out[2] = out[3] = 0; return out; }
    for (let c = 0; c < 3; c++) out[c] = mix((A.rgb[c] / 255) * A.a, (B.rgb[c] / 255) * B.a, f) / a;
    out[3] = a;
    return out;
  }
  function setStop(x, out) { out[0] = x.rgb[0] / 255; out[1] = x.rgb[1] / 255; out[2] = x.rgb[2] / 255; out[3] = x.a; return out; }

  // 512-entry lookup table, used by every gradient-mapped layer.
  const LUT_N = 512;
  function buildLut(stops) {
    const s = normStops(stops), lut = new Float32Array(LUT_N * 4), o = [0, 0, 0, 0];
    for (let i = 0; i < LUT_N; i++) {
      sampleStops(s, i / (LUT_N - 1), o);
      lut[i * 4] = o[0]; lut[i * 4 + 1] = o[1]; lut[i * 4 + 2] = o[2]; lut[i * 4 + 3] = o[3];
    }
    return lut;
  }
  function lutAt(lut, f, out) {
    const i = (clamp(f) * (LUT_N - 1)) | 0, j = i * 4;
    out[0] = lut[j]; out[1] = lut[j + 1]; out[2] = lut[j + 2]; out[3] = lut[j + 3];
    return out;
  }

  // ---- weighted palettes ----
  // A palette is { colors: [{ c: '#hex', a: 0..1, w: weight }], blend: 0..1, loop: bool }.
  // Each colour gets a share of the ramp proportional to its weight ("strength"); blend 0 gives hard
  // bands, 1 smooth gradients; loop makes the end flow back into the start (for cycling effects).
  function mixHex(a, b, f) { const x = hexToRgb(a), y = hexToRgb(b); return rgbToHex(mix(x[0], y[0], f), mix(x[1], y[1], f), mix(x[2], y[2], f)); }
  function paletteToStops(pal) {
    const cols = ((pal && pal.colors) || []).filter((c) => c.w > 0);
    if (!cols.length) return [{ at: 0, color: '#000000', a: 0 }];
    if (cols.length === 1) return [{ at: 0, color: cols[0].c, a: cols[0].a ?? 1 }];
    const b = clamp(pal.blend ?? 1), tot = cols.reduce((s, c) => s + c.w, 0);
    let s = 0;
    const holds = cols.map((c) => { const w = c.w / tot, ctr = s + w / 2, hw = ((1 - b) * w) / 2; s += w; return { c, a0: ctr - hw, a1: ctr + hw }; });
    const st = [];
    let edge;
    if (pal.loop) {
      const L = holds[holds.length - 1], F = holds[0];
      const g0 = L.a1 - 1, g1 = F.a0, f = (0 - g0) / Math.max(1e-6, g1 - g0);
      edge = { color: mixHex(L.c.c, F.c.c, f), a: mix(L.c.a ?? 1, F.c.a ?? 1, f) };
      st.push({ at: 0, color: edge.color, a: edge.a });
    } else st.push({ at: 0, color: cols[0].c, a: cols[0].a ?? 1 });
    for (const hd of holds) { st.push({ at: clamp(hd.a0), color: hd.c.c, a: hd.c.a ?? 1 }); st.push({ at: clamp(hd.a1), color: hd.c.c, a: hd.c.a ?? 1 }); }
    if (pal.loop) st.push({ at: 1, color: edge.color, a: edge.a });
    else { const l = cols[cols.length - 1]; st.push({ at: 1, color: l.c, a: l.a ?? 1 }); }
    return st;
  }
  function stopsToPalette(stops, loop = false) {
    if (stops && stops.colors) return clone(stops);
    const s = [...(stops || [])].sort((a, b) => a.at - b.at);
    const colors = s.map((x, i) => {
      const prev = i ? (x.at + s[i - 1].at) / 2 : 0, next = i < s.length - 1 ? (x.at + s[i + 1].at) / 2 : 1;
      return { c: x.color, a: x.a ?? 1, w: Math.max(0.05, Math.round((next - prev) * 100) / 100) };
    });
    return { colors, blend: 1, loop };
  }
  function pickWeighted(pal, r) {
    const cols = ((pal && pal.colors) || []).filter((c) => c.w > 0);
    if (!cols.length) return { c: '#ffffff', a: 1 };
    let tot = 0; for (const c of cols) tot += c.w;
    let x = r * tot;
    for (const c of cols) { x -= c.w; if (x <= 0) return c; }
    return cols[cols.length - 1];
  }
  const rgbaStr = (hex, a = 1) => { const [r, g, b] = hexToRgb(hex); return `rgba(${r},${g},${b},${a})`; };

  // ---- noise (value noise on an integer lattice that tiles with the given period) ----
  function hash2(x, y, seed) {
    let h = (x * 374761393 + y * 668265263 + seed * 1442695041) | 0;
    h = Math.imul(h ^ (h >>> 13), 1274126177);
    h ^= h >>> 16;
    return (h >>> 0) / 4294967296;
  }
  const quint = (t) => t * t * t * (t * (t * 6 - 15) + 10);
  function vnoise(x, y, px, py, seed) {
    const ix = Math.floor(x), iy = Math.floor(y), fx = x - ix, fy = y - iy;
    const x0 = wrap(ix, px), x1 = wrap(ix + 1, px), y0 = wrap(iy, py), y1 = wrap(iy + 1, py);
    const a = hash2(x0, y0, seed), b = hash2(x1, y0, seed), c = hash2(x0, y1, seed), d = hash2(x1, y1, seed);
    const u = quint(fx), v = quint(fy);
    return mix(mix(a, b, u), mix(c, d, u), v);
  }
  // Fractal noise; periods double each octave so the result still tiles. Returns ~0..1.
  function fbm(x, y, px, py, oct, seed) {
    let sum = 0, amp = 0.5, norm = 0;
    for (let o = 0; o < oct; o++) {
      sum += amp * vnoise(x, y, px, py, seed + o * 17);
      norm += amp; amp *= 0.5; x *= 2; y *= 2; px *= 2; py *= 2;
    }
    return sum / norm;
  }
  function ridged(x, y, px, py, oct, seed) {
    let sum = 0, amp = 0.5, norm = 0;
    for (let o = 0; o < oct; o++) {
      const n = 1 - Math.abs(vnoise(x, y, px, py, seed + o * 31) * 2 - 1);
      sum += amp * n * n; norm += amp; amp *= 0.5; x *= 2; y *= 2; px *= 2; py *= 2;
    }
    return sum / norm;
  }

  // Seeded RNG for presets and ids.
  function rng(seed) { let s = seed >>> 0 || 1; return () => { s = Math.imul(s ^ (s >>> 15), 2246822519) + 0x9e3779b9 | 0; s ^= s >>> 13; return (s >>> 0) / 4294967296; }; }
  let idc = 0;
  const uid = (p = 'id') => p + '_' + Date.now().toString(36) + '_' + (idc++).toString(36);
  // A drawing canvas that works on the page and inside a Web Worker (OffscreenCanvas), so the
  // renderer can run in either.
  const makeCanvas = (w, h) => { if (typeof document !== 'undefined') { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; } return new OffscreenCanvas(w, h); };
  const clone = (x) => JSON.parse(JSON.stringify(x));

  // cyex number formats: 1.000.000 and 1,5 MB
  const fmtInt = (n) => String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  function fmtBytes(b) {
    if (b < 1024) return fmtInt(b) + ' B';
    const u = ['KB', 'MB', 'GB']; let i = -1;
    do { b /= 1024; i++; } while (b >= 1024 && i < u.length - 1);
    return (b < 10 ? b.toFixed(1) : Math.round(b).toString()).replace('.', ',') + ' ' + u[i];
  }
  const fmtNum = (n, d = 0) => (+n).toFixed(d).replace('.', ',');

  PL.imageCache = PL.imageCache || {}; // decoded pictures for image and picture layers (filled by the app)
  Object.assign(PL, {
    TAU, clamp, fract, mix, smoothstep, tri, wrap,
    hexToRgb, rgbToHex, packRGBA, unpackRGBA, hexAlphaToDec, decToHexAlpha,
    hsvToRgb, rgbToHsl, hslToRgb, luma,
    normStops, sampleStops, buildLut, lutAt, LUT_N, mixHex, paletteToStops, stopsToPalette, pickWeighted, rgbaStr,
    hash2, vnoise, fbm, ridged, rng, uid, makeCanvas, clone, fmtInt, fmtBytes, fmtNum,
  });
}
