import { describe, expect, it } from 'vitest';
import { browserOf, chat, cluster, strip, type HomeEmote } from './pick';

function emote(i: number, o: { animated?: boolean; zw?: boolean; wide?: boolean; day?: number } = {}): HomeEmote {
  const w = o.wide ? 64 : 32;
  return {
    id: `e${i}`,
    name: `emote${i}`,
    host: `https://cdn.test/${i}`,
    animated: !!o.animated,
    createdAt: `2026-11-${String(o.day ?? 1).padStart(2, '0')}T00:00:00Z`,
    flags: { zeroWidth: !!o.zw },
    files: [
      { name: '1x.webp', width: w, height: 32, isStatic: false },
      ...(o.animated ? [{ name: '1x_static.webp', width: w, height: 32, isStatic: true }] : []),
    ],
  };
}

const lib = [
  emote(0),
  emote(1, { animated: true }),
  emote(2, { zw: true }),
  emote(3, { wide: true }),
  emote(4, { animated: true, day: 9 }),
  ...Array.from({ length: 10 }, (_, i) => emote(5 + i, { day: 2 + i })),
];

describe('cluster', () => {
  it('leads with animated tiles, adds one wide and one zero-width stack, never repeats', () => {
    const c = cluster(lib);
    expect(c.tiles.slice(0, 2).every((e) => e.animated)).toBe(true);
    expect(c.wide?.id).toBe('e3');
    expect(c.overlay?.id).toBe('e2');
    expect(c.base && !c.base.flags.zeroWidth && !c.base.animated).toBe(true);
    const ids = [...c.tiles, c.wide, c.base, c.overlay].map((e) => e?.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('drops the stack when there is no zero-width emote, and copes with an empty library', () => {
    expect(cluster(lib.filter((e) => !e.flags.zeroWidth)).overlay).toBeNull();
    expect(cluster([])).toEqual({ tiles: [], wide: null, base: null, overlay: null });
  });

  it('skips emotes whose files are not made yet', () => {
    const c = cluster([{ ...emote(99), host: '' }, ...lib]);
    expect(c.tiles.some((e) => e.id === 'e99')).toBe(false);
  });
});

describe('strip', () => {
  it('keeps square emotes only, in the order given', () => {
    expect(strip(lib).some((e) => e.id === 'e3')).toBe(false);
    expect(strip(lib).map((e) => e.id)).toEqual(lib.filter((e) => e.id !== 'e3').map((e) => e.id));
    expect(strip(lib, 3)).toHaveLength(3);
  });
});

describe('chat', () => {
  it('needs eight plain emotes, and gives the platforms different lines', () => {
    expect(chat(lib.slice(0, 5), 'twitch')).toEqual([]);
    const t = chat(lib, 'twitch');
    const k = chat(lib, 'kick');
    expect(t.length).toBeGreaterThan(5);
    expect(JSON.stringify(t)).not.toBe(JSON.stringify(k));
  });
});

describe('browserOf', () => {
  const ua = {
    chrome: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
    edge: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0',
    firefox: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0',
    safari: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15',
    ios: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/131.0 Mobile/15E148 Safari/604.1',
    android: 'Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36',
  };
  it('names the browsers that take the extension', () => {
    expect(browserOf(ua.chrome)).toBe('chrome');
    expect(browserOf(ua.edge)).toBe('chrome');
    expect(browserOf(ua.firefox)).toBe('firefox');
    expect(browserOf(ua.safari)).toBe('other');
    expect(browserOf(ua.ios)).toBe('other');
    expect(browserOf(ua.android)).toBe('other');
  });
});
