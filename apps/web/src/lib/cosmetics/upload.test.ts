import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api';
import { apiBackend } from '$lib/paint-studio/backend';
import { animationOf, checkAnimation, checkFile, checkSize, PICTURE_RULES, pictureAccept, waitReady } from './upload';

describe('picture checks before an upload', () => {
  it('takes PNG, APNG, JPEG, GIF and WebP badges up to 8 MB', () => {
    for (const type of ['image/png', 'image/apng', 'image/jpeg', 'image/gif', 'image/webp']) expect(checkFile({ type, size: 1000 }, 'badge')).toBeNull();
    expect(checkFile({ type: 'image/svg+xml', size: 1000 }, 'badge')).toMatch(/PNG, JPEG, GIF or WebP/);
    expect(checkFile({ type: 'image/png', size: 8 * 1024 * 1024 }, 'badge')).toBeNull();
    expect(checkFile({ type: 'image/png', size: 8 * 1024 * 1024 + 1 }, 'badge')).toMatch(/Badge pictures are at most 8 MB \(this one is 8,0 MB\)/);
    expect(pictureAccept('badge')).toBe('image/png,image/apng,image/jpeg,image/gif,image/webp');
  });
  it('takes the same types and 8 MB for paints', () => {
    for (const type of ['image/png', 'image/apng', 'image/jpeg', 'image/gif', 'image/webp']) expect(checkFile({ type, size: 1000 }, 'paint')).toBeNull();
    expect(checkFile({ type: 'image/svg+xml', size: 1000 }, 'paint')).toMatch(/PNG, JPEG, GIF or WebP/);
    expect(checkFile({ type: 'image/png', size: 8 * 1024 * 1024 }, 'paint')).toBeNull();
    expect(checkFile({ type: 'image/png', size: 5 * 1024 * 1024 + 1 }, 'paint')).toBeNull();
    expect(checkFile({ type: 'image/png', size: 8 * 1024 * 1024 + 1 }, 'paint')).toMatch(/Paint pictures are at most 8 MB \(this one is 8,0 MB\)/);
    expect(PICTURE_RULES.paint.maxBytes).toBe(PICTURE_RULES.badge.maxBytes);
    expect(pictureAccept('paint')).toBe(pictureAccept('badge'));
  });
  it('says what the API would say about the size', () => {
    expect(checkSize('badge', 72, 72)).toBeNull();
    expect(checkSize('badge', 2048, 2048)).toBeNull();
    expect(checkSize('badge', 96, 80)).toMatch(/square and at least 72 × 72 px \(this one is 96 × 80 px\)/);
    expect(checkSize('badge', 64, 64)).toMatch(/72 × 72/);
    expect(checkSize('badge', 2049, 2049)).toMatch(/at most 2048 × 2048 px/);
    expect(checkSize('paint', 512, 64)).toBeNull();
    expect(checkSize('paint', 256, 32)).toMatch(/64 px tall \(this one is 32 px\)/);
    // Paint shape: at least 64 tall, at most 8:1, up to 4096 px a side.
    expect(checkSize('paint', 4096, 512)).toBeNull();
    expect(checkSize('paint', 512, 512)).toBeNull();
    expect(checkSize('paint', 520, 64)).toMatch(/8:1 \(this one is 520 × 64 px\)/);
    expect(checkSize('paint', 4200, 600)).toMatch(/at most 4096 px a side/);
    expect(checkSize('paint', 600, 4200)).toMatch(/at most 4096 px a side/);
  });
});

