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

  it('refuses tabs, newlines and other control characters (browsers strip them into //host)', () => {
    for (const bad of ['/\t/evil.test', '/\n/x', '/\r/x', '/\u0000/x', '/\u007F/x', ' /\t/evil.test']) expect(safeBack(bad)).toBe('/');
    expect(safeBack('/%09/x')).toBe('/%09/x');
  });

  it('is applied to the sign-in address', () => {
    expect(signInUrl('twitch', '//evil.test')).toContain('return_to=%2F');
    expect(signInUrl('twitch', '//evil.test')).not.toContain('evil');
  });
});

describe('gql credentials', () => {
  it('sends no cookie on the server (SSR): credentials omit', async () => {
    let seen: RequestInit | undefined;
    const spy = (async (_u: unknown, init?: RequestInit) => {
      seen = init;
      return new Response(JSON.stringify({ data: { ok: true } }));
    }) as unknown as typeof fetch;
    await gql({ toString: () => 'query { ok }' } as never, {}, spy);
    expect(seen?.credentials).toBe('omit');
  });
});

describe('uploadForm', () => {
  it('sends only the flags that are on, and tags comma-separated', async () => {
    const { uploadForm } = await import('./api');
    const f = uploadForm(new Blob(['x']), 'catJAM', ['cat', 'dance'], { zeroWidth: true, visibility: 'PUBLIC' });
    expect(f.get('name')).toBe('catJAM');
    expect(f.get('tags')).toBe('cat,dance');
    expect(f.get('zero_width')).toBe('true');
    expect(f.get('private')).toBeNull();
    expect(f.get('visibility')).toBeNull();
    // NSFW emotes aren't allowed: the form never sends the old flag.
    expect(f.get('nsfw')).toBeNull();
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

  it('names the owner only when uploading for an account you edit', async () => {
    const { uploadForm } = await import('./api');
    expect(uploadForm(new Blob(['x']), 'a', [], {}, 'OWNER1').get('owner_id')).toBe('OWNER1');
    expect(uploadForm(new Blob(['x']), 'a', [], {}, null).get('owner_id')).toBeNull();
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

describe('gqlPublic (GET with a persisted-query hash, ADR 0035)', () => {
  const doc = (text: string) => ({ toString: () => text }) as never;
  const ok = (data: unknown) => new Response(JSON.stringify({ data }));

  it('asks by hash over GET without credentials, and sends the text once on a miss', async () => {
    const { gqlPublic, sha256Hex } = await import('./api');
    const seen: { url: string; init?: RequestInit }[] = [];
    let n = 0;
    const spy = (async (u: string, init?: RequestInit) => {
      seen.push({ url: String(u), init });
      n += 1;
      if (n === 1) return new Response(JSON.stringify({ errors: [{ message: 'PersistedQueryNotFound', extensions: { code: 'persisted_query_not_found' } }] }));
      return ok({ activeAnnouncement: null });
    }) as unknown as typeof fetch;
    const out = await gqlPublic(doc('query A { activeAnnouncement { id } }'), {}, spy);
    expect(out).toEqual({ activeAnnouncement: null });
    expect(seen).toHaveLength(2);
    const hash = await sha256Hex('query A { activeAnnouncement { id } }');
    for (const { url, init } of seen) {
      expect(init?.method).toBe('GET');
      expect(init?.credentials).toBe('omit');
      const q = new URL(url).searchParams;
      expect(JSON.parse(q.get('extensions')!)).toEqual({ persistedQuery: { version: 1, sha256Hash: hash } });
    }
    expect(new URL(seen[0].url).searchParams.has('query')).toBe(false);
    expect(new URL(seen[1].url).searchParams.get('query')).toBe('query A { activeAnnouncement { id } }');
  });

  it('a hit is one request; variables go along as JSON', async () => {
    const { gqlPublic } = await import('./api');
    const urls: string[] = [];
    const spy = (async (u: string) => {
      urls.push(String(u));
      return ok({ x: 1 });
    }) as unknown as typeof fetch;
    await gqlPublic(doc('query B($id: String!) { x(id: $id) }'), { id: '7' }, spy);
    expect(urls).toHaveLength(1);
    expect(JSON.parse(new URL(urls[0]).searchParams.get('variables')!)).toEqual({ id: '7' });
  });

  it('a mutation never goes over GET', async () => {
    const { gqlPublic } = await import('./api');
    let method: string | undefined;
    const spy = (async (_u: string, init?: RequestInit) => {
      method = init?.method;
      return ok({ y: true });
    }) as unknown as typeof fetch;
    await gqlPublic(doc('mutation C { y }'), {}, spy);
    expect(method).toBe('POST');
  });

  it('throws the API error of a failed answer', async () => {
    const { gqlPublic } = await import('./api');
    const spy = (async () => new Response(JSON.stringify({ errors: [{ message: 'No.', extensions: { code: 'forbidden' } }] }))) as unknown as typeof fetch;
    await expect(gqlPublic(doc('query D { z }'), {}, spy)).rejects.toMatchObject({ code: 'forbidden' });
  });
});
