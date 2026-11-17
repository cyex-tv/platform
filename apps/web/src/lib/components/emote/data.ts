// What the emote page asks for besides the emote itself. Both are best-effort: a failure hides
// the part that needed them instead of failing the page.
import { gql } from '$lib/api';
import { GLOBAL_SET, SEARCH_BY_OWNER } from '$lib/queries';
import type { PageEmote } from './emote-page';

/** The global set's emotes, for the chat preview (a zero-width emote needs one to sit on). */
export async function globalPool(fetcher: typeof fetch = fetch) {
  try {
    return (await gql(GLOBAL_SET, {}, fetcher)).globalSet.emotes.map((m) => ({ ...m.emote, name: m.name }));
  } catch {
    return [];
  }
}

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