// Animated files built by hand: only the parts the reader looks at.
function gif(frames: number, delayCs: number): Uint8Array {
  const out = [...'GIF89a'].map((c) => c.charCodeAt(0));
  out.push(1, 0, 1, 0, 0x80, 0, 0, 0, 0, 0, 255, 255, 255);
  for (let i = 0; i < frames; i++) {
    out.push(0x21, 0xf9, 4, 0, delayCs & 255, delayCs >> 8, 0, 0);
    out.push(0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 0x01, 0);
  }
  out.push(0x3b);
  return new Uint8Array(out);
}
function chunk(type: string, data: number[], le = false): number[] {
  const n = data.length;
  const len = le ? [n & 255, (n >> 8) & 255, (n >> 16) & 255, n >>> 24] : [n >>> 24, (n >> 16) & 255, (n >> 8) & 255, n & 255];
  const head = [...type].map((c) => c.charCodeAt(0));
  return le ? [...head, ...len, ...data, ...(n & 1 ? [0] : [])] : [...len, ...head, ...data, 0, 0, 0, 0];
}
function apng(frames: number, delayMs: number): Uint8Array {
  const out = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, ...chunk('IHDR', Array(13).fill(0)), ...chunk('acTL', [0, 0, 0, frames, 0, 0, 0, 0])];
  for (let i = 0; i < frames; i++) {
    const fc = Array(26).fill(0);
    fc[20] = delayMs >> 8;
    fc[21] = delayMs & 255;
    fc[23] = 1000 & 255;
    fc[22] = 1000 >> 8;
    out.push(...chunk('fcTL', fc));
  }
  out.push(...chunk('IEND', []));
  return new Uint8Array(out);
}
function webp(frames: number, delayMs: number): Uint8Array {
  const body = [...'WEBP'].map((c) => c.charCodeAt(0));
  body.push(...chunk('VP8X', Array(10).fill(0), true), ...chunk('ANIM', Array(6).fill(0), true));
  for (let i = 0; i < frames; i++) {
    const data = Array(16).fill(0);
    data[12] = delayMs & 255;
    data[13] = (delayMs >> 8) & 255;
    body.push(...chunk('ANMF', data, true));
  }
  const n = body.length;
  return new Uint8Array([...'RIFF'].map((c) => c.charCodeAt(0)).concat([n & 255, (n >> 8) & 255, (n >> 16) & 255, n >>> 24], body));
}

describe('animated badge checks before an upload', () => {
  it('reads frames and run time from GIF, APNG and WebP', () => {
    expect(animationOf(gif(3, 5))).toEqual({ frames: 3, durationMs: 150 });
    expect(animationOf(apng(4, 250))).toEqual({ frames: 4, durationMs: 1000 });
    expect(animationOf(webp(5, 80))).toEqual({ frames: 5, durationMs: 400 });
    // Under 20 ms counts as 100, as browsers show it.
    expect(animationOf(gif(2, 1))).toEqual({ frames: 2, durationMs: 200 });
    // One frame is a still picture; other bytes aren't read.
    expect(animationOf(gif(1, 5))).toBeNull();
    expect(animationOf(new Uint8Array([0xff, 0xd8, 0xff, 0]))).toBeNull();
    expect(animationOf(new Uint8Array([71, 73, 70]))).toBeNull();
  });
  it('holds animated badges to 120 frames, 10 seconds and 64 MP', () => {
    expect(checkAnimation('badge', animationOf(gif(120, 5)), 72, 72)).toBeNull();
    expect(checkAnimation('badge', animationOf(gif(121, 5)), 72, 72)).toMatch(/at most 120 frames \(this one has 121\)/);
    expect(checkAnimation('badge', animationOf(gif(5, 200)), 72, 72)).toBeNull();
    expect(checkAnimation('badge', animationOf(webp(6, 2000)), 72, 72)).toMatch(/at most 10 seconds \(this one runs 12,0 s\)/);
    expect(checkAnimation('badge', { frames: 120, durationMs: 1000 }, 512, 512)).toBeNull();
    expect(checkAnimation('badge', { frames: 100, durationMs: 1000 }, 1000, 1000)).toMatch(/over 64 MP/);
    // Paints hold to the same limits; a still picture has nothing to check.
    expect(checkAnimation('paint', animationOf(gif(121, 5)), 256, 64)).toMatch(/Animated paints have at most 120 frames/);
    expect(checkAnimation('paint', animationOf(gif(120, 5)), 256, 64)).toBeNull();
    expect(checkAnimation('paint', animationOf(webp(6, 2000)), 256, 64)).toMatch(/at most 10 seconds/);
    expect(checkAnimation('badge', null, 72, 72)).toBeNull();
  });
});

