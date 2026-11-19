// Minimal PNG writing and reading for the browser checks (RGBA 8-bit only, what Chrome's
// screenshots and these test pictures use), and the painted-pixel measure used on painted names.
import { deflateSync, inflateSync } from 'node:zlib';

const CRC = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = CRC[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, 'latin1');
  data.copy(out, 8);
  out.writeUInt32BE(crc(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
}

/** A PNG of `w × h` whose pixel (x, y) is `at(x, y)` → [r, g, b, a]. */
export function png(w, h, at) {
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) raw.set(at(x, y), y * (w * 4 + 1) + 1 + x * 4);
  const head = Buffer.alloc(13);
  head.writeUInt32BE(w, 0);
  head.writeUInt32BE(h, 4);
  head.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', head), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

/** Decodes an 8-bit RGB or RGBA PNG (no interlace) to { w, h, px: [r, g, b][] }. */
export function readPng(buf) {
  let at = 8;
  let w = 0, h = 0, type = 6;
  const idat = [];
  while (at < buf.length) {
    const len = buf.readUInt32BE(at);
    const kind = buf.toString('latin1', at + 4, at + 8);
    const data = buf.subarray(at + 8, at + 8 + len);
    if (kind === 'IHDR') {
      w = data.readUInt32BE(0);
      h = data.readUInt32BE(4);
      type = data[9];
    }
    if (kind === 'IDAT') idat.push(data);
    at += 12 + len;
  }
  const bpp = type === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(idat));
  const stride = w * bpp;
  const cur = Buffer.alloc(stride), prev = Buffer.alloc(stride);
  const px = [];
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let i = 0; i < stride; i++) {
      const a = i >= bpp ? cur[i - bpp] : 0, b = prev[i], c = i >= bpp ? prev[i - bpp] : 0;
      const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
      const pred = f === 1 ? a : f === 2 ? b : f === 3 ? (a + b) >> 1 : f === 4 ? (pa <= pb && pa <= pc ? a : pb <= pc ? b : c) : 0;
      cur[i] = (line[i] + pred) & 0xff;
    }
    for (let x = 0; x < w; x++) px.push([cur[x * bpp], cur[x * bpp + 1], cur[x * bpp + 2]]);
    cur.copy(prev);
  }
  return { w, h, px };
}

function hsl([r, g, b]) {
  const [R, G, B] = [r / 255, g / 255, b / 255];
  const max = Math.max(R, G, B), min = Math.min(R, G, B), l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const hue = max === R ? (G - B) / d + (G < B ? 6 : 0) : max === G ? (B - R) / d + 2 : (R - G) / d + 4;
  return [hue / 6, s, l];
}

/**
 * How much of a captured painted name is painted, and in how many colours: `cover` is the share of
 * pixels clearly off the ground (the most common border colour), `hues` how many of 12 hue bands
 * and `lums` how many of 9 lightness bands hold at least 3% of them. A plain colour gives about 1
 * of each; a name that vanished gives a tiny cover.
 */
export function paintedPixels(buf) {
  const { w, h, px } = readPng(buf);
  const key = (p) => p.join(',');
  const border = new Map();
  for (let x = 0; x < w; x++) for (const y of [0, h - 1]) border.set(key(px[y * w + x]), (border.get(key(px[y * w + x])) ?? 0) + 1);
  const ground = [...border.entries()].sort((a, b) => b[1] - a[1])[0][0].split(',').map(Number);
  const far = px.filter((p) => Math.abs(p[0] - ground[0]) + Math.abs(p[1] - ground[1]) + Math.abs(p[2] - ground[2]) > 60);
  const hues = new Map(), lums = new Map();
  for (const p of far) {
    const [hh, s, l] = hsl(p);
    const lb = Math.round(l * 8);
    lums.set(lb, (lums.get(lb) ?? 0) + 1);
    if (s > 0.2 && l > 0.12 && l < 0.9) hues.set(Math.floor(hh * 12) % 12, (hues.get(Math.floor(hh * 12) % 12) ?? 0) + 1);
  }
  const floor = Math.max(4, far.length * 0.03);
  const count = (m) => [...m.values()].filter((v) => v >= floor).length;
  return { cover: Math.round((far.length / px.length) * 1000) / 1000, hues: count(hues), lums: count(lums) };
}
