// Your personal lists: emotes and creators you hid from your own view ("Hide for me"), and your
// favourite emotes. They are private, so the API keeps the public reads the same for everyone
// (and cacheable) and the site filters with these lists here: browse, search, trending and the
// header search drop what you hid; hearts and the favourites-first searches read the favourites.
// Loaded once per account; every change is applied at once and put back if the API refuses it.
import { SvelteSet } from 'svelte/reactivity';
import { gql } from '$lib/api';
import { FAVOURITE_EMOTE, HIDE_CREATOR, HIDE_EMOTE, PERSONAL_LISTS, UNFAVOURITE_EMOTE, UNHIDE_CREATOR, UNHIDE_EMOTE } from '$lib/queries';
import { session } from '$lib/tool/session.svelte';

/** What a filter needs from an emote: its id and, when known, its creator's. */
export type Hideable = { id: string; owner?: { id?: string } | null };

/** Keeps the emotes that neither hidden list covers. Pure, so it is tested. */
export function withoutHidden<T extends Hideable>(list: readonly T[], emotes: ReadonlySet<string>, creators: ReadonlySet<string>): T[] {
  if (!emotes.size && !creators.size) return list as T[];
  return list.filter((e) => !emotes.has(e.id) && !(e.owner?.id && creators.has(e.owner.id)));
}

/** Favourites first (in their own order), then the rest without them. Pure, so it is tested. */
export function favouritesFirst<T extends { id: string }>(favourites: readonly T[], rest: readonly T[], max: number): T[] {
  const seen = new Set(favourites.map((e) => e.id));
  return [...favourites, ...rest.filter((e) => !seen.has(e.id))].slice(0, max);
}

class Personal {
  readonly hiddenEmotes = new SvelteSet<string>();
  readonly hiddenCreators = new SvelteSet<string>();
  readonly favourites = new SvelteSet<string>();
  /** The lists for the signed-in account have arrived. */
  loaded = $state(false);
  #for = '';
  #loading: Promise<void> | null = null;

  /** Loads once per account; signed out, the lists are empty. */
  ensure() {
    const me = session.me?.id ?? '';
    if (me !== this.#for) {
      this.#for = me;
      this.hiddenEmotes.clear();
      this.hiddenCreators.clear();
      this.favourites.clear();
      this.loaded = false;
      this.#loading = null;
    }
    if (!me || this.loaded || this.#loading) return;
    this.#loading = this.refresh();
  }

  async refresh(): Promise<void> {
    const me = session.me?.id;
    if (!me) return;
    try {
      const r = await gql(PERSONAL_LISTS, {});
      if (session.me?.id !== me) return;
      replace(this.hiddenEmotes, r.hiddenEmotes.map((h) => h.emoteId));
      replace(this.hiddenCreators, r.hiddenCreators.map((h) => h.accountId));
      replace(this.favourites, r.favouriteEmoteIds);
      this.loaded = true;
    } catch {
      // Nothing hidden and no hearts until the next page view: the site still works.
    } finally {
      this.#loading = null;
    }
  }

  /** `list` without what you hid. */
  visible<T extends Hideable>(list: readonly T[]): T[] {
    return withoutHidden(list, this.hiddenEmotes, this.hiddenCreators);
  }

  hides(e: Hideable): boolean {
    return this.hiddenEmotes.has(e.id) || (!!e.owner?.id && this.hiddenCreators.has(e.owner.id));
  }

  hideEmote(id: string) {
    return flip(this.hiddenEmotes, id, true, () => gql(HIDE_EMOTE, { id }));
  }
  unhideEmote(id: string) {
    return flip(this.hiddenEmotes, id, false, () => gql(UNHIDE_EMOTE, { id }));
  }
  hideCreator(accountId: string) {
    return flip(this.hiddenCreators, accountId, true, () => gql(HIDE_CREATOR, { accountId }));
  }
  unhideCreator(accountId: string) {
    return flip(this.hiddenCreators, accountId, false, () => gql(UNHIDE_CREATOR, { accountId }));
  }
  favourite(id: string) {
    return flip(this.favourites, id, true, () => gql(FAVOURITE_EMOTE, { id }));
  }
  unfavourite(id: string) {
    return flip(this.favourites, id, false, () => gql(UNFAVOURITE_EMOTE, { id }));
  }
}

function replace(set: SvelteSet<string>, ids: string[]) {
  set.clear();
  for (const id of ids) set.add(id);
}

/** Changes the set at once, then asks the API; a refusal puts it back and is thrown. */
async function flip(set: SvelteSet<string>, id: string, on: boolean, call: () => Promise<unknown>) {
  const was = set.has(id);
  if (on) set.add(id);
  else set.delete(id);
  try {
    await call();
  } catch (e) {
    if (was) set.add(id);
    else set.delete(id);
    throw e;
  }
}

export const personal = new Personal();

/** What a refused change says, in one line. */
export function listError(e: unknown): string {
  const code = (e as { code?: string } | null)?.code;
  if (code === 'limit_reached' || code === 'rate_limited' || code === 'own_account') return (e as Error).message;
  if (code === 'not_found') return 'That emote isn’t there any more.';
  return e instanceof Error ? e.message : 'That didn’t work.';
}
