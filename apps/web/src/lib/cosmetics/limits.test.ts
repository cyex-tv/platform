import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PICTURE_RULES, checkAnimation, checkFile, checkSize, pictureHint, rulesFromLimits } from './upload';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), gqlPublic: (...a: unknown[]) => gql(...a) }));

import { forgetLimits, loadLimits, loadPictureRules } from './limits';

const media = (o: object) => ({ maxBytes: 8 * 1024 * 1024, maxSide: 2048, maxFrames: 120, maxTotalPixels: 64_000_000, maxDurationMs: 10_000, minSide: 72, maxAspect: 1, ...o });
const answer = (badge: object = {}, paint: object = {}) => ({
  limits: { badge: media(badge), paint: media({ maxSide: 4096, minSide: 64, maxAspect: 8, ...paint }) },
});

beforeEach(() => {
  gql.mockReset();
  forgetLimits();
});

describe('picture rules from the API', () => {
  it('are the built-in copy when the API says the same', () => {
    expect(rulesFromLimits(answer().limits)).toEqual(PICTURE_RULES);
  });

  it('use what an operator set instead', () => {
    const rules = rulesFromLimits(answer({ maxBytes: 2 * 1024 * 1024, maxSide: 1024, minSide: 36, maxFrames: 60 }).limits);
    expect(checkFile({ type: 'image/png', size: 3 * 1024 * 1024 }, 'badge', rules)).toMatch(/at most 2 MB/);
    expect(checkFile({ type: 'image/png', size: 3 * 1024 * 1024 }, 'badge')).toBeNull();
    expect(checkSize('badge', 40, 40, rules)).toBeNull();
    expect(checkSize('badge', 30, 30, rules)).toMatch(/at least 36 × 36 px/);
    expect(checkSize('badge', 2000, 2000, rules)).toMatch(/at most 1024 × 1024 px/);
    expect(checkAnimation('badge', { frames: 61, durationMs: 1000 }, 72, 72, rules)).toMatch(/at most 60 frames/);
    expect(pictureHint('badge', rules)).toBe('Square, at least 36 × 36 px. PNG, JPEG, GIF or WebP, up to 2 MB; animated up to 60 frames and 10 seconds.');
  });

  it('take the paint shape from the API too', () => {
    const rules = rulesFromLimits(answer({}, { minSide: 32, maxAspect: 4 }).limits);
    expect(checkSize('paint', 128, 31, rules)).toMatch(/at least 32 px tall/);
    expect(checkSize('paint', 130, 32, rules)).toMatch(/at most 4 times as wide as they are tall, 4:1/);
  });

  it('fall back to the built-in rules without an answer', () => {
    expect(rulesFromLimits(null)).toBe(PICTURE_RULES);
  });
});

describe('loading the limits', () => {
  it('asks once and shares the answer', async () => {
    gql.mockResolvedValue(answer({ maxFrames: 90 }));
    const [a, b] = await Promise.all([loadLimits(), loadLimits()]);
    expect(a).toBe(b);
    expect(gql).toHaveBeenCalledTimes(1);
    expect((await loadPictureRules()).badge.maxFrames).toBe(90);
  });

  it('keeps the built-in rules when the API can’t be reached, and asks again next time', async () => {
    gql.mockRejectedValueOnce(new Error('down'));
    expect(await loadPictureRules()).toBe(PICTURE_RULES);
    gql.mockResolvedValue(answer({ maxFrames: 90 }));
    expect((await loadPictureRules()).badge.maxFrames).toBe(90);
    expect(gql).toHaveBeenCalledTimes(2);
  });
});
