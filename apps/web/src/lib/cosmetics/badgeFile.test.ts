import { describe, expect, it } from 'vitest';
import { badgeFiles, lacksLadder, pickBadgeFile, pickSize } from './badgeFile';

const ladder = ['1x.webp', '2x.webp', '4x.webp', '128.webp', '256.webp', '512.webp'];
const old = ['1x.webp', '2x.webp', '4x.webp', '1x.avif', '2x.avif', '4x.avif'];
const pick = (names: string[], drawn: number, dpr: number, still = false) => pickBadgeFile(names, drawn, dpr, still)?.name;

describe('the badge size ladder', () => {
  it('takes the smallest size at or above drawn × pixel ratio, else the largest', () => {
    expect(pickSize([18, 36, 72], 20)).toBe(36);
    expect(pickSize([18, 36, 72], 72)).toBe(72);
    expect(pickSize([18, 36, 72], 200)).toBe(72);
    expect(pickSize([], 18)).toBeNull();
  });

  it('picks the file for the size drawn and the screen', () => {
    expect(pick(ladder, 18, 1)).toBe('1x.webp');
    expect(pick(ladder, 18, 2)).toBe('2x.webp');
    expect(pick(ladder, 18, 3)).toBe('4x.webp');
    // The account picker's 28 px cards.
    expect(pick(ladder, 28, 1)).toBe('2x.webp');
    expect(pick(ladder, 28, 2)).toBe('4x.webp');
    expect(pick(ladder, 28, 3)).toBe('128.webp');
    // The upload preview's 72 px.
    expect(pick(ladder, 72, 1)).toBe('4x.webp');
    expect(pick(ladder, 72, 2)).toBe('256.webp');
    expect(pick(ladder, 72, 3)).toBe('256.webp');
    expect(pick(ladder, 36, 1.5)).toBe('4x.webp');
    expect(pick(ladder, 300, 2)).toBe('512.webp');
  });

  it('counts a ratio of 0 (before mount) as 1', () => {
    expect(pick(ladder, 18, 0)).toBe('1x.webp');
    expect(pick(ladder, 28, 0)).toBe('2x.webp');
  });

  it('falls back cleanly for pictures made before the ladder', () => {
    expect(pick(old, 72, 2)).toBe('4x.webp');
    expect(pick(old, 28, 3)).toBe('4x.webp');
    expect(pick(old, 18, 1)).toBe('1x.webp');
    expect(pick([], 18, 1)).toBeUndefined();
  });

  it('shows the first frame when asked, and animates at the largest animated size there is', () => {
    const animated = [...ladder.slice(0, 4), ...ladder.map((n) => n.replace('.webp', '_static.webp'))];
    // 256 and 512 were too heavy to animate: their still frames exist, the animated files don't.
    expect(pick(animated, 72, 2)).toBe('128.webp');
    expect(pick(animated, 72, 2, true)).toBe('256_static.webp');
    expect(pick(animated, 18, 1, true)).toBe('1x_static.webp');
    // A still picture has no _static files: its files are the still.
    expect(pick(ladder, 18, 1, true)).toBe('1x.webp');
    expect(badgeFiles(animated, false).map((f) => f.px)).toEqual([18, 36, 72, 128]);
  });

  it('knows a picture made before the ladder', () => {
    expect(lacksLadder(old)).toBe(true);
    expect(lacksLadder(ladder)).toBe(false);
    expect(lacksLadder(['1x.webp', '2x.webp', '4x.webp', '128.webp', '256_static.webp', '512_static.webp'])).toBe(false);
    expect(lacksLadder([])).toBe(false);
  });
});
