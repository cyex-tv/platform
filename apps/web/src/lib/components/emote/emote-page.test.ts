import { describe, expect, it } from 'vitest';
import { backTarget, baseEmote, channelCount, chatLines, flagsOf, rangeLabel, scaleFiles, tabFrom, tabUrl, tabUrlStale, trust, type PageEmote } from './emote-page';

const file = (scale: number, opts: Partial<PageEmote['files'][number]> = {}) => ({
  name: `${scale}x${opts.isStatic ? '_static' : ''}.webp`,
  format: 'WEBP' as const,
  scale,
  width: 32 * scale,
  height: 32 * scale,
  size: 900 * scale,
  isStatic: false,
  ...opts,
});

function emote(over: Partial<PageEmote> = {}): PageEmote {
  return {
    id: 'E1',
    name: 'fernYawn',
    state: 'APPROVED',
    animated: true,
    tags: [],
    host: 'https://cdn.example/emotes/1',
    createdAt: '2026-11-17T10:00:00.000Z',
    channelCount: 0,
    visibility: 'PUBLIC',
    flags: { zeroWidth: false, private: false, wide: false },
    owner: { id: 'A1', displayName: 'nora_byte', avatar: null },
    files: [1, 2, 3, 4].flatMap((s) => [file(s), file(s, { isStatic: true, size: 400 * s })]),
    ...over,
  };
}

describe('backTarget', () => {
  it('goes back to the search the reader came from', () => {
    const t = backTarget(new URL('http://x/search?q=cat&sort=new'), '/emotes/E1');
    expect(t).toEqual({ href: '/search?q=cat&sort=new', label: 'Back to results', back: true });
  });
  it('names creator and channel pages', () => {
    expect(backTarget(new URL('http://x/users/A1'), '/emotes/E1').label).toBe('Back to creator');
    expect(backTarget(new URL('http://x/channels/twitch/quietoak'), '/emotes/E1').label).toBe('Back to channel');
  });
  it('falls back to all emotes with no history or from elsewhere', () => {
    expect(backTarget(null, '/emotes/E1')).toEqual({ href: '/search', label: 'All emotes', back: false });
    expect(backTarget(new URL('http://x/emotes/E2'), '/emotes/E1').back).toBe(false);
    expect(backTarget(new URL('http://x/developers'), '/emotes/E1').href).toBe('/search');
  });
});

describe('facts', () => {
  it('picks the moving WebP at each scale', () => {
    const files = scaleFiles([...emote().files, file(2, { name: '2x.avif', format: 'AVIF' })]);
    expect(files.map((f) => f.name)).toEqual(['1x.webp', '2x.webp', '3x.webp', '4x.webp']);
  });
  it('has no kind, format or file-size facts any more (owner, 2026/11/22)', async () => {
    const mod: Record<string, unknown> = await import('./emote-page');
    for (const gone of ['kind', 'formats', 'fileSize', 'labels']) expect(mod[gone]).toBeUndefined();
  });
});

describe('chat', () => {
  const pool = [
    emote({ id: 'G1', name: 'wideOne', files: [file(1, { width: 96 })] }),
    emote({ id: 'G2', name: 'zw', flags: { zeroWidth: true, private: false, wide: false } }),
    emote({ id: 'G3', name: 'plain' }),
    emote({ id: 'G4', name: 'other' }),
  ];
  it('stands a zero-width emote on an ordinary, narrow global emote', () => {
    expect(baseEmote(pool, 'x')?.name).toBe('plain');
    const e = emote({ name: 'hat', flags: { zeroWidth: true, private: false, wide: false } });
    const lines = chatLines(e, pool);
    const layered = lines[1].parts.filter((p) => 'emote' in p).map((p) => ('emote' in p ? p.emote.name : ''));
    expect(layered).toEqual(['plain', 'hat']);
  });
  it('still shows the emote with an empty pool', () => {
    const lines = chatLines(emote(), []);
    expect(lines).toHaveLength(2);
    expect(lines.every((l) => l.parts.some((p) => 'emote' in p && p.emote.name === 'fernYawn'))).toBe(true);
  });
});

