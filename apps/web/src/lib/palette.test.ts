import { describe, expect, it } from 'vitest';
import { announce, asLogin, channelJumps, move, pageJumps, PAGES } from './palette';
import { hasStill, isWide, srcsetW } from './emote';

describe('palette', () => {
  it('spots logins and offers the right platforms', () => {
    expect(asLogin('hi there')).toBeNull();
    expect(asLogin('ab')).toBeNull();
    expect(asLogin('@Lumen_Fox')).toEqual({ twitch: true, kick: true, login: 'lumen_fox' });
    expect(asLogin('nora-byte')).toEqual({ twitch: false, kick: true, login: 'nora-byte' });
    expect(channelJumps('quietoak').map((j) => j.href)).toEqual(['/channels/twitch/quietoak', '/channels/kick/quietoak']);
    expect(channelJumps('two words')).toEqual([]);
  });

  it('filters pages by every word', () => {
    expect(pageJumps('')).toEqual(PAGES);
    expect(pageJumps('api ref').map((p) => p.href)).toEqual(['/developers/reference']);
    expect(pageJumps('zzz')).toEqual([]);
  });

  it('moves the active row and wraps', () => {
    expect(move(-1, 1, 3)).toBe(0);
    expect(move(-1, -1, 3)).toBe(2);
    expect(move(2, 1, 3)).toBe(0);
    expect(move(0, -1, 3)).toBe(2);
    expect(move(0, 1, 0)).toBe(-1);
  });
});

describe('emote cards', () => {
  const files = (w: number, still: boolean) =>
    [1, 2, 3, 4].flatMap((k) => [
      { name: `${k}x.webp`, width: w * k, height: 32 * k, isStatic: false },
      ...(still ? [{ name: `${k}x_static.webp`, width: w * k, height: 32 * k, isStatic: true }] : []),
    ]);
  const e = { host: 'https://cdn.cyex.tv/emotes/a', animated: true, files: files(32, true) };

  it('knows wide emotes and still frames', () => {
    expect(isWide(e)).toBe(false);
    expect(isWide({ ...e, files: files(64, false) })).toBe(true);
    expect(hasStill(e)).toBe(true);
    expect(hasStill({ ...e, files: files(32, false) })).toBe(false);
    expect(hasStill({ ...e, animated: false })).toBe(false);
  });

  it('builds width srcsets for the moving and the still files', () => {
    expect(srcsetW(e)).toBe(
      'https://cdn.cyex.tv/emotes/a/1x.webp 32w, https://cdn.cyex.tv/emotes/a/2x.webp 64w, https://cdn.cyex.tv/emotes/a/3x.webp 96w, https://cdn.cyex.tv/emotes/a/4x.webp 128w',
    );
    expect(srcsetW(e, true)).toContain('1x_static.webp 32w');
  });
});

describe('announce', () => {
  it('counts what is listed', () => {
    expect(announce({ emotes: 6, channels: 2, pages: 1 }, 'lumen')).toBe('6 emotes, 2 channels, 1 page.');
    expect(announce({ emotes: 1, channels: 0, pages: 0 }, 'x')).toBe('1 emote.');
    expect(announce({ emotes: 0, channels: 0, pages: 4 }, '')).toBe('4 pages.');
    expect(announce({ emotes: 1200, channels: 1, pages: 0 }, 'x')).toBe('1.200 emotes, 1 channel.');
  });

  it('says when nothing matches, and nothing without text', () => {
    expect(announce({ emotes: 0, channels: 0, pages: 0 }, 'zzz')).toBe('No emotes or pages match “zzz”.');
    expect(announce({ emotes: 0, channels: 0, pages: 0 }, '')).toBe('');
  });
});
