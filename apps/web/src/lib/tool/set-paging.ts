// How the set editor reads a set: a page at a time (a set can hold a thousand emotes).

/** Emotes drawn per page, as on the public set page. */
export const SET_PAGE = 120;

/** How many pages of `SET_PAGE` cover `loaded` emotes (at least one): what a re-read fetches to keep the screen as it is. */
export function pagesFor(loaded: number): number {
  return Math.max(1, Math.ceil(loaded / SET_PAGE));
}

/**
 * The pages a "Load more" asks for, given the last page read (`read`) and how many emotes were
 * removed since: each removal pulls the emotes after it one place up, so the one that is now
 * last on the page read (and not on screen) sits earlier. Counting the pages from what is on
 * screen instead (`items.length`) asks for the same page again after a removal.
 */
export function pagesAfter(read: number, removed: number): number[] {
  const first = Math.floor(Math.max(0, read * SET_PAGE - removed) / SET_PAGE) + 1;
  return Array.from({ length: read + 1 - first + 1 }, (_, i) => first + i);
}

/** Who is in a set: the ids and the names the Add panel checks against. */
export type Members = { ids: Set<string>; names: Set<string> };

/** A change the editor made itself, so the member lists follow it without being read again. */
export type MemberChange =
  | { kind: 'add'; id: string; name: string }
  | { kind: 'remove'; id: string; name: string }
  | { kind: 'rename'; id: string; from: string; to: string };

/** The members after `c`. */
export function applyChange(m: Members, c: MemberChange): Members {
  const ids = new Set(m.ids);
  const names = new Set(m.names);
  if (c.kind === 'add') {
    ids.add(c.id);
    names.add(c.name);
  } else if (c.kind === 'remove') {
    ids.delete(c.id);
    names.delete(c.name);
  } else {
    names.delete(c.from);
    names.add(c.to);
  }
  return { ids, names };
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

/** A read that was slow: the lists as it found them, then every change made here since it began. */
export function replayChanges(read: Members, changes: readonly MemberChange[]): Members {
  return changes.reduce(applyChange, read);
}
