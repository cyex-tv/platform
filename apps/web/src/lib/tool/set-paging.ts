// How the set editor reads a set: a page at a time (a set can hold a thousand emotes).

/** Emotes drawn per page, as on the public set page. */
export const SET_PAGE = 120;

/** How many pages of `SET_PAGE` cover `loaded` emotes (at least one): what a re-read fetches to keep the screen as it is. */
export function pagesFor(loaded: number): number {
  return Math.max(1, Math.ceil(loaded / SET_PAGE));
}

/** The page a "Load more" asks for next, given how many emotes are on screen. */
export function nextPage(loaded: number): number {
  return Math.floor(loaded / SET_PAGE) + 1;
}

/** Adds a page to the list without repeating an emote that is on screen already (emotes added meanwhile shift the pages). */
export function appendPage<T extends { emote: { id: string } }>(items: readonly T[], page: readonly T[]): T[] {
  const have = new Set(items.map((m) => m.emote.id));
  return [...items, ...page.filter((m) => !have.has(m.emote.id))];
}

/** Every page after the first that `total` emotes need, with `per` to a page. */
export function morePages(total: number, per: number): number[] {
  const pages = Math.ceil(total / per);
  return Array.from({ length: Math.max(0, pages - 1) }, (_, i) => i + 2);
}
