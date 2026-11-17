import { describe, expect, it } from 'vitest';
import { ready, size, src, srcset } from './emote';
import { bytes, compact, count, date } from './format';

describe('format', () => {
  it('groups numbers with dots', () => {
    expect(count(0)).toBe('0');
    expect(count(999)).toBe('999');
    expect(count(1000)).toBe('1.000');
    expect(count(1234567)).toBe('1.234.567');
    expect(count(-4500)).toBe('-4.500');
  });

  it('shortens big numbers with a decimal comma', () => {
    expect(compact(950)).toBe('950');
    expect(compact(1234)).toBe('1,2K');
    expect(compact(1000)).toBe('1K');
    expect(compact(2_500_000)).toBe('2,5M');
  });

  it('writes sizes like 1,5 MB', () => {
    expect(bytes(900)).toBe('900 B');
    expect(bytes(1536)).toBe('1,5 KB');
    expect(bytes(5 * 1024 * 1024)).toBe('5 MB');
  });

  it('writes dates as yyyy/mm/dd', () => {
    expect(date('2026-01-05T12:00:00.000Z')).toMatch(/^2026\/01\/0[45]$/);
    expect(date('nonsense')).toBe('');
  });
});

describe('emote pictures', () => {
  const still = {
    host: 'https://cdn.cyex.tv/emotes/abc',
    animated: false,
    files: [{ name: '1x.webp', width: 32, height: 28, isStatic: false }],
  };
  const moving = { ...still, animated: true };

  it('builds CDN URLs per scale, static frames only for animated ones', () => {
    expect(src(still)).toBe('https://cdn.cyex.tv/emotes/abc/2x.webp');
    expect(src(still, 4, true)).toBe('https://cdn.cyex.tv/emotes/abc/4x.webp');
    expect(src(moving, 1, true)).toBe('https://cdn.cyex.tv/emotes/abc/1x_static.webp');
    expect(srcset(still)).toBe(
      'https://cdn.cyex.tv/emotes/abc/1x.webp 1x, https://cdn.cyex.tv/emotes/abc/2x.webp 2x, https://cdn.cyex.tv/emotes/abc/4x.webp 4x',
    );
  });

  it('knows when the files are made and how big 1x is', () => {
    expect(ready(still)).toBe(true);
    expect(ready({ ...still, files: [] })).toBe(false);
    expect(ready({ ...still, host: '' })).toBe(false);
    expect(size(still)).toEqual({ width: 32, height: 28 });
    expect(size({ ...still, files: [] })).toEqual({ width: 32, height: 32 });
  });
});
