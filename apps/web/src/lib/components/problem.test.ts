import { describe, expect, it, vi } from 'vitest';
import { COPY, causeOfCode, causeOfStatus, isOnline, problemTitle, resolveCause, serverMessage, unreachableMessage } from './problem';

describe('the one error pattern', () => {
  it('maps API codes and HTTP statuses to four causes', () => {
    expect(causeOfCode('not_found')).toBe('missing');
    expect(causeOfCode('forbidden')).toBe('forbidden');
    expect(causeOfCode('offline')).toBe('server');
    expect(causeOfCode('bad_answer')).toBe('server');
    expect(causeOfCode(null)).toBe('server');
    expect(causeOfStatus(404)).toBe('missing');
    expect(causeOfStatus(403)).toBe('forbidden');
    expect(causeOfStatus(500)).toBe('server');
    expect(causeOfStatus(400)).toBe('server');
  });
  it('blames the network when the browser is offline', () => {
    expect(resolveCause('server', false)).toBe('offline');
    expect(resolveCause('server', true)).toBe('server');
    expect(resolveCause('missing', false)).toBe('missing');
  });
  it('has a payment cause with its own words and a Try again', () => {
    expect(COPY.payment.title).toBe('The payment didn’t go through.');
    expect(COPY.payment.retry).toBe(true);
    expect(resolveCause('payment', false)).toBe('payment');
    expect(problemTitle('payment')).toBe('Payment failed - cyex.tv');
  });
  it('words an unanswered request from COPY, by whether the browser is online', () => {
    expect(unreachableMessage(false)).toBe(`${COPY.offline.title} ${COPY.offline.body}`);
    expect(unreachableMessage(true)).toBe(`${COPY.server.title} ${COPY.server.body}`);
    expect(serverMessage()).toBe(`${COPY.server.title} ${COPY.server.body}`);
    expect(unreachableMessage(false)).not.toMatch(/can.t be reached/);
  });
  it('reads navigator.onLine, and counts as online without one', () => {
    expect(isOnline()).toBe(true);
    vi.stubGlobal('navigator', { onLine: false });
    expect(isOnline()).toBe(false);
    expect(unreachableMessage()).toBe(unreachableMessage(false));
    vi.unstubAllGlobals();
  });
  it('offers Try again only where asking again can help', () => {
    expect(COPY.server.retry && COPY.offline.retry).toBe(true);
    expect(COPY.missing.retry || COPY.forbidden.retry).toBe(false);
  });
  it('titles the page by cause, with a not-found that can name what is missing', () => {
    expect(problemTitle('server')).toBe('Something went wrong - cyex.tv');
    expect(problemTitle('missing', true, 'No emote here')).toBe('No emote here - cyex.tv');
    expect(problemTitle('missing')).toBe('Nothing here - cyex.tv');
    expect(problemTitle('server', false)).toBe('You’re offline - cyex.tv');
  });
});
