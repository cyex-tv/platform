// The public pages' +page.ts load functions: how they read the address, which calls they make,
// and what they hand the page when the API fails.
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));
vi.mock('$lib/components/emote/data', () => ({
  moreBy: vi.fn(async () => ['more']),
  channelsUsing: vi.fn(async () => ['channels']),
  trendingOf: vi.fn(async () => ['trending']),
}));

const { ApiError } = await import('$lib/api');
const { SEARCH, SEARCH_BY_OWNER, ACCOUNT_PROFILE, ACCOUNT_COSMETICS, EMOTE, PUBLIC_SET, SHOP_ITEMS } = await import('$lib/queries');
const { PER_PAGE } = await import('$lib/components/browse/params');

const fetch = vi.fn() as unknown as typeof globalThis.fetch;
const at = (path: string) => new URL(`http://localhost${path}`);
// The load functions take SvelteKit's event; the tests pass only what they read.
const call = (load: unknown, event: Record<string, unknown>) => (load as (e: unknown) => Promise<any>)({ fetch, ...event });

beforeEach(() => {
  gql.mockReset();
});

describe('/search', () => {
  const page = async () => (await import('./search/+page')).load;
  const emotes = (total: number, n = 2) => ({ searchEmotes: { total, items: Array.from({ length: n }, (_, i) => ({ id: `E${i}` })) } });

  it('reads the address into the query: text, sort, kind, tags', async () => {
    gql.mockResolvedValue(emotes(2));
    const out = await call(await page(), { url: at('/search?q=%20pog%20&sort=new&kind=animated&tag=cat&tag=dog') });
    expect(gql).toHaveBeenCalledTimes(1);
    const [doc, vars] = gql.mock.calls[0];
    expect(doc).toBe(SEARCH);
    expect(vars).toMatchObject({ query: 'pog', sort: 'NEW', animated: true, tags: ['cat', 'dog'], page: 1, perPage: PER_PAGE });
    expect(out.view).toMatchObject({ q: 'pog', sort: 'new', kind: 'animated' });
    expect(out.error).toBeNull();
  });

  it('a shared link with n=3 loads three pages, but never more than exist', async () => {
    gql.mockImplementation(async (_d: unknown, v: { page: number }) => ({ searchEmotes: { total: PER_PAGE * 2 + 1, items: [{ id: `p${v.page}` }] } }));
    const out = await call(await page(), { url: at('/search?q=a&n=3') });
    expect(gql.mock.calls.map(([, v]) => v.page)).toEqual([1, 2, 3]);
    expect(out.pages).toBe(3);
    expect(out.items.map((i: { id: string }) => i.id)).toEqual(['p1', 'p2', 'p3']);

    gql.mockClear();
    gql.mockImplementation(async (_d: unknown, v: { page: number }) => ({ searchEmotes: { total: PER_PAGE, items: [{ id: `p${v.page}` }] } }));
    const small = await call(await page(), { url: at('/search?q=a&n=9') });
    expect(gql).toHaveBeenCalledTimes(1);
    expect(small.pages).toBe(1);
  });

  it('clamps nonsense in the address instead of passing it on', async () => {
    gql.mockResolvedValue(emotes(0, 0));
    const out = await call(await page(), { url: at('/search?q=a&n=999&sort=bogus&kind=nope') });
    expect(out.view.pages).toBeLessThanOrEqual(10);
    expect(gql.mock.calls[0][1]).toMatchObject({ sort: 'RELEVANCE', animated: null });
  });

  it('hands over the API’s message, or a plain one, when the search fails', async () => {
    gql.mockRejectedValueOnce(new ApiError('Too many searches.', 'rate_limited'));
    const a = await call(await page(), { url: at('/search?q=a') });
    expect(a).toMatchObject({ error: 'Too many searches.', items: [], total: 0 });
    gql.mockRejectedValueOnce(new TypeError('network'));
    const b = await call(await page(), { url: at('/search?q=a') });
    expect(b.error).toBe('Search isn’t available right now.');
  });

  it('an old Users tab link lands on the emote search, the name kept, without asking for people', async () => {
    for (const [from, to] of [['/search?tab=users&q=ana&n=2', '/search?q=ana&n=2'], ['/search?tab=users', '/search']]) {
      try {
        await call(await page(), { url: at(from) });
        expect.unreachable();
      } catch (e) {
        expect(e).toMatchObject({ status: 308, location: to });
      }
    }
    expect(gql).not.toHaveBeenCalled();
  });
});

