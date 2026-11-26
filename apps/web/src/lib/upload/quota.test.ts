import { describe, expect, it } from 'vitest';
import { quotaView } from './quota';

const q = (pending: number, pendingMax: number | null = 20) => ({ accountId: 'A', pending, pendingMax, uploadsThisHour: 0, uploadsPerHour: 20, maxBytes: 8388608, maxSide: 1000, maxFrames: 1000 });

describe('the upload quota line', () => {
  it('counts what waits and how much room is left', () => {
    expect(quotaView(q(3))).toEqual({ line: '3 of 20 waiting for review.', full: false, refusal: null, room: 17 });
    expect(quotaView(null)).toBeNull();
  });

  it('at the cap it is full and says so, for you or the account you upload for', () => {
    expect(quotaView(q(20))?.refusal).toBe('You have 20 emotes waiting for review. Upload more once some are reviewed.');
    expect(quotaView(q(20))?.full).toBe(true);
    expect(quotaView(q(1, 1), true)?.refusal).toBe('That account has 1 emote waiting for review. Upload more once some are reviewed.');
    // A role lowered the cap below what already waits.
    expect(quotaView(q(30, 25))).toMatchObject({ full: true, room: 0 });
  });
});

describe('quotaView without a cap', () => {
  it('shows the count and never blocks (the owners)', () => {
    const v = quotaView(q(250, null));
    expect(v).toMatchObject({ line: '250 waiting for review.', full: false, refusal: null });
    expect(v!.room).toBe(Infinity);
  });
});
