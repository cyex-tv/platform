import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const gql = vi.fn();
let paddleConfig: unknown = null;
let paddleEvent: ((e: { name?: string }) => void) | null = null;
vi.mock('$lib/api', () => ({ gql: (...a: unknown[]) => gql(...a), ApiError: class extends Error {} }));
vi.mock('$lib/queries', () => ({ ORDER: 'ORDER', START_CHECKOUT: 'START' }));
vi.mock('$lib/tool/notices.svelte', () => ({ notify: vi.fn(), fail: vi.fn() }));
vi.mock('./paddle', () => ({
  openPaddle: (c: unknown, onevent: (e: { name?: string }) => void) => {
    paddleConfig = c;
    paddleEvent = onevent;
    return Promise.resolve();
  },
}));

import { safeCheckoutUrl } from './shop';
import { buy, checkAgain, dismiss, flow } from './checkout.svelte';

const order = (status: string) => ({ id: 'o1', status, itemId: 'i', itemName: 'Item', gift: false });
const started = (extra: object = {}) => ({
  startCheckout: { order: order('PENDING'), clientToken: 'tok', transactionId: 't', environment: 'sandbox', checkoutUrl: null, ...extra },
});

describe('safeCheckoutUrl', () => {
  it('allows https on cyex.tv and Paddle checkout hosts only', () => {
    expect(safeCheckoutUrl('https://buy.paddle.com/checkout/x')).toBe('https://buy.paddle.com/checkout/x');
    expect(safeCheckoutUrl('https://sandbox-buy.paddle.com/c')).toBeTruthy();
    expect(safeCheckoutUrl('https://cyex.tv/shop/pay')).toBeTruthy();
  });
  it('refuses other schemes, hosts, look-alikes and credentials', () => {
    for (const u of ['http://buy.paddle.com/x', 'javascript:alert(1)', 'https://evil.example/x', 'https://buy.paddle.com.evil.example/x', 'https://cyex.tv@evil.example/', 'https://a:b@cyex.tv/', 'nope', '', null, undefined]) {
      expect(safeCheckoutUrl(u as string)).toBeNull();
    }
  });
});

describe('checkout after the poll gave up', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    gql.mockReset();
  });
  afterEach(() => {
    dismiss();
    vi.useRealTimers();
  });

  it('a completion past the 15-minute cutoff starts polling again and reaches paid', async () => {
    gql.mockImplementation(async (q: string) => (q === 'START' ? started() : { order: order('PENDING') }));
    await buy({ itemId: 'i', itemName: 'Item', gift: null, to: null });
    await vi.advanceTimersByTimeAsync(16 * 60_000); // the poll gave up
    const calls = gql.mock.calls.length;
    await vi.advanceTimersByTimeAsync(30_000);
    expect(gql.mock.calls.length).toBe(calls); // nothing polling any more
    gql.mockImplementation(async () => ({ order: order('PAID') }));
    paddleEvent!({ name: 'checkout.completed' });
    expect(flow.phase).toBe('confirming');
    await vi.advanceTimersByTimeAsync(3000);
    expect(flow.phase).toBe('paid');
  });

  it('opens Paddle.js by transaction id when there is no checkout URL, with no redirect', async () => {
    const assign = vi.fn();
    vi.stubGlobal('window', { location: { assign } });
    gql.mockImplementation(async () => started({ checkoutUrl: null }));
    await buy({ itemId: 'i', itemName: 'Item', gift: null, to: null });
    expect(paddleConfig).toMatchObject({ transactionId: 't', clientToken: 'tok' });
    expect(flow.phase).toBe('paying');
    expect(assign).not.toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it('with neither a token nor a URL it waits on the order and never redirects', async () => {
    const assign = vi.fn();
    vi.stubGlobal('window', { location: { assign } });
    gql.mockImplementation(async () => started({ clientToken: null, checkoutUrl: null }));
    await buy({ itemId: 'i', itemName: 'Item', gift: null, to: null });
    expect(flow.phase).toBe('confirming');
    expect(assign).not.toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it('refuses an unsafe hosted page', async () => {
    gql.mockImplementation(async () => started({ clientToken: null, checkoutUrl: 'javascript:alert(1)' }));
    await buy({ itemId: 'i', itemName: 'Item', gift: null, to: null });
    expect(flow.phase).toBe('failed');
  });
});

describe('checkout that stopped checking', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    gql.mockReset();
  });
  afterEach(() => {
    dismiss();
    vi.useRealTimers();
  });

  it('says so after 15 minutes, and Check again asks once more', async () => {
    gql.mockImplementation(async (q: string) => (q === 'START' ? started({ clientToken: null }) : { order: order('PENDING') }));
    await buy({ itemId: 'i', itemName: 'Item', gift: null, to: null });
    await vi.advanceTimersByTimeAsync(16 * 60_000);
    expect(flow.phase).toBe('stalled');
    gql.mockImplementation(async () => ({ order: order('PAID') }));
    checkAgain();
    expect(flow.phase).toBe('confirming');
    await vi.advanceTimersByTimeAsync(3000);
    expect(flow.phase).toBe('paid');
  });
});

describe('idempotency keys', () => {
  afterEach(() => dismiss());

  it('a start that failed sends the same key when the same purchase is tried again; one that answered ends it', async () => {
    const keys: unknown[] = [];
    gql.mockImplementation(async (q: string, v: { idempotencyKey?: string }) => {
      if (q !== 'START') return { order: order('PENDING') };
      keys.push(v.idempotencyKey);
      if (keys.length === 1) throw Object.assign(new Error('slow'), { code: 'timeout' });
      return started({ clientToken: null, checkoutUrl: null });
    });
    const purchase = { itemId: 'i', itemName: 'Item', gift: null, to: null };
    expect(await buy({ ...purchase })).toBeTruthy(); // timed out
    await buy({ ...purchase }); // the same item again, from a fresh click
    expect(keys[0]).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    expect(keys[1]).toBe(keys[0]);
    dismiss();
    await buy({ ...purchase }); // a new purchase after one that started
    expect(keys[2]).not.toBe(keys[0]);
    dismiss();
    await buy({ ...purchase, gift: { platform: 'TWITCH', login: 'friend' } }); // another recipient
    expect(keys[3]).not.toBe(keys[2]);
  });
});