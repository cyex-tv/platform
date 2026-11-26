import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EXPORT_PATH, HINT, expiryText, exportToken, makeToken, offerConsoleToken, resetHint } from './console-token';

const TOKEN = 'Zm9vYmFyYmF6cXV4cXV1eGNvcmdlZ3JhdWx0Z2FycGx5';
const ok = () => vi.fn(async () => Response.json({ token: TOKEN, token_type: 'Bearer', expires_at: '2027-02-20T00:00:00.000Z', session_id: '1' }));

// Storage and cookies that record every write, so a test can say nothing was kept.
const writes: string[] = [];
function storage(name: string) {
  const data = new Map<string, string>();
  return {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => (writes.push(`${name}.${k}=${v}`), data.set(k, v)),
    removeItem: (k: string) => data.delete(k),
    clear: () => data.clear(),
    key: () => null,
    get length() {
      return data.size;
    },
  };
}
function storageUntouched() {
  expect(writes).toEqual([]);
}

describe('console token', () => {
  beforeEach(() => {
    writes.length = 0;
    vi.stubGlobal('localStorage', storage('localStorage'));
    vi.stubGlobal('sessionStorage', storage('sessionStorage'));
    vi.stubGlobal('document', {
      get cookie() {
        return '';
      },
      set cookie(v: string) {
        writes.push(`cookie=${v}`);
      },
    });
    resetHint();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    delete (globalThis as { cyextv?: unknown }).cyextv;
  });

  it('asks the API for a new token with the cookie and returns it, keeping nothing', async () => {
    const f = ok();
    expect(await exportToken(f as unknown as typeof fetch)).toBe(TOKEN);
    expect(f).toHaveBeenCalledTimes(1);
    const [url, init] = f.mock.calls[0] as unknown as [string, RequestInit];
    expect(url.endsWith(EXPORT_PATH)).toBe(true);
    expect(init).toMatchObject({ method: 'POST', credentials: 'include' });
    storageUntouched();
  });

  it('makes a token with its expiry, written 2026/11/26, 14:05', async () => {
    expect(await makeToken(ok() as unknown as typeof fetch)).toEqual({ token: TOKEN, expiresAt: '2027-02-20T00:00:00.000Z' });
    const d = new Date(2026, 10, 26, 14, 5);
    expect(expiryText(d.toISOString())).toBe('2026/11/26, 14:05');
    expect(expiryText(null)).toBe('');
    expect(expiryText('nope')).toBe('');
    storageUntouched();
  });

  it('passes the API errors on', async () => {
    const out = vi.fn(async () => Response.json({ error: 'signed_out', message: 'Sign in to use this.' }, { status: 401 }));
    await expect(exportToken(out as unknown as typeof fetch)).rejects.toMatchObject({ code: 'signed_out' });
    const slow = vi.fn(async () => Response.json({ error: 'rate_limited', message: 'Later.' }, { status: 429 }));
    await expect(exportToken(slow as unknown as typeof fetch)).rejects.toMatchObject({ code: 'rate_limited' });
    const off = vi.fn(async () => {
      throw new TypeError('network');
    });
    await expect(exportToken(off as unknown as typeof fetch)).rejects.toMatchObject({ code: 'offline' });
    const odd = vi.fn(async () => Response.json({}));
    await expect(exportToken(odd as unknown as typeof fetch)).rejects.toMatchObject({ code: 'bad_answer' });
  });

  it('offers cyextv.token() only while signed in, with one hint per page load', async () => {
    const win: { cyextv?: { token?: () => Promise<string> } } = {};
    const log = vi.fn();
    const f = ok();

    offerConsoleToken(false, win, f as unknown as typeof fetch, log);
    expect(win.cyextv?.token).toBeUndefined();
    expect(log).not.toHaveBeenCalled();

    offerConsoleToken(true, win, f as unknown as typeof fetch, log);
    offerConsoleToken(true, win, f as unknown as typeof fetch, log);
    expect(log).toHaveBeenCalledTimes(1);
    expect(log).toHaveBeenCalledWith(HINT);
    expect(f).not.toHaveBeenCalled(); // nothing is made until asked for

    expect(await win.cyextv!.token!()).toBe(TOKEN);
    expect(f).toHaveBeenCalledTimes(1);
    storageUntouched();

    offerConsoleToken(false, win, f as unknown as typeof fetch, log);
    expect(win.cyextv?.token).toBeUndefined();
  });

  it('is on the page global for the console', () => {
    offerConsoleToken(true, undefined, ok() as unknown as typeof fetch, () => {});
    expect(typeof (globalThis as unknown as { cyextv: { token: unknown } }).cyextv.token).toBe('function');
    offerConsoleToken(false, undefined, undefined, () => {});
    expect((globalThis as unknown as { cyextv: { token?: unknown } }).cyextv.token).toBeUndefined();
  });
});
