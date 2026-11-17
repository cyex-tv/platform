// @ts-nocheck
/* Shared per-pixel helpers for layers and effects (loads right after core.js; uses only core.js exports). */
export default function install(PL) {
  'use strict';
  const { wrap, hexToRgb } = PL;

  const rgb01 = (hex) => hexToRgb(hex).map((x) => x / 255);
  // prepare() helper: stores p['_' + key] = [r, g, b] (0..1) for every colour key.
  const colorPrep = (keys) => (p) => { for (const k of keys) p['_' + k] = rgb01(p[k]); };
  // Visit every pixel of buf: fn(i, u, v, x, y) with i the buffer index and u, v in 0..1 at the pixel centre.
  const eachPx = (buf, W, H, fn) => { for (let y = 0, i = 0; y < H; y++) for (let x = 0; x < W; x++, i += 4) fn(i, (x + 0.5) / W, (y + 0.5) / H, x, y); return buf; };

  // Bilinear sample at pixel coordinates (x, y) (pixel centres at +0.5). Edges wrap, or clamp when wrapEdges is false.
  function bilinear(buf, W, H, x, y, out, wrapEdges = true) {
    x -= 0.5; y -= 0.5;
    const x0 = Math.floor(x), y0 = Math.floor(y), fx = x - x0, fy = y - y0;
    let xa, xb, ya, yb;
    if (wrapEdges) {
      xa = ((x0 % W) + W) % W; xb = (((x0 + 1) % W) + W) % W;
      ya = ((y0 % H) + H) % H; yb = (((y0 + 1) % H) + H) % H;
    } else {
      xa = Math.max(0, Math.min(W - 1, x0)); xb = Math.max(0, Math.min(W - 1, x0 + 1));
      ya = Math.max(0, Math.min(H - 1, y0)); yb = Math.max(0, Math.min(H - 1, y0 + 1));
    }
    const a = (ya * W + xa) * 4, b = (ya * W + xb) * 4, c = (yb * W + xa) * 4, d = (yb * W + xb) * 4;
    for (let k = 0; k < 4; k++) {
      const m1 = buf[a + k] + (buf[b + k] - buf[a + k]) * fx, m2 = buf[c + k] + (buf[d + k] - buf[c + k]) * fx;
      out[k] = m1 + (m2 - m1) * fy;
    }
    return out;
  }

  // Warp the image: fn(x, y, out2) writes the source position (pixels) for the pixel centre (x, y) into out2[0], out2[1].
  function remap(buf, W, H, fn) {
    const out = new Float32Array(buf.length), o = new Float64Array(4), s = new Float64Array(2);
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
      fn(x + 0.5, y + 0.5, s);
      bilinear(buf, W, H, s[0], s[1], o);
      const i = (y * W + x) * 4; out[i] = o[0]; out[i + 1] = o[1]; out[i + 2] = o[2]; out[i + 3] = o[3];
    }
    return out;
  }

  // Two-pass box blur with sliding-window sums (cost independent of the radius). Premultiplied so transparent
  // pixels don't darken edges; wraps horizontally, clamps vertically. Sums below EPS count as empty.
  const EPS = 1e-12;
  function boxBlur(buf, W, H, r) {
    if (r < 1) return buf;
    r = Math.round(r);
    let src = buf;
    const tmp = new Float32Array(buf.length), acc = new Float64Array(W * 4), stride = W * 4;
    for (let pass = 0; pass < 2; pass++) {
      const n = 2 * r + 1;
      // horizontal
      for (let y = 0; y < H; y++) {
        const row = y * W;
        let s0 = 0, s1 = 0, s2 = 0, s3 = 0;
        for (let k = -r; k <= r; k++) { const i = (row + wrap(k, W)) * 4, a = src[i + 3]; s0 += src[i] * a; s1 += src[i + 1] * a; s2 += src[i + 2] * a; s3 += a; }
        for (let x = 0; x < W; x++) {
          const j = (row + x) * 4;
          if (s3 > EPS) { tmp[j] = s0 / n; tmp[j + 1] = s1 / n; tmp[j + 2] = s2 / n; tmp[j + 3] = s3 / n; }
          else { tmp[j] = 0; tmp[j + 1] = 0; tmp[j + 2] = 0; tmp[j + 3] = 0; }
          const ia = (row + wrap(x + r + 1, W)) * 4, aa = src[ia + 3], ib = (row + wrap(x - r, W)) * 4, ab = src[ib + 3];
          s0 += src[ia] * aa - src[ib] * ab; s1 += src[ia + 1] * aa - src[ib + 1] * ab; s2 += src[ia + 2] * aa - src[ib + 2] * ab; s3 += aa - ab;
        }
      }
      // vertical (rows clamp at the top and bottom), then un-premultiply
      const out = new Float32Array(buf.length);
      acc.fill(0);
      for (let k = -r; k <= r; k++) { const base = Math.max(0, Math.min(H - 1, k)) * stride; for (let q = 0; q < stride; q++) acc[q] += tmp[base + q]; }
      for (let y = 0; y < H; y++) {
        const base = y * stride;
        for (let q = 0; q < stride; q += 4) {
          const s3 = acc[q + 3];
          if (s3 > EPS) { const a = s3 / n, j = base + q; out[j] = acc[q] / n / a; out[j + 1] = acc[q + 1] / n / a; out[j + 2] = acc[q + 2] / n / a; out[j + 3] = a; }
        }
        const ba = Math.max(0, Math.min(H - 1, y + r + 1)) * stride, bb = Math.max(0, Math.min(H - 1, y - r)) * stride;
        for (let q = 0; q < stride; q++) acc[q] += tmp[ba + q] - tmp[bb + q];
      }
      src = out;
      r = Math.max(1, Math.round(r * 0.6));
    }
    return src;
  }

  // 3x3 hue-rotation matrix (row-major) for deg degrees.
  function hueMatrix(deg) {
    const a = (deg * Math.PI) / 180, c = Math.cos(a), s = Math.sin(a);
    return [0.213 + c * 0.787 - s * 0.213, 0.715 - c * 0.715 - s * 0.715, 0.072 - c * 0.072 + s * 0.928,
      0.213 - c * 0.213 + s * 0.143, 0.715 + c * 0.285 + s * 0.14, 0.072 - c * 0.072 - s * 0.283,
      0.213 - c * 0.213 - s * 0.787, 0.715 - c * 0.715 + s * 0.715, 0.072 + c * 0.928 + s * 0.072];
  }

  PL.fx = { rgb01, colorPrep, eachPx, bilinear, remap, boxBlur, hueMatrix };
}
