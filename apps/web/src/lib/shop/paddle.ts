// Paddle.js, loaded only when someone starts a checkout (never on a page view), then opened for
// the transaction the API made. The script comes from Paddle's own host, as Paddle requires
// (https://developer.paddle.com/paddlejs/include-paddlejs); the CSP allows that host and the
// checkout frame (svelte.config.js).

const SRC = 'https://cdn.paddle.com/paddle/v2/paddle.js';

export type PaddleEvent = { name?: string; data?: unknown };
type PaddleJs = {
  Environment: { set(env: 'sandbox' | 'production'): void };
  Initialize(options: { token: string; eventCallback?: (e: PaddleEvent) => void }): void;
  Checkout: { open(options: { transactionId: string; settings?: Record<string, unknown> }): void };
};
declare global {
  interface Window {
    Paddle?: PaddleJs;
  }
}

let loading: Promise<PaddleJs> | null = null;

/** Loads Paddle.js once. A failed load isn't kept, so the next checkout tries again. */
export function loadPaddle(): Promise<PaddleJs> {
  if (typeof window === 'undefined') return Promise.reject(new Error('no window'));
  if (window.Paddle) return Promise.resolve(window.Paddle);
  if (loading) return loading;
  const mine = new Promise<PaddleJs>((resolve, reject) => {
    const s = document.createElement('script');
    s.src = SRC;
    s.async = true;
    s.onload = () => (window.Paddle ? resolve(window.Paddle) : reject(new Error('Paddle.js did not start')));
    s.onerror = () => {
      s.remove();
      reject(new Error('Paddle.js did not load'));
    };
    document.head.append(s);
  });
  loading = mine;
  mine.catch(() => {
    if (loading === mine) loading = null;
  });
  return mine;
}

let initialised: string | null = null;
let listener: ((e: PaddleEvent) => void) | null = null;

/**
 * Opens Paddle's checkout for a transaction. `onevent` hears Paddle's events (`checkout.completed`,
 * `checkout.closed`, `checkout.error`). Paddle.js is initialised once per page, so the events of
 * the latest checkout are the ones delivered.
 */
export async function openPaddle(
  checkout: { transactionId: string; clientToken: string; environment: string },
  onevent: (e: PaddleEvent) => void,
): Promise<void> {
  const paddle = await loadPaddle();
  listener = onevent;
  if (initialised !== checkout.clientToken) {
    paddle.Environment.set(checkout.environment === 'sandbox' ? 'sandbox' : 'production');
    paddle.Initialize({ token: checkout.clientToken, eventCallback: (e) => listener?.(e) });
    initialised = checkout.clientToken;
  }
  paddle.Checkout.open({ transactionId: checkout.transactionId });
}
