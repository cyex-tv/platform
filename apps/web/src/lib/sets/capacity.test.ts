import { describe, expect, it } from 'vitest';
import { capacityProblem, capacityValue, rangeHint, type CapacityRange } from './capacity';

const user: CapacityRange = { min: 1, max: 1000, ownerMax: 1000, used: 0, staff: false };
const holding: CapacityRange = { min: 24, max: 1000, ownerMax: 1000, used: 24, staff: false };
const subscriber: CapacityRange = { min: 1, max: 2500, ownerMax: 2500, used: 0, staff: false };
const staff: CapacityRange = { min: 1, max: 1_000_000, ownerMax: 1000, used: 0, staff: true };

describe('set capacity', () => {
  it('takes 1 to the owner’s limit for users', () => {
    expect(capacityProblem('1', user)).toBeNull();
    expect(capacityProblem('1000', user)).toBeNull();
    expect(capacityProblem('1.000', user)).toBeNull();
    expect(capacityProblem('0', user)).toBe('At least 1.');
    expect(capacityProblem('1001', user)).toBe('Up to 1.000.');
    expect(capacityProblem('', user)).toMatch(/number/);
    expect(capacityProblem('12a', user)).toMatch(/whole number/);
    expect(capacityProblem('-5', user)).toMatch(/whole number/);
  });

  it('a role limit raises it, staff go to a million', () => {
    expect(capacityProblem('2500', subscriber)).toBeNull();
    expect(capacityProblem('2501', subscriber)).toBe('Up to 2.500.');
    expect(capacityProblem('1000000', staff)).toBeNull();
    expect(capacityProblem('1000001', staff)).toBe('Up to 1.000.000.');
  });

  it('never goes below what the set holds', () => {
    expect(capacityProblem('23', holding)).toMatch(/holds 24 emotes/);
    expect(capacityProblem('24', holding)).toBeNull();
  });

  it('reads dots as thousands marks', () => {
    expect(capacityValue('2.500')).toBe(2500);
    expect(capacityValue(' 12 ')).toBe(12);
    expect(capacityValue('x')).toBeNull();
  });

  it('says the range under the field', () => {
    expect(rangeHint(user)).toBe('1 to 1.000 slots.');
    expect(rangeHint(holding)).toBe('24 to 1.000 slots: it holds 24.');
    expect(rangeHint(staff)).toBe('1 to 1.000.000 slots. Above 1.000 only as staff.');
  });
});
