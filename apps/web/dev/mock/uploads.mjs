// Mock API: uploads. See ../mock-api.mjs.
import { deflateSync } from 'node:zlib';

// Uploaded cosmetic pictures (image id -> bytes), served under /cosmetics/uploads/<id>/<n>x.webp.
export const UPLOAD_FILES = new Map();

/** A multipart/form-data body as { name: { data: Buffer, type } }, or null when it isn't one. */
export function multipart(body, contentType = '') {
  const boundary = /boundary=(?:"([^"]+)"|([^;]+))/i.exec(contentType);
  if (!boundary) return null;
  const sep = Buffer.from(`--${boundary[1] ?? boundary[2]}`);
  const out = {};
  let at = body.indexOf(sep);
  while (at >= 0) {
    const start = at + sep.length;
    if (body.subarray(start, start + 2).toString() === '--') break;
    const next = body.indexOf(sep, start);
    if (next < 0) break;
    const part = body.subarray(start + 2, next - 2); // drop the CRLF after the boundary and before the next
    const headEnd = part.indexOf('\r\n\r\n');
    if (headEnd >= 0) {
      const head = part.subarray(0, headEnd).toString('utf8');
      const name = /name="([^"]*)"/i.exec(head)?.[1];
      if (name && !(name in out)) out[name] = { data: part.subarray(headEnd + 4), type: /content-type:\s*([^\r\n]+)/i.exec(head)?.[1] ?? '' };
    }
    at = next;
  }
  return out;
}

/** Format, size and animation of a PNG/APNG, GIF or WebP from its bytes (null for anything else). */
export function probeImage(b) {
  if (b.length > 4 && b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) {
    // A JPEG: the size is in the first start-of-frame segment.
    for (let i = 2; i + 9 < b.length; ) {
      if (b[i] !== 0xff) break;
      const marker = b[i + 1];
      if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker))
        return { mime: 'image/jpeg', width: b.readUInt16BE(i + 7), height: b.readUInt16BE(i + 5), animated: false };
      i += 2 + b.readUInt16BE(i + 2);
    }
    return null;
  }
  if (b.length > 24 && b.readUInt32BE(0) === 0x89504e47) {
    return { mime: 'image/png', width: b.readUInt32BE(16), height: b.readUInt32BE(20), animated: b.includes(Buffer.from('acTL')) };
  }
  if (b.length > 10 && b.subarray(0, 4).toString('latin1') === 'GIF8') {
    // more than one image descriptor (0x2C after a graphic control block) is an animation
    const frames = b.toString('latin1').split('\x21\xF9\x04').length - 1;
    return { mime: 'image/gif', width: b.readUInt16LE(6), height: b.readUInt16LE(8), animated: frames > 1 };
  }
  if (b.length > 30 && b.subarray(0, 4).toString('latin1') === 'RIFF' && b.subarray(8, 12).toString('latin1') === 'WEBP') {
    const chunk = b.subarray(12, 16).toString('latin1');
    const animated = b.includes(Buffer.from('ANIM'));
    if (chunk === 'VP8X') return { mime: 'image/webp', width: 1 + b.readUIntLE(24, 3), height: 1 + b.readUIntLE(27, 3), animated };
    if (chunk === 'VP8 ') return { mime: 'image/webp', width: b.readUInt16LE(26) & 0x3fff, height: b.readUInt16LE(28) & 0x3fff, animated };
    if (chunk === 'VP8L') {
      const v = b.readUInt32LE(21);
      return { mime: 'image/webp', width: (v & 0x3fff) + 1, height: ((v >> 14) & 0x3fff) + 1, animated };
    }
  }
  return null;
}

/** A 4:1 PNG `h` px tall: coloured diagonal bands with fine lines, so detail scales with h. */
const MOSAIC = new Map();
export function mosaicPng(h) {
  if (MOSAIC.has(h)) return MOSAIC.get(h);
  const w = h * 4;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  const cell = h / 4;
  const rule = Math.max(1, h / 128);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const t = x / w;
      const band = ((x + y * 2) / cell) % 1;
      const k = x % cell < rule || y % cell < rule ? 0.55 : 1;
      const o = y * (w * 4 + 1) + 1 + x * 4;
      const c = (v) => Math.max(0, Math.min(255, v * k));
      raw[o] = c(255 * (0.55 + 0.45 * Math.sin(6.28 * (t + band * 0.2))));
      raw[o + 1] = c(255 * (0.45 + 0.4 * Math.sin(6.28 * (t * 1.3 + 0.33))));
      raw[o + 2] = c(255 * (0.6 + 0.4 * Math.sin(6.28 * (t * 0.7 + 0.66 + band * 0.1))));
      raw[o + 3] = 255;
    }
  }
  const table = Array.from({ length: 256 }, (_, n) => { let c = n; for (let i = 0; i < 8; i++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
  const crc = (buf) => { let c = 0xffffffff; for (const v of buf) c = table[(c ^ v) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
  const chunk = (type, data) => {
    const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type), data]);
    const sum = Buffer.alloc(4); sum.writeUInt32BE(crc(body));
    return Buffer.concat([len, body, sum]);
  };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6;
  const png = Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
  MOSAIC.set(h, png);
  return png;
}
