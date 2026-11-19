// One purchase at a time, from the click on Buy to "it's yours". The flow lives here, not in the
// item dialog, because Paddle's checkout opens above the page and the dialog (a modal in the top
// layer) would hide it: the dialog closes when the payment opens, and the shop page shows this
// state in its place (CheckoutStatus).
//
//   idle → starting → paying → confirming → paid
//                        ↘ cancelled (closed without paying: open it again or dismiss)
//                                      ↘ slow (still pending after a while: it keeps checking)
//        ↘ failed (an error sentence, or the order was not completed)
//
// Polling asks `order(id)` every few seconds until the order is PAID, REFUNDED or FAILED. The order
// id is kept in sessionStorage while a hosted checkout page (checkoutUrl) has the tab, so coming
// back to /shop picks the purchase up again.
import { gql } from '$lib/api';
import { ORDER, START_CHECKOUT } from '$lib/queries';
import type { OrderFieldsFragment, Platform } from '$lib/gql/graphql';
import { notify } from '$lib/tool/notices.svelte';
import { openPaddle, type PaddleEvent } from './paddle';
import { safeCheckoutUrl, settled, shopMessage, type OrderStatus } from './shop';

export type Phase = 'idle' | 'starting' | 'paying' | 'confirming' | 'slow' | 'cancelled' | 'paid' | 'failed';
export type Recipient = { platform: Platform; login: string } | null;
/** `to`: who a gift goes to, for the sentence (their login, or their name after a resume). */
export type Purchase = { itemId: string; itemName: string; gift: Recipient; to: string | null; /** A resumed order carries no recipient, only whether it was a gift. */ resumedGift?: boolean };

export const flow = $state<{ phase: Phase; purchase: Purchase | null; order: OrderFieldsFragment | null; error: string | null }>({
  phase: 'idle',
  purchase: null,
  order: null,
  error: null,
});

const KEY = 'cyex-shop-order';
const POLL_MS = 2500;
const SLOW_MS = 90_000;
const GIVE_UP_MS = 15 * 60_000;

let timer: ReturnType<typeof setTimeout> | undefined;
let since = 0;
let reopen: (() => void) | null = null;
let gen = 0;

const keep = (id: string | null) => {
  try {
    if (id) sessionStorage.setItem(KEY, id);
    else sessionStorage.removeItem(KEY);
  } catch {
    // Private mode: the flow still works, it just can't be resumed after a redirect.
  }
};

function stop() {
  clearTimeout(timer);
  timer = undefined;
}

function finish(order: OrderFieldsFragment) {
  stop();
  flow.order = order;
  keep(null);
  if (order.status === 'PAID') {
    flow.phase = 'paid';
    flow.error = null;
    // Finished while the shop page was left (a slow confirmation): say so where they are.
    if (typeof location !== 'undefined' && !location.pathname.startsWith('/shop')) notify(`${order.itemName} is yours. Find it in Cosmetics.`);
  } else {
    flow.phase = 'failed';
    flow.error =
      order.status === 'REFUNDED'
        ? 'This order was refunded, so the cosmetics were taken back.'
        : 'The payment wasn’t completed, so nothing was charged. You can try again.';
  }
}

function poll(id: string, mine: number) {
  stop();
  timer = setTimeout(async () => {
    if (mine !== gen) return;
    try {
      const { order } = await gql(ORDER, { id });
      if (mine !== gen) return;
      if (order && settled(order.status as OrderStatus)) return finish(order);
      if (order) flow.order = order;
    } catch {
      // A failed check isn't the end of the purchase: ask again.
    }
    if (mine !== gen) return;
    const waited = Date.now() - since;
    if (waited > GIVE_UP_MS) {
      stop();
      return;
    }
    if (waited > SLOW_MS && flow.phase === 'confirming') flow.phase = 'slow';
    poll(id, mine);
  }, POLL_MS);
}

/**
 * Starts a purchase: the order, then Paddle's checkout (or its hosted page), then the wait for
 * payment. Resolves once the payment window is open (not when it is paid): null, or the sentence
 * for why the order couldn't start (the item dialog shows it where the person is looking).
 */
