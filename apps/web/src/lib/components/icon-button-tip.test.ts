import { describe, expect, it } from 'vitest';
import { tipShift } from './IconButton.svelte';

describe('tooltip position at the window edge', () => {
  it('leaves a tooltip that fits where it is', () => {
    expect(tipShift(100, 32, 60, 375)).toBe(0);
  });
  it('moves a tooltip back in from the right edge (the pager’s Next button at 375 px)', () => {
    // button 335..367, tooltip 70 wide centred on 351: right edge at 386
    const shift = tipShift(335, 32, 70, 375);
    expect(shift).toBe(-15);
    expect(351 - 35 + shift + 70).toBeLessThanOrEqual(375 - 4);
  });
  it('moves a tooltip back in from the left edge', () => {
    expect(tipShift(0, 32, 70, 375)).toBe(23);
  });
  it('gives up when the tooltip is wider than the window', () => {
    expect(tipShift(0, 32, 400, 320)).toBe(0);
  });
});
