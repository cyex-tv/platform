import { describe, expect, it } from 'vitest';
import { backTarget, baseEmote, channelCount, chatLines, formats, kind, labels, scaleFiles, trust, type PageEmote } from './emote-page';

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
    flags: { zeroWidth: false, nsfw: false, private: false },
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
  it('lists formats once each', () => {
    expect(formats([file(1), file(1, { format: 'AVIF' }), file(2)])).toEqual(['WebP', 'AVIF']);
  });
  it('describes type and labels', () => {
    const e = emote({ flags: { zeroWidth: true, nsfw: true, private: false } });
    expect(kind(e)).toBe('Animated, zero-width');
    expect(labels(e)).toEqual(['animated', 'zero-width', 'NSFW']);
    const wide = emote({ animated: false, files: [file(1, { width: 64 })] });
    expect(kind(wide)).toBe('Still, wide');
  });
});

describe('chat', () => {
  const pool = [
    emote({ id: 'G1', name: 'wideOne', files: [file(1, { width: 96 })] }),
    emote({ id: 'G2', name: 'zw', flags: { zeroWidth: true, nsfw: false, private: false } }),
    emote({ id: 'G3', name: 'plain' }),
    emote({ id: 'G4', name: 'other' }),
  ];
  it('stands a zero-width emote on an ordinary, narrow global emote', () => {
    expect(baseEmote(pool, 'x')?.name).toBe('plain');
    const e = emote({ name: 'hat', flags: { zeroWidth: true, nsfw: false, private: false } });
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
