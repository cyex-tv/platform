// @ts-nocheck
/* Local encoders, no libraries: animated GIF (median-cut palette + LZW, frame differencing), animated WebP (browser stills, muxed here) and APNG (zlib via the
   browser's built-in CompressionStream). Frames are ImageData objects of equal size. */
export default function install(PL) {
  'use strict';

  class ByteOut {
    constructor() { this.buf = new Uint8Array(1 << 16); this.n = 0; }
    grow(k) { if (this.n + k > this.buf.length) { let s = this.buf.length; while (s < this.n + k) s *= 2; const b = new Uint8Array(s); b.set(this.buf.subarray(0, this.n)); this.buf = b; } }
    byte(b) { this.grow(1); this.buf[this.n++] = b; }
    bytes(a) { this.grow(a.length); this.buf.set(a, this.n); this.n += a.length; }
    u16(v) { this.byte(v & 255); this.byte((v >> 8) & 255); }
    u24(v) { this.byte(v & 255); this.byte((v >> 8) & 255); this.byte((v >> 16) & 255); }
    u32(v) { this.byte(v & 255); this.byte((v >>> 8) & 255); this.byte((v >>> 16) & 255); this.byte((v >>> 24) & 255); }
    str(s) { for (let i = 0; i < s.length; i++) this.byte(s.charCodeAt(i)); }
    result() { return this.buf.slice(0, this.n); }
  }

  // ---------- palette ----------
  const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5].map((v) => (v / 16 - 0.5));

  // Median cut over a 15-bit histogram of every frame's opaque pixels.
  function buildPalette(frames, maxColors, alphaCut) {
    const hist = new Uint32Array(32768);
    for (const f of frames) {
      const d = f.data;
      for (let i = 0; i < d.length; i += 4) {
        if (d[i + 3] < alphaCut) continue;
        hist[((d[i] >> 3) << 10) | ((d[i + 1] >> 3) << 5) | (d[i + 2] >> 3)]++;
      }
    }
    const cols = [];
    for (let k = 0; k < 32768; k++) if (hist[k]) cols.push(k);
    if (!cols.length) return [[0, 0, 0]];
    const comp = (k, c) => (c === 0 ? (k >> 10) & 31 : c === 1 ? (k >> 5) & 31 : k & 31);
    let boxes = [{ cols, count: cols.reduce((a, k) => a + hist[k], 0) }];
    const rangeOf = (b) => {
      let best = 0, bc = 0;
      for (let c = 0; c < 3; c++) {
        let mn = 31, mx = 0;
        for (const k of b.cols) { const v = comp(k, c); if (v < mn) mn = v; if (v > mx) mx = v; }
        if (mx - mn > best) { best = mx - mn; bc = c; }
      }
      return [best, bc];
    };
    while (boxes.length < maxColors) {
      let pick = -1, score = 0, pc = 0;
      boxes.forEach((b, i) => {
        if (b.cols.length < 2) return;
        const [r, c] = rangeOf(b);
        const s = r * Math.sqrt(b.count);
        if (s > score) { score = s; pick = i; pc = c; }
      });
      if (pick < 0) break;
      const b = boxes[pick];
      b.cols.sort((x, y) => comp(x, pc) - comp(y, pc));
      let acc = 0, half = b.count / 2, cut = 1;
      for (let i = 0; i < b.cols.length - 1; i++) { acc += hist[b.cols[i]]; if (acc >= half) { cut = i + 1; break; } cut = i + 1; }
      const A = b.cols.slice(0, cut), B = b.cols.slice(cut);
      const cnt = (l) => l.reduce((a, k) => a + hist[k], 0);
      boxes.splice(pick, 1, { cols: A, count: cnt(A) }, { cols: B, count: cnt(B) });
    }
    return boxes.map((b) => {
      let r = 0, g = 0, bl = 0, n = 0;
      for (const k of b.cols) { const w = hist[k]; r += (((k >> 10) & 31) * 8 + 4) * w; g += (((k >> 5) & 31) * 8 + 4) * w; bl += ((k & 31) * 8 + 4) * w; n += w; }
      return [Math.round(r / n), Math.round(g / n), Math.round(bl / n)];
    });
  }

  function makeMapper(pal, offset) {
    const cache = new Int16Array(32768).fill(-1);
    return (r, g, b) => {
      const k = ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3);
      let v = cache[k];
      if (v >= 0) return v;
      let best = 1e9;
      for (let i = 0; i < pal.length; i++) {
        const p = pal[i], dr = p[0] - r, dg = p[1] - g, db = p[2] - b;
        const dd = dr * dr * 2 + dg * dg * 4 + db * db * 3;
        if (dd < best) { best = dd; v = i; }
      }
      cache[k] = v + offset;
      return v + offset;
    };
  }

  // ---------- LZW (variable-width codes, clear at 4096, 255-byte sub-blocks) ----------
  function lzw(out, minCode, idx) {
    out.byte(minCode);
    const clear = 1 << minCode, eoi = clear + 1;
    let codeSize = minCode + 1, next = eoi + 1;
    let table = new Map();
    let cur = 0, shift = 0;
    const block = new Uint8Array(255); let bn = 0;
    const flushByte = (b) => { block[bn++] = b; if (bn === 255) { out.byte(255); out.bytes(block); bn = 0; } };
    const emit = (code) => { cur |= code << shift; shift += codeSize; while (shift >= 8) { flushByte(cur & 255); cur >>>= 8; shift -= 8; } };
    emit(clear);
    let ib = idx[0];
    for (let i = 1; i < idx.length; i++) {
      const k = idx[i], key = (ib << 8) | k;
      const c = table.get(key);
      if (c === undefined) {
        emit(ib);
        if (next === 4096) { emit(clear); next = eoi + 1; codeSize = minCode + 1; table = new Map(); }
        else { if (next >= 1 << codeSize) codeSize++; table.set(key, next++); }
        ib = k;
      } else ib = c;
    }
    emit(ib); emit(eoi);
    if (shift > 0) flushByte(cur & 255);
    if (bn) { out.byte(bn); out.bytes(block.subarray(0, bn)); }
    out.byte(0);
  }

  /* opts: { delayCs, colors (2..256), dither (bool), alphaCut (0..255), matte: [r,g,b] | null, loop,
             diff (default true): store only pixels that changed since the previous frame,
             localPalettes (default false): one median-cut palette per frame } */
  function encodeGif(frames, opts = {}) {
    // Differencing only pays off when much of the paint stands still; for paints that move everywhere it
    // can come out larger. Encoding is cheap next to rendering, so try both and keep the smaller file.
    if (opts.diff !== false && opts.diff !== 'force' && frames.length > 1 && !opts._inner) {
      const a = encodeGif(frames, Object.assign({}, opts, { diff: 'force', _inner: true })), b = encodeGif(frames, Object.assign({}, opts, { diff: false, _inner: true }));
      return a.size <= b.size ? a : b;
    }
    const W = frames[0].width, H = frames[0].height, N = W * H;
    const alphaCut = opts.alphaCut ?? 128;
    // With a matte, translucent pixels are blended onto it; fully transparent ones stay transparent.
    const src = frames.map((f) => {
      if (!opts.matte) return f;
      const o = new ImageData(W, H), d = f.data, e = o.data, m = opts.matte;
      for (let i = 0; i < d.length; i += 4) {
        const a = d[i + 3] / 255;
        e[i] = d[i] * a + m[0] * (1 - a); e[i + 1] = d[i + 1] * a + m[1] * (1 - a); e[i + 2] = d[i + 2] * a + m[2] * (1 - a);
        e[i + 3] = opts.keepAlpha && d[i + 3] < alphaCut ? 0 : 255;
      }
      return o;
    });
    const n = src.length;
    const diff = opts.diff !== false && n > 1;
    const local = !!opts.localPalettes;
    const hasAlpha = src.some((f) => { const d = f.data; for (let i = 3; i < d.length; i += 4) if (d[i] < alphaCut) return true; return false; });
    // Index 0 is the transparent colour whenever the paint has transparency or frames are differenced.
    const useTrans = hasAlpha || diff;
    const offset = useTrans ? 1 : 0;
    const maxC = Math.max(2, Math.min(256, opts.colors || 256)) - offset;
    const tableBits = (len) => { let b = 1; while ((1 << b) < len + offset) b++; return b; };
    const putTable = (pal, bits) => {
      for (let i = 0; i < (1 << bits); i++) {
        const c = i < offset ? [0, 0, 0] : pal[i - offset] || [0, 0, 0];
        out.byte(c[0]); out.byte(c[1]); out.byte(c[2]);
      }
    };

    const out = new ByteOut();
    out.str('GIF89a'); out.u16(W); out.u16(H);
    let gpal = null, gmap = null, gbits = 0;
    if (local) { out.byte(7 << 4); out.byte(0); out.byte(0); }
    else {
      gpal = buildPalette(src, maxC, alphaCut); gbits = tableBits(gpal.length); gmap = makeMapper(gpal, offset);
      out.byte(0x80 | (7 << 4) | (gbits - 1)); out.byte(0); out.byte(0);
      putTable(gpal, gbits);
    }
    if (n > 1) { out.bytes([0x21, 0xff, 0x0b]); out.str('NETSCAPE2.0'); out.bytes([3, 1]); out.u16(opts.loop ?? 0); out.byte(0); }
    const delay = Math.max(2, Math.round(opts.delayCs || 4));

    // A frame is written one step late, so the next frame can still change its disposal (see below).
    const flush = (r) => {
      out.bytes([0x21, 0xf9, 4, (r.disposal << 2) | (useTrans ? 1 : 0)]); out.u16(Math.min(65535, r.delay)); out.byte(0); out.byte(0);
      out.byte(0x2c); out.u16(r.x); out.u16(r.y); out.u16(r.w); out.u16(r.h);
      let bits = gbits;
      if (local) { bits = r.bits; out.byte(0x80 | (bits - 1)); putTable(r.pal, bits); } else out.byte(0);
      let sub = r.full;
      if (r.w !== W || r.h !== H) {
        sub = new Uint8Array(r.w * r.h);
        for (let y = 0; y < r.h; y++) sub.set(r.full.subarray((r.y + y) * W + r.x, (r.y + y) * W + r.x + r.w), y * r.w);
      }
      lzw(out, Math.max(2, bits), sub);
    };

    let pending = null;
    for (let f = 0; f < n; f++) {
      const d = src[f].data;
      let cleared = f === 0 || !diff; // true: the canvas is empty when this frame is drawn
      if (diff && f > 0) {
        // Opaque -> transparent can't be expressed by leaving pixels alone. Disposal belongs to the frame it is set
        // on and runs after that frame is shown, so the PREVIOUS frame gets disposal 2 (clear its whole rectangle,
        // so it is widened to the full canvas) and this frame is then written in full onto the empty canvas.
        const pd = src[f - 1].data;
        for (let i = 3; i < d.length; i += 4) if (pd[i] >= alphaCut && d[i] < alphaCut) { cleared = true; break; }
        if (cleared) { pending.disposal = 2; pending.x = 0; pending.y = 0; pending.w = W; pending.h = H; }
      }
      const pd = f > 0 ? src[f - 1].data : null;
      const mask = new Uint8Array(N);
      let x0 = W, y0 = H, x1 = -1, y1 = -1;
      for (let y = 0, p = 0; y < H; y++) {
        for (let x = 0; x < W; x++, p++) {
          const i = p * 4;
          if (d[i + 3] < alphaCut) continue;
          if (!cleared && pd[i + 3] >= alphaCut && pd[i] === d[i] && pd[i + 1] === d[i + 1] && pd[i + 2] === d[i + 2]) continue;
          mask[p] = 1;
          if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y;
        }
      }
      let rect;
      if (!diff) rect = [0, 0, W, H];
      else if (x1 < 0) {
        if (!cleared) { pending.delay += delay; continue; } // nothing changed: just show the last frame longer
        rect = [0, 0, 1, 1];
      } else rect = [x0, y0, x1 - x0 + 1, y1 - y0 + 1];

      let pal = gpal, map = gmap, bits = gbits;
      if (local) {
        const tmp = new Uint8ClampedArray(N * 4);
        for (let p = 0; p < N; p++) if (mask[p]) { const i = p * 4; tmp[i] = d[i]; tmp[i + 1] = d[i + 1]; tmp[i + 2] = d[i + 2]; tmp[i + 3] = 255; }
        pal = buildPalette([{ data: tmp }], maxC, 1); bits = tableBits(pal.length); map = makeMapper(pal, offset);
      }
      const full = new Uint8Array(N); // 0 = transparent (or colour 0 when nothing is transparent)
      for (let y = 0, p = 0; y < H; y++) {
        for (let x = 0; x < W; x++, p++) {
          if (!mask[p]) continue;
          const i = p * 4;
          let r = d[i], g = d[i + 1], b = d[i + 2];
          if (opts.dither) {
            const k = BAYER[(y & 3) * 4 + (x & 3)] * 24;
            r = Math.max(0, Math.min(255, r + k)); g = Math.max(0, Math.min(255, g + k)); b = Math.max(0, Math.min(255, b + k));
          }
          full[p] = map(r, g, b);
        }
      }
      if (pending) flush(pending);
      // Without differencing every frame is drawn on its own: restore to background (2) when there is transparency.
      pending = { x: rect[0], y: rect[1], w: rect[2], h: rect[3], full, pal, bits, delay, disposal: !diff && hasAlpha ? 2 : 1 };
    }
    if (pending) flush(pending);
    out.byte(0x3b);
    return new Blob([out.result()], { type: 'image/gif' });
  }

  // ---------- APNG ----------
  const CRC = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
  function crc32(bytes, c = 0xffffffff) { for (let i = 0; i < bytes.length; i++) c = CRC[(c ^ bytes[i]) & 255] ^ (c >>> 8); return c; }
  const u32 = (v) => [(v >>> 24) & 255, (v >>> 16) & 255, (v >>> 8) & 255, v & 255];
  function chunk(out, type, data) {
    out.bytes(u32(data.length));
    const t = new Uint8Array([...type].map((c) => c.charCodeAt(0)));
    out.bytes(t); out.bytes(data);
    out.bytes(u32((crc32(data, crc32(t)) ^ 0xffffffff) >>> 0));
  }
  async function deflate(raw) {
    const cs = new CompressionStream('deflate');
    const stream = new Blob([raw]).stream().pipeThrough(cs);
    return new Uint8Array(await new Response(stream).arrayBuffer());
  }
  function filtered(f) { // filter type 1 (Sub) per row: cheap and compresses gradients well
    const W = f.width, H = f.height, d = f.data, raw = new Uint8Array(H * (W * 4 + 1));
    for (let y = 0; y < H; y++) {
      const o = y * (W * 4 + 1); raw[o] = 1;
      for (let x = 0; x < W * 4; x++) { const v = d[y * W * 4 + x]; raw[o + 1 + x] = (v - (x >= 4 ? d[y * W * 4 + x - 4] : 0)) & 255; }
    }
    return raw;
  }
  async function encodeApng(frames, opts = {}) {
    const W = frames[0].width, H = frames[0].height, out = new ByteOut();
    out.bytes([137, 80, 78, 71, 13, 10, 26, 10]);
    chunk(out, 'IHDR', new Uint8Array([...u32(W), ...u32(H), 8, 6, 0, 0, 0]));
    const anim = frames.length > 1;
    if (anim) chunk(out, 'acTL', new Uint8Array([...u32(frames.length), ...u32(opts.loop ?? 0)]));
    let seq = 0;
    const num = Math.max(1, Math.round(opts.delayMs || 40)), den = 1000;
    for (let i = 0; i < frames.length; i++) {
      if (anim) {
        chunk(out, 'fcTL', new Uint8Array([...u32(seq++), ...u32(W), ...u32(H), ...u32(0), ...u32(0), (num >> 8) & 255, num & 255, (den >> 8) & 255, den & 255, 1, 0]));
      }
      const z = await deflate(filtered(frames[i]));
      if (i === 0) chunk(out, 'IDAT', z);
      else { const d = new Uint8Array(z.length + 4); d.set(u32(seq++)); d.set(z, 4); chunk(out, 'fdAT', d); }
    }
    chunk(out, 'IEND', new Uint8Array(0));
    return new Blob([out.result()], { type: anim ? 'image/apng' : 'image/png' });
  }

  // ---------- animated WebP (browser-made stills, muxed here) ----------
  function stillMaker(w, h) {
    const off = typeof OffscreenCanvas !== 'undefined';
    const cv = off ? new OffscreenCanvas(w, h) : Object.assign(document.createElement('canvas'), { width: w, height: h });
    const ctx = cv.getContext('2d');
    return async (frame, quality) => {
      ctx.putImageData(frame, 0, 0);
      let blob = null;
      try {
        blob = off ? await cv.convertToBlob({ type: 'image/webp', quality }) : await new Promise((res) => cv.toBlob(res, 'image/webp', quality));
      } catch (e) { blob = null; }
      if (!blob || blob.type !== 'image/webp') throw new Error('webp-unsupported');
      return new Uint8Array(await blob.arrayBuffer());
    };
  }
  // The image chunks (ALPH, VP8, VP8L) of a still WebP, as {type, data}.
  function webpImageChunks(b) {
    const tag = (p) => String.fromCharCode(b[p], b[p + 1], b[p + 2], b[p + 3]);
    if (b.length < 20 || tag(0) !== 'RIFF' || tag(8) !== 'WEBP') throw new Error('webp-unsupported');
    const dv = new DataView(b.buffer, b.byteOffset, b.byteLength), res = [];
    for (let p = 12; p + 8 <= b.length;) {
      const type = tag(p), size = dv.getUint32(p + 4, true);
      if (type === 'ALPH' || type === 'VP8 ' || type === 'VP8L') res.push({ type, data: b.subarray(p + 8, Math.min(b.length, p + 8 + size)) });
      p += 8 + size + (size & 1);
    }
    if (!res.some((c) => c.type !== 'ALPH')) throw new Error('webp-unsupported');
    return res;
  }
  const riffChunk = (out, type, data) => { out.str(type); out.u32(data.length); out.bytes(data); if (data.length & 1) out.byte(0); };

  async function encodeWebp(frames, opts = {}) {
    const W = frames[0].width, H = frames[0].height;
    const quality = Math.max(0, Math.min(1, opts.quality ?? 0.9));
    const dur = Math.max(0, Math.min(0xffffff, Math.round(opts.delayMs ?? 40)));
    const still = stillMaker(W, H);
    const frameChunks = []; let alpha = false;
    for (const f of frames) {
      const cs = webpImageChunks(await still(f, quality));
      if (cs.some((c) => c.type === 'ALPH' || c.type === 'VP8L')) alpha = true;
      const o = new ByteOut();
      o.u24(0); o.u24(0); o.u24(W - 1); o.u24(H - 1); o.u24(dur);
      o.byte(0x02); // blending off (overwrite), do not dispose: every frame covers the whole canvas
      for (const c of cs) riffChunk(o, c.type, c.data);
      frameChunks.push(o.result());
    }
    const out = new ByteOut();
    out.str('RIFF'); out.u32(0); out.str('WEBP');
    const vp = new ByteOut();
    vp.byte(0x02 | (alpha ? 0x10 : 0)); vp.u24(0); vp.u24(W - 1); vp.u24(H - 1);
    riffChunk(out, 'VP8X', vp.result());
    const an = new ByteOut(); an.u32(0); an.u16(opts.loop ?? 0);
    riffChunk(out, 'ANIM', an.result());
    for (const fc of frameChunks) riffChunk(out, 'ANMF', fc);
    const bytes = out.result();
    new DataView(bytes.buffer).setUint32(4, bytes.length - 8, true);
    return new Blob([bytes], { type: 'image/webp' });
  }

  /* Predicted file size in bytes. Pass every frame, or (with opts.total = real frame count) a sample:
     frames[0] is the first frame, the rest are consecutive frames from the middle of the loop. */
  async function estimateSize(frames, format, opts = {}) {
    const total = Math.max(frames.length, opts.total || frames.length);
    const sampled = frames.length < total;
    if (format === 'gif') {
      if (!sampled && frames.length <= 6) return encodeGif(frames, opts).size;
      let win;
      if (sampled) win = frames.slice(1);
      else { const m = Math.max(1, Math.floor(frames.length / 2) - 1); win = frames.slice(m, m + 4); }
      const first = encodeGif([frames[0]], opts).size;
      if (win.length < 2) return first * total;
      const one = encodeGif([win[0]], opts).size, many = encodeGif(win, opts).size;
      return Math.round(first + (total - 1) * Math.max(8, (many - one) / (win.length - 1)));
    }
    let pick = frames;
    if (!sampled && frames.length > 4) {
      const n = frames.length;
      pick = [...new Set([0, Math.floor(n / 3), Math.floor((2 * n) / 3), n - 1])].map((i) => frames[i]);
    }
    const blob = format === 'webp' ? await encodeWebp(pick, opts) : await encodeApng(pick, opts);
    return Math.round((blob.size / pick.length) * total);
  }

  PL.Encode = { encodeGif, encodeApng, encodeWebp, estimateSize };
}