describe('trust', () => {
  it('has words for every state', () => {
    for (const s of ['APPROVED', 'PENDING', 'REJECTED', 'TAKEN_DOWN'] as const) {
      expect(trust(s).status.length).toBeGreaterThan(0);
      expect(trust(s).line).toMatch(/\.$/);
    }
  });
});

describe('channelCount', () => {
  it('hides the fact when no channel uses the emote', () => {
    expect(channelCount(emote())).toBeNull();
    expect(channelCount(emote({ channelCount: 12 }))).toBe(12);
  });
});

describe('flags on the card', () => {
  it('has none for a still, public, ordinary emote', () => {
    expect(flagsOf(emote({ animated: false }))).toEqual([]);
  });
  it('lists motion, chat behaviour, then visibility, in that order (no NSFW flag)', () => {
    const e = emote({ visibility: 'UNLISTED', flags: { zeroWidth: true, private: false, wide: true } });
    expect(flagsOf(e).map((f) => f.key)).toEqual(['animated', 'zero-width', 'wide', 'unlisted']);
    expect(flagsOf(e).every((f) => f.tip.length > 0)).toBe(true);
  });
  it('marks private emotes, also from the old private flag alone', () => {
    expect(flagsOf(emote({ visibility: 'PRIVATE' })).at(-1)?.key).toBe('private');
    expect(flagsOf(emote({ visibility: null as never, flags: { zeroWidth: false, private: true, wide: false } })).at(-1)?.key).toBe('private');
  });
  it('derives wide from the 1x file when the API flag is off', () => {
    const files = [file(1, { width: 112, height: 32 })];
    expect(flagsOf(emote({ animated: false, files })).map((f) => f.key)).toEqual(['wide']);
  });
});

describe('pager range', () => {
  it('says which rows show, with the total when known', () => {
    expect(rangeLabel(0, 24, 26)).toBe('1–24 of 26');
    expect(rangeLabel(24, 2, 26)).toBe('25–26 of 26');
    expect(rangeLabel(0, 30)).toBe('1–30');
    expect(rangeLabel(0, 24, 1500)).toBe('1–24 of 1.500');
  });
  it('is empty when nothing shows', () => {
    expect(rangeLabel(0, 0, 0)).toBe('');
  });
});

describe('emote page tabs in the address', () => {
  const ids = ['channels', 'activity', 'more'] as const;
  it('reads the tab, falling back to the first for none, unknown or unavailable', () => {
    expect(tabFrom('activity', ids)).toBe('activity');
    expect(tabFrom(null, ids)).toBe('channels');
    expect(tabFrom('nope', ids)).toBe('channels');
    expect(tabFrom('channels', ['activity', 'more'])).toBe('activity');
    expect(tabFrom('more', [])).toBeUndefined();
  });
  it('writes ?tab= for other tabs and a clean address for the first, keeping the rest', () => {
    const here = new URL('http://x/emotes/E1?ref=a#top');
    expect(tabUrl(here, 'activity', 'channels')).toBe('/emotes/E1?ref=a&tab=activity#top');
    expect(tabUrl(new URL('http://x/emotes/E1?tab=activity'), 'channels', 'channels')).toBe('/emotes/E1');
    expect(tabUrl(new URL('http://x/emotes/E1?tab=activity'), 'more', 'channels')).toBe('/emotes/E1?tab=more');
  });
  it('tidies an unknown tab and the default spelled out, never a good one', () => {
    expect(tabUrlStale('nope', 'channels', 'channels')).toBe(true);
    expect(tabUrlStale('channels', 'channels', 'channels')).toBe(true);
    expect(tabUrlStale('activity', 'activity', 'channels')).toBe(false);
    expect(tabUrlStale(null, 'channels', 'channels')).toBe(false);
  });
});
