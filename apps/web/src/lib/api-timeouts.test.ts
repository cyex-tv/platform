import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, fetchWith, gql, uploadEmote } from './api';
import { COPY } from './components/problem';

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

/** A deadline that comes after `ms`, standing in for AbortSignal.timeout(20 s). */
const soon = (ms = 5) =>
  vi.spyOn(AbortSignal, 'timeout').mockImplementation(() => {
    const c = new AbortController();
    setTimeout(() => c.abort(), ms);
    return c.signal;
  });

describe('gql never hangs', () => {
  const doc = { toString: () => 'query { x }' } as never;
  /** A server that never answers: the promise only ends when the call's own signal aborts it. */
  const silent = ((_url: string, init?: RequestInit) =>
    new Promise((_resolve, reject) => {
      if (init?.signal?.aborted) reject(new DOMException('timed out', 'TimeoutError'));
      init?.signal?.addEventListener('abort', () => reject(new DOMException('timed out', 'TimeoutError')));
    })) as unknown as typeof fetch;

  it('gives every call a 20 s deadline', async () => {
    const seen: (AbortSignal | null | undefined)[] = [];
    const spy = vi.spyOn(AbortSignal, 'timeout');
    const ok = ((_u: string, init?: RequestInit) => {
      seen.push(init?.signal);
      return Promise.resolve(new Response(JSON.stringify({ data: { x: 1 } })));
    }) as unknown as typeof fetch;
    await gql(doc, {}, ok);
    expect(seen[0]).toBeInstanceOf(AbortSignal);
    expect(spy).toHaveBeenCalledWith(20_000);
  });

  it('ends a call the server never answers with the server wording and code timeout', async () => {
    soon();
    const e = (await gql(doc, {}, silent).catch((x) => x)) as ApiError;
    expect(e).toBeInstanceOf(ApiError);
    expect(e.code).toBe('timeout');
    expect(e.message).toBe(`${COPY.server.title} ${COPY.server.body}`);
  });

  it('says offline for a call that timed out with no network', async () => {
    vi.stubGlobal('navigator', { onLine: false });
    vi.spyOn(AbortSignal, 'timeout').mockImplementation(() => AbortSignal.abort());
    const e = (await gql(doc, {}, silent).catch((x) => x)) as ApiError;
    expect(e.message).toBe(`${COPY.offline.title} ${COPY.offline.body}`);
  });

  it('ends a body that never finishes arriving too', async () => {
    soon();
    const stuckBody = ((_u: string, init?: RequestInit) => {
      const body = new ReadableStream({
        start(ctl) {
          init?.signal?.addEventListener('abort', () => ctl.error(new DOMException('timed out', 'TimeoutError')));
        },
      });
      return Promise.resolve(new Response(body, { status: 200 }));
    }) as unknown as typeof fetch;
    const e = (await gql(doc, {}, stuckBody).catch((x) => x)) as ApiError;
    expect(e.code).toBe('timeout');
  });

  it('keeps that deadline when a caller adds its own signal (fetchWith)', async () => {
    soon();
    const outer = new AbortController();
    const e = (await gql(doc, {}, fetchWith(outer.signal, silent)).catch((x) => x)) as ApiError;
    expect(e.code).toBe('timeout'); // not hanging: the call's deadline survived the wrapper
  });

  it('still stops when the caller’s own signal fires first', async () => {
    const outer = new AbortController();
    const pending = gql(doc, {}, fetchWith(outer.signal, silent)).catch((x) => x);
    outer.abort();
    const e = (await pending) as ApiError;
    expect(e.code).toBe('offline'); // the caller aborted: not a timeout (the loader tells these apart)
  });
});

describe('uploadEmote never hangs', () => {
  type Handler = ((e: unknown) => void) | null;
  class FakeXhr {
    static last: FakeXhr;
    upload: { onprogress: Handler; onload: Handler } = { onprogress: null, onload: null };
    onerror: Handler = null;
    onload: Handler = null;
    status = 0;
    responseText = '';
    responseType = '';
    withCredentials = false;
    aborted = false;
    constructor() {
      FakeXhr.last = this;
    }
    open() {}
    send() {}
    abort() {
      this.aborted = true;
    }
  }
  const file = () => new File([new Uint8Array(10)], 'a.png', { type: 'image/png' });
  const setup = () => {
    vi.useFakeTimers();
    vi.stubGlobal('XMLHttpRequest', FakeXhr);
  };

  it('stalls after 30 s with no progress, and says so', async () => {
    setup();
    const p = uploadEmote(file(), 'a', []).catch((x) => x);
    await vi.advanceTimersByTimeAsync(29_999);
    expect(FakeXhr.last.aborted).toBe(false);
    await vi.advanceTimersByTimeAsync(2);
    const e = (await p) as ApiError;
    expect(e.code).toBe('stalled');
    expect(e.message).toMatch(/^The upload stalled/);
    expect(FakeXhr.last.aborted).toBe(true);
  });

  it('keeps going while bytes keep moving, then waits for the answer', async () => {
    setup();
    const shares: number[] = [];
    const p = uploadEmote(file(), 'a', [], undefined, (s) => shares.push(s));
    for (let i = 1; i <= 4; i++) {
      await vi.advanceTimersByTimeAsync(20_000);
      FakeXhr.last.upload.onprogress?.({ lengthComputable: true, loaded: i, total: 4 });
    }
    expect(shares).toEqual([0.25, 0.5, 0.75, 1]);
    FakeXhr.last.upload.onload?.({});
    await vi.advanceTimersByTimeAsync(50_000); // the API is checking the picture
    FakeXhr.last.status = 202;
    FakeXhr.last.responseText = JSON.stringify({ id: 'E1', name: 'a', state: 'PENDING' });
    await FakeXhr.last.onload?.({});
    await expect(p).resolves.toMatchObject({ id: 'E1' });
  });

  it('stalls too when the API never answers after the last byte', async () => {
    setup();
    const p = uploadEmote(file(), 'a', []).catch((x) => x);
    FakeXhr.last.upload.onload?.({});
    await vi.advanceTimersByTimeAsync(60_001);
    expect(((await p) as ApiError).code).toBe('stalled');
  });

  it('stops on cancel, with code aborted, and ignores a late answer', async () => {
    setup();
    const ctl = new AbortController();
    const p = uploadEmote(file(), 'a', [], undefined, undefined, null, { signal: ctl.signal }).catch((x) => x);
    ctl.abort();
    const e = (await p) as ApiError;
    expect(e.code).toBe('aborted');
    expect(FakeXhr.last.aborted).toBe(true);
    await FakeXhr.last.onload?.({}); // must not throw or change anything
  });

  it('refuses to start when already cancelled', async () => {
    setup();
    const e = (await uploadEmote(file(), 'a', [], undefined, undefined, null, { signal: AbortSignal.abort() }).catch((x) => x)) as ApiError;
    expect(e.code).toBe('aborted');
  });
});
