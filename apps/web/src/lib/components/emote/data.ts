// What the emote page asks for besides the emote itself. Both are best-effort: a failure hides
// the part that needed them instead of failing the page.
import { gql } from '$lib/api';
import { EMOTE_CHANNELS, EMOTE_TRENDING, SEARCH_BY_OWNER } from '$lib/queries';
import { CHANNEL_PAGE, type EmoteChannel, type PageEmote } from './emote-page';

/** Up to 12 more approved emotes by the same creator, this one left out. */
export async function moreBy(e: PageEmote, fetcher: typeof fetch = fetch): Promise<PageEmote[]> {
  if (!e.owner || e.state !== 'APPROVED') return [];
  try {
    const found = (await gql(SEARCH_BY_OWNER, { owner: e.owner.id, perPage: 13 }, fetcher)).searchEmotes;
    return found.items.filter((x) => x.id !== e.id && x.owner?.id === e.owner?.id).slice(0, 12);
  } catch {
    return [];
  }
}

/** Where the emote stands among the trending ones of the last 7 days (top 100), or null. */
export async function trendingOf(e: PageEmote, fetcher: typeof fetch = fetch): Promise<{ rank: number; channels: number } | null> {
  if (e.state !== 'APPROVED' || e.flags.private) return null;
  try {
    const t = (await gql(EMOTE_TRENDING, { id: e.id, window: 'WEEK' }, fetcher)).emoteTrending;
    return t ? { rank: t.rank, channels: t.channels } : null;
  } catch {
    return null;
  }
}

/** One page of the channels showing the emote, newest first. Throws when the API can't be reached. */
export async function channelPage(id: string, offset: number, fetcher: typeof fetch = fetch): Promise<EmoteChannel[]> {
  const list = (await gql(EMOTE_CHANNELS, { id, limit: CHANNEL_PAGE, offset }, fetcher)).emoteChannels;
  return list.map((c) => ({ platform: c.platform, login: c.login, accountId: c.account.id, displayName: c.account.displayName, avatar: c.account.avatar }));
}

/** The first page of channels with the emote in their active set; null (the tab is hidden) when none, or on a failure. */
export async function channelsUsing(e: PageEmote, fetcher: typeof fetch = fetch): Promise<EmoteChannel[] | null> {
  if (e.state !== 'APPROVED' || e.channelCount <= 0) return null;
  try {
    const list = await channelPage(e.id, 0, fetcher);
    return list.length ? list : null;
  } catch {
    return null;
  }
}