describe('/emotes redirects to /search with the query kept', () => {
  it('308', async () => {
    const { load } = await import('./emotes/+page');
    try {
      await call(load, { url: at('/emotes?q=pog&sort=top') });
      expect.unreachable();
    } catch (e) {
      expect(e).toMatchObject({ status: 308, location: '/search?q=pog&sort=top' });
    }
  });
});

describe('/users/[id]', () => {
  const page = async () => {
    const { load } = await import('./users/[id]/+page');
    // SvelteKit's event: the address and untrack, which these loads read.
    return (e: Record<string, unknown>) => (load as (e: unknown) => unknown)({ url: at(`/users/${(e.params as { id: string }).id}`), untrack: (f: () => unknown) => f(), ...e });
  };

  it('an old Collection link opens the Cosmetics tab, asking nothing', async () => {
    try {
      await call(await page(), { params: { id: 'U 1' }, url: at('/users/U%201?s=collection') });
      expect.unreachable();
    } catch (e) {
      expect(e).toMatchObject({ status: 308, location: '/users/U%201?s=cosmetics' });
    }
    expect(gql).not.toHaveBeenCalled();
  });

  it('asks for the person’s emotes, profile and look by the id in the address', async () => {
    gql.mockImplementation(async (doc: unknown) => {
      if (doc === SEARCH_BY_OWNER) return { searchEmotes: { items: [{ id: 'E1' }], total: 1 } };
      if (doc === ACCOUNT_PROFILE) return { accountProfile: { account: { id: 'U1', displayName: 'nora' }, identities: [], activeSets: [] } };
      if (doc === ACCOUNT_COSMETICS) return { accountCosmetics: [{ accountId: 'U1', badge: 'B', paint: 'P' }, { accountId: 'U2', badge: 'x', paint: 'y' }] };
    });
    const out = await call(await page(), { params: { id: 'U1' } });
    expect(gql.mock.calls.find(([d]) => d === SEARCH_BY_OWNER)?.[1]).toMatchObject({ owner: 'U1', page: 1, sort: 'TOP' });
    expect(gql.mock.calls.find(([d]) => d === ACCOUNT_COSMETICS)?.[1]).toEqual({ accountIds: ['U1'] });
    expect(out).toMatchObject({ id: 'U1', creator: { id: 'U1' }, look: { badge: 'B', paint: 'P' }, total: 1, error: null });
    expect(out.meta).toMatchObject({ title: 'nora', url: 'https://cyex.tv/users/U1', noindex: false });
  });

  it('an unknown person has no profile and an empty look', async () => {
    gql.mockImplementation(async (doc: unknown) =>
      doc === SEARCH_BY_OWNER ? { searchEmotes: { items: [], total: 0 } } : doc === ACCOUNT_PROFILE ? { accountProfile: null } : { accountCosmetics: [] },
    );
    const out = await call(await page(), { params: { id: 'nobody' } });
    expect(out).toMatchObject({ profile: null, creator: null, look: { badge: null, paint: null }, meta: { noindex: true } });
  });

  it('a failure becomes the page’s error, not a throw', async () => {
    gql.mockRejectedValue(new ApiError('Down.', 'server'));
    expect(await call(await page(), { params: { id: 'U1' } })).toMatchObject({ id: 'U1', profile: null, items: [], error: 'Down.' });
    gql.mockRejectedValue(new Error('x'));
    expect((await call(await page(), { params: { id: 'U1' } })).error).toBe('This page can’t load right now.');
  });
});

