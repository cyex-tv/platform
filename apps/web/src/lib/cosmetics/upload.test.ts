import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api';
import { apiBackend } from '$lib/paint-studio/backend';
import { checkFile, checkSize, waitReady } from './upload';

describe('picture checks before an upload', () => {
  it('takes PNG, APNG, GIF and WebP up to 5 MB', () => {
    for (const type of ['image/png', 'image/apng', 'image/gif', 'image/webp']) expect(checkFile({ type, size: 1000 })).toBeNull();
    expect(checkFile({ type: 'image/jpeg', size: 1000 })).toMatch(/PNG, APNG, GIF or WebP/);
    expect(checkFile({ type: 'image/svg+xml', size: 1000 })).toMatch(/PNG/);
    expect(checkFile({ type: 'image/png', size: 5 * 1024 * 1024 + 1 })).toMatch(/at most 5 MB \(this one is 5,0 MB\)/);
  });
  it('says what the API would say about the size', () => {
    expect(checkSize('badge', 72, 72)).toBeNull();
    expect(checkSize('badge', 96, 80)).toMatch(/square and at least 72 × 72 px \(this one is 96 × 80 px\)/);
    expect(checkSize('badge', 64, 64)).toMatch(/72 × 72/);
    expect(checkSize('paint', 512, 64)).toBeNull();
    expect(checkSize('paint', 256, 32)).toMatch(/64 px tall \(this one is 32 px\)/);
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
