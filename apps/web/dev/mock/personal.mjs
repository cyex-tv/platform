// Mock API: personal. See ../mock-api.mjs.
import { ME_ID } from './config.mjs';
import { publicAccount } from './directory.mjs';
import { err, findEmote } from './sets.mjs';

// Personal lists (hide for me, favourites), as the API: private to the account, newest first, capped
// (1000 emotes, 200 creators, 1000 favourites), only emotes you may see (approved, or yours).
const HIDDEN_EMOTES = new Map();
const HIDDEN_CREATORS = new Map();
const FAVOURITES = new Map();
export const PERSONAL_FIELDS = ['hiddenEmotes', 'hiddenCreators', 'myFavourites', 'favouriteEmoteIds', 'hideEmote', 'unhideEmote',
  'hideCreator', 'unhideCreator', 'favouriteEmote', 'unfavouriteEmote'];
// Newest first; equal times keep the later-added first.
const newestFirst = (m) => [...m.entries()].reverse();
const seeable = (id) => {
  const e = findEmote(id);
  return e && (e.state === 'APPROVED' || e.owner?.id === ME_ID) ? e : null;
};
export function personalLists(q, v) {
  const has = (name) => new RegExp(`\\b${name}\\b`).test(q);
  const data = {};
  const add = (map, id, cap, full) => {
    if (map.has(id)) return false;
    if (map.size >= cap) return err(full, 'limit_reached');
    map.set(id, new Date().toISOString());
    return true;
  };
  for (const [field, map, cap, full] of [
    ['hideEmote', HIDDEN_EMOTES, 1000, 'You’ve hidden 1.000 emotes, the most there can be. Unhide some first.'],
    ['favouriteEmote', FAVOURITES, 1000, 'You have 1.000 favourites, the most there can be. Remove some first.'],
  ]) {
    if (!has(field)) continue;
    if (!seeable(v.id)) return err('Not found.', 'not_found');
    const r = add(map, v.id, cap, full);
    if (r?.errors) return r;
    data[field] = r;
  }
  if (has('hideCreator')) {
    if (v.accountId === ME_ID) return err('You can’t hide your own emotes.', 'own_account');
    if (!publicAccount(v.accountId ?? '')) return err('Not found.', 'not_found');
    const r = add(HIDDEN_CREATORS, v.accountId, 200, 'You’ve hidden 200 creators, the most there can be. Unhide some first.');
    if (r?.errors) return r;
    data.hideCreator = r;
  }
  if (has('unhideEmote')) data.unhideEmote = HIDDEN_EMOTES.delete(v.id);
  if (has('unhideCreator')) data.unhideCreator = HIDDEN_CREATORS.delete(v.accountId);
  if (has('unfavouriteEmote')) data.unfavouriteEmote = FAVOURITES.delete(v.id);
  if (has('hiddenEmotes')) data.hiddenEmotes = newestFirst(HIDDEN_EMOTES).map(([emoteId, hiddenAt]) => ({ emoteId, hiddenAt, emote: seeable(emoteId) }));
  if (has('hiddenCreators'))
    data.hiddenCreators = newestFirst(HIDDEN_CREATORS).map(([accountId, hiddenAt]) => ({ accountId, hiddenAt, account: publicAccount(accountId) }));
  if (has('favouriteEmoteIds')) data.favouriteEmoteIds = newestFirst(FAVOURITES).map(([id]) => id);
  if (has('myFavourites')) {
    const text = String(v.query ?? '').trim().toLowerCase();
    const per = Math.min(100, Math.max(1, v.perPage ?? 50));
    const page = Math.max(1, v.page ?? 1);
    const all = newestFirst(FAVOURITES).map(([id]) => seeable(id)).filter((e) => e && (!text || e.name.toLowerCase().includes(text)));
    data.myFavourites = { total: all.length, items: all.slice((page - 1) * per, page * per) };
  }
  return { data };
}