describe('/emotes/[id]', () => {
  const page = async () => (await import('./emotes/[id]/+page')).load;

  it('loads the emote and what goes beside it', async () => {
    const emote = { id: 'E1', name: 'pog', state: 'APPROVED', visibility: 'PUBLIC', animated: false, host: '', files: [], flags: { private: false, zeroWidth: false } };
    gql.mockResolvedValue({ emote });
    const out = await call(await page(), { params: { id: 'E1' } });
    expect(gql.mock.calls[0][0]).toBe(EMOTE);
    expect(gql.mock.calls[0][1]).toEqual({ id: 'E1' });
    expect(out).toMatchObject({ id: 'E1', emote, more: ['more'], channels: ['channels'], trending: ['trending'] });
    expect(out.meta).toMatchObject({ title: 'pog', url: 'https://cyex.tv/emotes/E1', noindex: false });
  });

  it('no emote for the server: the page asks again in the browser', async () => {
    gql.mockResolvedValue({ emote: null });
    expect(await call(await page(), { params: { id: 'E9' } })).toMatchObject({ id: 'E9', emote: null, more: [], channels: null, trending: null, meta: { noindex: true } });
  });

  it('an API failure is a 503 with its message', async () => {
    gql.mockRejectedValue(new ApiError('Down.', 'server'));
    await expect(call(await page(), { params: { id: 'E1' } })).rejects.toMatchObject({ status: 503, body: { message: 'Down.' } });
  });
});

describe('/sets/[id]', () => {
  const page = async () => (await import('./sets/[id]/+page')).load;

  it('loads a public set by id; a private one comes back as nothing', async () => {
    const set = { id: 'S1', name: 'Main', visibility: 'LISTED', capacity: 600, slotsUsed: 0, emotes: [] };
    gql.mockResolvedValue({ set });
    expect(await call(await page(), { params: { id: 'S1' } })).toMatchObject({ id: 'S1', set, meta: { title: 'Main', noindex: false } });
    expect(gql.mock.calls[0][0]).toBe(PUBLIC_SET);
    expect(gql.mock.calls[0][1]).toEqual({ id: 'S1' });
    gql.mockResolvedValue({ set: null });
    expect(await call(await page(), { params: { id: 'S2' } })).toMatchObject({ id: 'S2', set: null, meta: { noindex: true } });
  });

  it('an API failure is a 503', async () => {
    gql.mockRejectedValue(new ApiError('Down.', 'server'));
    await expect(call(await page(), { params: { id: 'S1' } })).rejects.toMatchObject({ status: 503 });
  });
});

describe('/shop', () => {
  const page = async () => (await import('./shop/+page')).load;

  it('lists what is on sale', async () => {
    gql.mockResolvedValue({ shopItems: [{ id: 'I1' }] });
    expect(await call(await page(), {})).toEqual({ items: [{ id: 'I1' }], error: null });
    expect(gql.mock.calls[0][0]).toBe(SHOP_ITEMS);
    expect(gql.mock.calls[0][1]).toEqual({ all: false });
  });

  it('tells offline from a server problem', async () => {
    gql.mockRejectedValueOnce(new ApiError('x', 'offline'));
    expect((await call(await page(), {})).error).toBe('offline');
    gql.mockRejectedValueOnce(new ApiError('x', 'server'));
    expect((await call(await page(), {})).error).toBe('server');
    gql.mockRejectedValueOnce(new TypeError('x'));
    expect((await call(await page(), {})).error).toBe('server');
  });
});

describe('/cosmetics', () => {
  it('redirects to the shop (the gallery is gone)', async () => {
    const { load } = await import('./cosmetics/+page');
    try {
      await call(load, {});
      expect.unreachable();
    } catch (e) {
      expect(e).toMatchObject({ status: 308, location: '/shop' });
    }
  });
});
