import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const gql = vi.fn();
let paddleEvent: ((e: { name?: string }) => void) | null = null;
vi.mock('$lib/api', () => ({ gql: (...a: unknown[]) => gql(...a), ApiError: class extends Error {} }));
vi.mock('$lib/queries', () => ({ ORDER: 'ORDER', START_CHECKOUT: 'START' }));
vi.mock('$lib/tool/notices.svelte', () => ({ notify: vi.fn(), fail: vi.fn() }));
vi.mock('./paddle', () => ({
  openPaddle: (_c: unknown, onevent: (e: { name?: string }) => void) => {
    paddleEvent = onevent;
    return Promise.resolve();
  },
}));

import { safeCheckoutUrl } from './shop';
import { buy, dismiss, flow } from './checkout.svelte';

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

  it('refuses an unsafe hosted page', async () => {
    gql.mockImplementation(async () => started({ clientToken: null, checkoutUrl: 'javascript:alert(1)' }));
    await buy({ itemId: 'i', itemName: 'Item', gift: null, to: null });
    expect(flow.phase).toBe('failed');
  });
});
