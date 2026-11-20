import { describe, expect, it, vi } from 'vitest';

vi.mock('$env/dynamic/public', () => ({ env: { PUBLIC_SITE_URL: 'https://cyex.tv/' } }));

const { channelMeta, emoteMeta, missingMeta, profileMeta, setMeta, siteMeta, SET_IMAGES } = await import('./meta');

const file = (name: string) => ({ name, format: 'WEBP', scale: 1, width: 32, height: 32, size: 100, isStatic: false });
const pic = (id: string, animated = false) => ({
  host: `https://cdn.cyex.tv/emote/${id}`,
  animated,
  files: [file('1x.webp'), file('4x.webp'), ...(animated ? [file('1x_static.webp'), file('4x_static.webp')] : [])],
});
const emote = (over: Record<string, unknown> = {}) => ({
  id: 'E1',
  name: 'catJam',
  state: 'APPROVED',
  visibility: 'PUBLIC',
  channelCount: 1234,
  flags: { private: false, zeroWidth: false },
  owner: { displayName: 'lumen_fox' },
  ...pic('E1', true),
  ...over,
});

describe('emote card', () => {
  it('names the emote, its kind, creator and channels in cyex numbers, with the still at 4x', () => {
    const m = emoteMeta(emote());
    expect(m.title).toBe('catJam');
    expect(m.description).toBe('Animated emote by lumen_fox for Twitch and Kick chat. In 1.234 channels.');
    expect(m.url).toBe('https://cyex.tv/emotes/E1');
    expect(m.images).toEqual([{ url: 'https://cdn.cyex.tv/emote/E1/4x_static.webp', alt: 'catJam' }]);
    expect(m.card).toBe('summary');
    expect(m.noindex).toBe(false);
  });

  it('a still, zero-width emote in no channel says so plainly', () => {
    const m = emoteMeta(emote({ ...pic('E1'), channelCount: 0, flags: { private: false, zeroWidth: true } }));
    expect(m.description).toBe('Still zero-width emote by lumen_fox for Twitch and Kick chat.');
    expect(m.images[0].url).toBe('https://cdn.cyex.tv/emote/E1/4x.webp');
  });

  it('unlisted, private and pending emotes keep a preview but stay out of search', () => {
    expect(emoteMeta(emote({ visibility: 'UNLISTED' })).noindex).toBe(true);
    expect(emoteMeta(emote({ visibility: 'PRIVATE' })).noindex).toBe(true);
    expect(emoteMeta(emote({ flags: { private: true, zeroWidth: false } })).noindex).toBe(true);
    expect(emoteMeta(emote({ state: 'PENDING' })).noindex).toBe(true);
  });

  it('a picture still processing falls back to the site card', () => {
    const m = emoteMeta(emote({ host: '', files: [] }));
    expect(m.images[0].url).toBe('https://cyex.tv/og.png');
    expect(m.card).toBe('summary_large_image');
  });
});

describe('set card', () => {
  const set = (n: number, over: Record<string, unknown> = {}) => ({
    id: 'S1',
    name: 'Main',
    visibility: 'LISTED',
    capacity: 1000,
    slotsUsed: n,
    owner: { displayName: 'quietoak' },
    emotes: Array.from({ length: n }, (_, i) => ({ name: `e${i}`, emote: { name: `own${i}`, ...pic(`X${i}`) } })),
    ...over,
  });

  it('leads with its first emotes (as named in the set) and says how full it is', () => {
    const m = setMeta(set(7));
    expect(m.images).toHaveLength(SET_IMAGES);
    expect(m.images[0]).toEqual({ url: 'https://cdn.cyex.tv/emote/X0/4x.webp', alt: 'e0' });
    expect(m.description).toBe('Emote set by quietoak, 7 of 1.000 slots used. With e0, e1, e2 and more.');
    expect(m.url).toBe('https://cyex.tv/sets/S1');
  });

  it('an empty set gets the site card; unlisted and private ones are not indexed', () => {
    expect(setMeta(set(0)).images[0].url).toBe('https://cyex.tv/og.png');
    expect(setMeta(set(0)).description).toBe('Emote set by quietoak, 0 of 1.000 slots used.');
    expect(setMeta(set(2, { visibility: 'UNLISTED' })).noindex).toBe(true);
    expect(setMeta(set(2, { visibility: 'PRIVATE' })).noindex).toBe(true);
    expect(setMeta(set(2)).noindex).toBe(false);
  });
});

describe('profile and channel cards', () => {
  const profile = {
    account: { id: 'A1', displayName: 'nora_byte', avatar: 'https://static-cdn.jtvnw.net/a.png' },
    identities: [{ platform: 'TWITCH', login: 'nora_byte' }, { platform: 'KICK', login: 'nora' }],
    activeSets: [{}],
  };

  it('a profile: avatar, platforms, emotes made, channels', () => {
    const m = profileMeta(profile, 1);
    expect(m.description).toBe('nora_byte on Twitch and Kick. 1 emote made. Uses cyex.tv in 1 channel.');
    expect(m.images[0].url).toBe('https://static-cdn.jtvnw.net/a.png');
    expect(m.type).toBe('profile');
    expect(m.url).toBe('https://cyex.tv/users/A1');
  });

  it('a profile with nothing yet, no avatar', () => {
    const m = profileMeta({ ...profile, account: { ...profile.account, avatar: null }, identities: [], activeSets: [] }, 0);
    expect(m.description).toBe('nora_byte. No emotes yet.');
    expect(m.card).toBe('summary_large_image');
  });

  it('a channel: its set and size, with a lowercase canonical address', () => {
    const c = { platform: 'TWITCH', login: 'Nora_Byte', account: { displayName: 'nora_byte', avatar: null } };
    const m = channelMeta(c, { name: 'Main', emotes: [{}, {}] });
    expect(m.title).toBe('nora_byte on Twitch');
    expect(m.description).toBe('2 emotes in nora_byte’s Twitch chat, from the set Main.');
    expect(m.url).toBe('https://cyex.tv/channels/twitch/nora_byte');
    expect(channelMeta(c, null).description).toBe('nora_byte hasn’t picked a set for their Twitch chat yet.');
  });
});

describe('site card', () => {
  it('every other page: the wide mark picture, no og:title of its own', () => {
    const m = siteMeta('/search');
    expect(m.title).toBeUndefined();
    expect(m.images[0]).toMatchObject({ url: 'https://cyex.tv/og.png', width: 1200, height: 630 });
    expect(m.url).toBe('https://cyex.tv/search');
  });

  it('a missing thing is never indexed', () => {
    expect(missingMeta('/emotes/x', 'No emote here')).toMatchObject({ noindex: true, title: 'No emote here' });
  });

  it('names with direction controls are cleaned', () => {
    expect(emoteMeta(emote({ name: 'a‮b' })).title).toBe('ab');
  });
});