describe('waiting for the files', () => {
  const now = () => Promise.resolve();
  it('asks until ready, and asks again after a blip', async () => {
    const answers: (() => Promise<{ ready: boolean; id: string }>)[] = [
      () => Promise.reject(new ApiError('offline', 'offline')),
      () => Promise.resolve({ ready: false, id: 'a' }),
      () => Promise.resolve({ ready: true, id: 'a' }),
    ];
    const read = vi.fn(() => answers.shift()!());
    await expect(waitReady(read, { sleep: now })).resolves.toEqual({ ready: true, id: 'a' });
    expect(read).toHaveBeenCalledTimes(3);
  });
  it('stops at once when signed out or not allowed, with that reason (not "taking long")', async () => {
    for (const code of ['signed_out', 'forbidden', 'bad_origin']) {
      const read = vi.fn(() => Promise.reject(new ApiError(`no: ${code}`, code)));
      await expect(waitReady(read, { sleep: now })).rejects.toMatchObject({ code });
      expect(read).toHaveBeenCalledTimes(1);
    }
  });
  it('gives up after the timeout (two minutes by default) and when the picture is gone', async () => {
    await expect(waitReady(() => Promise.resolve({ ready: false }), { sleep: now, timeoutMs: 0 })).rejects.toMatchObject({ code: 'image_not_ready' });
    await expect(waitReady(() => Promise.resolve(null), { sleep: now })).rejects.toMatchObject({ code: 'not_found' });
  });
});

describe('the studio upload against the real answer shape', () => {
  afterEach(() => vi.unstubAllGlobals());
  it('reads the picture back through GraphQL even when the POST says it is ready already', async () => {
    // Regression: bytes the API already had came back at once in the REST shape (snake_case,
    // format "webp", is_static), the studio took that as the GraphQL picture, skipped the
    // read-back and picked the wrong file (an AVIF, and the same file as moving and still).
    const rest = { id: 'img1', kind: 'paint', host: 'https://cdn.cyex.tv/cosmetics/paints/abc', animated: true, width: 256, height: 64, ready: true, created_at: 'now',
      files: [
        { name: '4x.avif', format: 'avif', scale: 4, width: 1024, height: 256, size: 1, is_static: true },
        { name: '1x.webp', format: 'webp', scale: 1, width: 256, height: 64, size: 1, is_static: false },
      ] };
    const gqlImage = { id: 'img1', kind: 'PAINT', host: rest.host, animated: true, ready: true, width: 256, height: 64,
      files: [
        { name: '4x.avif', format: 'AVIF', scale: 4, width: 1024, height: 256, isStatic: true },
        { name: '4x.webp', format: 'WEBP', scale: 4, width: 1024, height: 256, isStatic: false },
        { name: '4x_static.webp', format: 'WEBP', scale: 4, width: 1024, height: 256, isStatic: true },
      ] };
    const calls: string[] = [];
    vi.stubGlobal('fetch', vi.fn(async (url: string, init: RequestInit) => {
      calls.push(String(url));
      if (String(url).endsWith('/v1/cosmetics/images')) return new Response(JSON.stringify(rest), { status: 202 });
      expect(String(init.body)).toContain('cosmeticImage');
      return new Response(JSON.stringify({ data: { cosmeticImage: gqlImage } }), { status: 200 });
    }));
    const pic = await apiBackend({ pollMs: 1 }).uploadPicture(new File([new Uint8Array(4)], 'p.webp', { type: 'image/webp' }));
    expect(calls.some((u) => u.endsWith('/v1/gql'))).toBe(true);
    expect(pic).toEqual({ id: 'img1', url: `${rest.host}/4x.webp`, still: `${rest.host}/4x_static.webp` });
  });
});