export async function buy(purchase: Purchase): Promise<string | null> {
  if (flow.phase === 'starting' || flow.phase === 'paying' || flow.phase === 'confirming') return 'You have a payment in progress. Finish or close it first.';
  const mine = ++gen;
  stop();
  flow.purchase = purchase;
  flow.order = null;
  flow.error = null;
  flow.phase = 'starting';
  try {
    const { startCheckout: co } = await gql(START_CHECKOUT, { itemId: purchase.itemId, recipient: purchase.gift });
    if (mine !== gen) return null;
    flow.order = co.order;
    since = Date.now();
    if (settled(co.order.status as OrderStatus)) {
      finish(co.order);
      return null;
    }
    keep(co.order.id);

    const onevent = (e: PaddleEvent) => {
      if (mine !== gen) return;
      if (e.name === 'checkout.completed') {
        flow.phase = 'confirming';
        since = Date.now();
        // The poll may have given up (a window left open past the cutoff): ask again from now.
        poll(co.order.id, mine);
      } else if (e.name === 'checkout.closed' && flow.phase === 'paying') {
        // Closed without paying (a completed checkout moved the phase on already).
        flow.phase = 'cancelled';
      } else if (e.name === 'checkout.error') {
        flow.phase = 'failed';
        flow.error = 'The payment window ran into a problem. Nothing was charged. Try again in a moment.';
        stop();
      }
    };

    const hosted = safeCheckoutUrl(co.checkoutUrl);
    const unsafe = !!co.checkoutUrl && !hosted;
    if (co.clientToken) {
      const open = () => {
        flow.phase = 'paying';
        openPaddle({ transactionId: co.transactionId, clientToken: co.clientToken!, environment: co.environment }, onevent).catch(() => {
          if (mine !== gen) return;
          // Paddle.js didn't load (blocked, offline): the hosted page is the fallback.
          if (hosted) window.location.assign(hosted);
          else {
            flow.phase = 'failed';
            flow.error = 'The payment window couldn’t load. Check that nothing blocks cdn.paddle.com, then try again.';
            stop();
          }
        });
      };
      reopen = open;
      open();
      poll(co.order.id, mine);
    } else if (unsafe) {
      // A payment page we don't recognise is never opened.
      flow.phase = 'failed';
      flow.error = 'The payment page couldn’t be opened safely. Nothing was charged. Try again in a moment.';
    } else if (hosted) {
      flow.phase = 'confirming';
      window.location.assign(hosted);
    } else {
      // Nothing to open: the order may complete without a payment page (a free item, the dev mock).
      flow.phase = 'confirming';
      poll(co.order.id, mine);
    }
    return null;
  } catch (e) {
    if (mine !== gen) return null;
    // The order never started: nothing to show on the page, the dialog says why.
    flow.phase = 'idle';
    flow.purchase = null;
    return shopMessage(e, !!purchase.gift);
  }
}

/** Tries a purchase that failed once more, with the same item and recipient. */
export async function retry(): Promise<void> {
  const purchase = flow.purchase;
  if (flow.phase !== 'failed' || !purchase) return;
  const said = await buy(purchase);
  // The order couldn't start this time either: keep the failure on the page with the reason.
  if (said) {
    flow.purchase = purchase;
    flow.error = said;
    flow.phase = 'failed';
  }
}

/** Opens the payment window again after it was closed. */
export function again() {
  if (flow.phase !== 'cancelled' || !reopen) return;
  reopen();
  // The window was closed for a while: the poll may have stopped, and a payment can still land.
  if (!timer && flow.order) {
    since = Date.now();
    poll(flow.order.id, gen);
  }
}

/** Clears the state (Dismiss, or the next purchase). */
export function dismiss() {
  gen++;
  stop();
  keep(null);
  reopen = null;
  flow.phase = 'idle';
  flow.purchase = null;
  flow.order = null;
  flow.error = null;
}

/**
 * Picks up a purchase a hosted checkout page left (the tab came back to /shop): asks for the order
 * once and carries on from there.
 */
export async function resume(): Promise<void> {
  if (flow.phase !== 'idle') return;
  let id: string | null = null;
  try {
    id = sessionStorage.getItem(KEY);
  } catch {
    return;
  }
  if (!id) return;
  const mine = ++gen;
  try {
    const { order } = await gql(ORDER, { id });
    if (mine !== gen) return;
    if (!order) return keep(null);
    flow.purchase = { itemId: order.itemId, itemName: order.itemName, gift: null, to: order.gift ? (order.recipient?.displayName ?? null) : null, resumedGift: order.gift };
    flow.order = order;
    since = Date.now();
    if (settled(order.status as OrderStatus)) return finish(order);
    flow.phase = 'confirming';
    poll(id, mine);
  } catch {
    keep(null);
  }
}
