// What the creator page needs beyond their emotes: the account, and their channels.
import { gql } from '$lib/api';
import { ACCOUNT } from '$lib/queries';

export type Creator = { id: string; displayName: string; avatar?: string | null };
export type CreatorChannel = { platform: 'twitch' | 'kick'; login: string };

/** The creator's own account (name, avatar), even before they have a public emote. Null when unknown. */
export async function lookupAccount(id: string, fetcher: typeof fetch = fetch): Promise<Creator | null> {
  return (await gql(ACCOUNT, { id }, fetcher)).account ?? null;
}

/** The channels a creator has on cyex.tv, to link from their page. */
export async function channelsOf(_id: string): Promise<CreatorChannel[]> {
  // Note: the API has no way to list an account's channels yet, so none are shown.
  return [];
}
