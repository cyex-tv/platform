import { describe, expect, it, vi } from 'vitest';
import { ApiError, gql, safeBack, signInUrl } from './api';
import { COPY } from './components/problem';

describe('safeBack', () => {
  it('keeps paths on this site', () => {
    expect(safeBack('/')).toBe('/');
    expect(safeBack('/me/account?tab=1')).toBe('/me/account?tab=1');
  });

  it('falls back to / for anything else', () => {
    for (const bad of ['//evil.test', '/\\evil.test', '/a\\b', 'https://evil.test', 'evil', '']) {
      expect(safeBack(bad)).toBe('/');
    }
  });

  it('is applied to the sign-in address', () => {
    expect(signInUrl('twitch', '//evil.test')).toContain('return_to=%2F');
    expect(signInUrl('twitch', '//evil.test')).not.toContain('evil');
  });
});

describe('uploadForm', () => {
  it('sends only the flags that are on, and tags comma-separated', async () => {
    const { uploadForm } = await import('./api');
    const f = uploadForm(new Blob(['x']), 'catJAM', ['cat', 'dance'], { zeroWidth: true, visibility: 'PUBLIC', nsfw: true });
    expect(f.get('name')).toBe('catJAM');
    expect(f.get('tags')).toBe('cat,dance');
    expect(f.get('zero_width')).toBe('true');
    expect(f.get('private')).toBeNull();
    expect(f.get('visibility')).toBeNull();
    expect(f.get('nsfw')).toBe('true');
    expect(f.get('file')).toBeInstanceOf(Blob);
    const plain = uploadForm(new Blob(['x']), 'a', []);
    expect([...plain.keys()]).toEqual(['name', 'file']);
  });

  it('sends unlisted as visibility only, and private with the older flag too', async () => {
    const { uploadForm } = await import('./api');
    const unlisted = uploadForm(new Blob(['x']), 'a', [], { visibility: 'UNLISTED' });
    expect(unlisted.get('visibility')).toBe('unlisted');
    expect(unlisted.get('private')).toBeNull();
    const priv = uploadForm(new Blob(['x']), 'a', [], { visibility: 'PRIVATE' });
    expect(priv.get('visibility')).toBe('private');
    expect(priv.get('private')).toBe('true');
  });
});

describe('ApiError messages for a request with no answer', () => {
  const failing = (() => Promise.reject(new TypeError('network'))) as unknown as typeof fetch;
  const doc = { toString: () => 'query { x }' } as never;

  it('uses the server wording from COPY while the browser says it is online', async () => {
    const e = (await gql(doc, {}, failing).catch((x) => x)) as ApiError;
    expect(e).toBeInstanceOf(ApiError);
    expect(e.code).toBe('offline');
    expect(e.message).toBe(`${COPY.server.title} ${COPY.server.body}`);
  });

  it('says You’re offline. when the browser has no network', async () => {
    vi.stubGlobal('navigator', { onLine: false });
    try {
      const e = (await gql(doc, {}, failing).catch((x) => x)) as ApiError;
      expect(e.message).toBe(`${COPY.offline.title} ${COPY.offline.body}`);
      expect(e.message).not.toContain('can’t be reached');
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('words an unreadable answer as the server’s too', async () => {
    const bad = (() => Promise.resolve(new Response('<html>', { status: 502 }))) as unknown as typeof fetch;
    const e = (await gql(doc, {}, bad).catch((x) => x)) as ApiError;
    expect(e.code).toBe('bad_answer');
    expect(e.message).toBe(`${COPY.server.title} ${COPY.server.body}`);
  });
});
