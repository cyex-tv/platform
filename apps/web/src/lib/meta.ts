// Link previews: what Discord, X, Slack and search engines read from a page's <head>. The public
// pages (emote, set, profile, channel) build theirs in their load function, so it is in the
// server-rendered HTML a crawler gets; every other page gets the site's own card (`siteMeta`).
// The root layout renders whichever the page has with <Meta>.
import { siteUrl } from './api';
import { ready, src, type Picture } from './emote';
import { clean, count } from './format';

export type MetaImage = { url: string; alt: string; width?: number; height?: number };

export type Meta = {
  /** og:title: the thing's own name, without "- cyex.tv" (og:site_name says that). Unset: the page's <title> (and any og:title it sets) stands. */
  title?: string;
  description?: string;
  /** The canonical address, absolute, without the query string. */
  url: string;
  /** In order: crawlers that show one picture take the first. */
  images: MetaImage[];
  /** `summary`: a square picture beside the text (emotes, avatars); `summary_large_image`: the wide site card. */
  card: 'summary' | 'summary_large_image';
  type: 'website' | 'profile';
  /** Kept out of search results (unlisted, private, pending, missing); the preview still works. */
  noindex: boolean;
};

/** The site's own card: the cyex mark on the page ground, 1200 × 630. */
export const SITE_IMAGE = { path: '/og.png', width: 1200, height: 630, alt: 'cyex.tv' } as const;

const canonical = (path: string) => `${siteUrl()}${path}`;
const siteImage = (): MetaImage => ({ url: canonical(SITE_IMAGE.path), alt: SITE_IMAGE.alt, width: SITE_IMAGE.width, height: SITE_IMAGE.height });

/** `1 emote`, `1.234 emotes`. */
const many = (n: number, one: string, more = `${one}s`) => `${count(n)} ${n === 1 ? one : more}`;

/** Any page without its own card: the site's title, its picture and the page's own address. */
export function siteMeta(path: string, noindex = false): Meta {
  return { url: canonical(path), images: [siteImage()], card: 'summary_large_image', type: 'website', noindex };
}

/** A page that answers "nothing here": no preview worth showing, and never indexed. */
export function missingMeta(path: string, title: string): Meta {
  return { ...siteMeta(path, true), title };
}

/** The still first frame at 4x (the biggest), or nothing while it is still processing. */
function still(e: Picture & { name: string }): MetaImage | null {
  if (!ready(e)) return null;
  return { url: src(e, 4, true), alt: clean(e.name) };
}

type EmoteLike = Picture & {
  id: string;
  name: string;
  state: string;
  visibility?: string | null;
  channelCount?: number | null;
  flags: { private: boolean; zeroWidth: boolean };
  owner?: { displayName: string } | null;
};

/** `/emotes/<id>`: its picture, what kind it is, who made it, how many channels use it. */
export function emoteMeta(e: EmoteLike): Meta {
  const name = clean(e.name);
  const kind = `${e.animated ? 'Animated' : 'Still'}${e.flags.zeroWidth ? ' zero-width' : ''} emote`;
  const by = e.owner ? ` by ${clean(e.owner.displayName)}` : '';
  const used = e.channelCount ? ` In ${many(e.channelCount, 'channel')}.` : '';
  const pic = still(e);
  const hidden = e.state !== 'APPROVED' || e.flags.private || (e.visibility != null && e.visibility !== 'PUBLIC');
  return {
    title: name,
    description: `${kind}${by} for Twitch and Kick chat.${used}`,
    url: canonical(`/emotes/${encodeURIComponent(e.id)}`),
    images: pic ? [pic] : [siteImage()],
    card: pic ? 'summary' : 'summary_large_image',
    type: 'website',
    noindex: hidden,
  };
}

type SetLike = {
  id: string;
  name: string;
  visibility: string;
  capacity: number;
  slotsUsed: number;
  owner?: { displayName: string } | null;
  emotes: { name: string; emote: Picture & { name: string } }[];
};

/** How many emotes a set's card shows: the first one leads, the rest follow for crawlers that take several. */
export const SET_IMAGES = 4;

/** `/sets/<id>`: its first emotes (in set order), owner, how full it is. */
export function setMeta(s: SetLike): Meta {
  const name = clean(s.name);
  const by = s.owner ? ` by ${clean(s.owner.displayName)}` : '';
  const used = Math.max(s.slotsUsed, s.emotes.length);
  const pics = s.emotes
    .map((m) => still({ ...m.emote, name: m.name }))
    .filter((p): p is MetaImage => !!p)
    .slice(0, SET_IMAGES);
  const first = s.emotes.slice(0, 3).map((m) => clean(m.name));
  const sample = first.length ? ` With ${first.join(', ')}${used > first.length ? ' and more' : ''}.` : '';
  return {
    title: name,
    description: `Emote set${by}, ${count(used)} of ${count(s.capacity)} slots used.${sample}`,
    url: canonical(`/sets/${encodeURIComponent(s.id)}`),
    images: pics.length ? pics : [siteImage()],
    card: pics.length ? 'summary' : 'summary_large_image',
    type: 'website',
    noindex: s.visibility !== 'LISTED',
  };
}

type ProfileLike = {
  account: { id: string; displayName: string; avatar?: string | null };
  identities: { platform: string; login: string }[];
  activeSets: unknown[];
};

const PLATFORM: Record<string, string> = { TWITCH: 'Twitch', KICK: 'Kick' };

/** `/users/<id>`: avatar, name, where they stream, how many emotes they made. */
export function profileMeta(p: ProfileLike, emotes: number): Meta {
  const name = clean(p.account.displayName);
  const on = [...new Set(p.identities.map((i) => PLATFORM[i.platform]).filter(Boolean))];
  const where = on.length ? ` on ${on.join(' and ')}` : '';
  const made = emotes ? ` ${many(emotes, 'emote')} made.` : ' No emotes yet.';
  const channels = p.activeSets.length ? ` Uses cyex.tv in ${many(p.activeSets.length, 'channel')}.` : '';
  const avatar = p.account.avatar ? [{ url: p.account.avatar, alt: name }] : [];
  return {
    title: name,
    description: `${name}${where}.${made}${channels}`,
    url: canonical(`/users/${encodeURIComponent(p.account.id)}`),
    images: avatar.length ? avatar : [siteImage()],
    card: avatar.length ? 'summary' : 'summary_large_image',
    type: 'profile',
    noindex: false,
  };
}

type ChannelLike = {
  platform: string;
  login: string;
  account: { displayName: string; avatar?: string | null };
};

/** `/channels/<platform>/<login>`: the streamer's avatar, the platform, the active set and its size. */
export function channelMeta(c: ChannelLike, set: { name: string; emotes: unknown[] } | null): Meta {
  const who = clean(c.account.displayName);
  const platform = PLATFORM[c.platform] ?? c.platform;
  const what = set
    ? `${many(set.emotes.length, 'emote')} in ${who}’s ${platform} chat, from the set ${clean(set.name)}.`
    : `${who} hasn’t picked a set for their ${platform} chat yet.`;
  const avatar = c.account.avatar ? [{ url: c.account.avatar, alt: who }] : [];
  return {
    title: `${who} on ${platform}`,
    description: what,
    url: canonical(`/channels/${c.platform.toLowerCase()}/${encodeURIComponent(c.login.toLowerCase())}`),
    images: avatar.length ? avatar : [siteImage()],
    card: avatar.length ? 'summary' : 'summary_large_image',
    type: 'profile',
    noindex: false,
  };
}
