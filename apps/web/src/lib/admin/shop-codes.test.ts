import { describe, expect, it } from 'vitest';
import { batchState, batchUsage } from './shop-codes';

const now = Date.parse('2026-11-21T12:00:00Z');

describe('batchState', () => {
  it('is active without an end or a switch-off', () => expect(batchState({}, now)).toBe('Active'));
  it('is expired after its end', () => expect(batchState({ expiresAt: '2026-11-20T00:00:00Z' }, now)).toBe('Expired'));
  it('is active before its end', () => expect(batchState({ expiresAt: '2026-12-01T00:00:00Z' }, now)).toBe('Active'));
  it('turned off wins over expired', () => expect(batchState({ expiresAt: '2026-11-20T00:00:00Z', disabledAt: '2026-11-19T00:00:00Z' }, now)).toBe('Turned off'));
});

describe('batchUsage', () => {
  it('counts used and left', () => expect(batchUsage({ codeCount: 10, usesPerCode: 2, redemptions: 5 })).toEqual({ total: 20, used: 5, left: 15 }));
  it('never goes below zero', () => expect(batchUsage({ codeCount: 1, usesPerCode: 1, redemptions: 3 }).left).toBe(0));
});
