import { describe, expect, it } from 'vitest';
import { deliveryLine, eventLabel, keyHint, urlProblem, usageBars, usageLine, usageTotals, webhookState, WEBHOOK_EVENTS } from './developers';

const NOW = new Date('2026-11-26T12:00:00Z');

describe('API key usage', () => {
  it('fills 30 days, oldest first, with zeros between', () => {
    const bars = usageBars([{ day: '2026-11-26', requests: 5 }, { day: '2026-11-20', requests: 2 }, { day: '2026-10-01', requests: 99 }], NOW);
    expect(bars).toHaveLength(30);
    expect(bars[0].day).toBe('2026-10-28');
    expect(bars[29]).toEqual({ day: '2026-11-26', requests: 5 });
    expect(bars.find((b) => b.day === '2026-11-20')?.requests).toBe(2);
    expect(bars.reduce((n, b) => n + b.requests, 0)).toBe(7);
  });
  it('sums the window and today', () => {
    expect(usageTotals([{ day: '2026-11-26', requests: 1200 }, { day: '2026-11-25', requests: 34 }], NOW)).toEqual({ total: 1234, today: 1200 });
    expect(usageLine([{ day: '2026-11-25', requests: 12345 }], NOW)).toBe('12.345 requests in 30 days, 0 today');
    expect(usageLine([{ day: '2026-11-26', requests: 1 }], NOW)).toBe('1 request in 30 days, 1 today');
    expect(usageLine([], NOW)).toBe('0 requests in 30 days, 0 today');
  });
  it('says when a key was made and used', () => {
    expect(keyHint({ createdAt: '2026-11-20T10:00:00Z', lastUsedAt: null })).toMatch(/^Made 2026\/11\/2\d \d\d:\d\d, never used$/);
    expect(keyHint({ createdAt: '2026-11-20T10:00:00Z', lastUsedAt: '2026-11-21T10:00:00Z' })).toContain('last used 2026/11/2');
  });
});

describe('webhooks', () => {
  it('names every event and the state', () => {
    expect(WEBHOOK_EVENTS).toHaveLength(8);
    expect(eventLabel('SET_EMOTE_ADDED')).toBe('Emote added');
    expect(eventLabel('SOMETHING_NEW')).toBe('SOMETHING_NEW');
    expect(webhookState({ enabled: true, failedDeliveries: 0 })).toBe('On');
    expect(webhookState({ enabled: true, failedDeliveries: 2 })).toBe('On, 2 failed in a row');
    expect(webhookState({ enabled: false, disabledReason: 'manual', failedDeliveries: 0 })).toBe('Off');
    expect(webhookState({ enabled: false, disabledReason: 'failures', failedDeliveries: 5 })).toBe('Off: too many failed deliveries');
    expect(webhookState({ enabled: false, disabledReason: 'blocked', failedDeliveries: 0 })).toBe('Off: the account was blocked');
  });
  it('describes deliveries', () => {
    expect(deliveryLine({ state: 'delivered', attempts: 1, responseStatus: 204, durationMs: 1234 })).toBe('Delivered · 204 · 1.234 ms');
    expect(deliveryLine({ state: 'failed', attempts: 8, error: 'timed out', durationMs: 10000 })).toBe('Failed · timed out · 8 attempts · 10.000 ms');
    expect(deliveryLine({ state: 'pending', attempts: 0 })).toBe('Waiting');
  });
  it('checks URLs before sending', () => {
    expect(urlProblem('https://hooks.example.com/x')).toBeNull();
    expect(urlProblem('')).toMatch(/Give the URL/);
    expect(urlProblem('http://hooks.example.com')).toMatch(/https/);
    expect(urlProblem('nope')).toMatch(/isn't a URL/);
    expect(urlProblem('https://a:b@example.com')).toMatch(/password/);
    expect(urlProblem('https://example.com:8443/x')).toMatch(/443/);
    expect(urlProblem('https://example.com:443/x')).toBeNull();
  });
});
