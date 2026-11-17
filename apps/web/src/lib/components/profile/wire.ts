// Slots for data the API doesn't serve yet. Each returns nothing today, so the page hides the slot;
// integration replaces the bodies when the fields land. Keep the return types.

export type Creator = { id: string; displayName: string; avatar?: string | null };
export type CreatorChannel = { platform: 'twitch' | 'kick'; login: string };

/** The creator's own account (name, avatar), even before they have a public emote. */
export async function lookupAccount(_id: string): Promise<Creator | null> {
  // wire: account
  return null;
}

/** The channels a creator has on cyex.tv, to link from their page. */
export async function channelsOf(_id: string): Promise<CreatorChannel[]> {
  // wire: account
  return [];
}
