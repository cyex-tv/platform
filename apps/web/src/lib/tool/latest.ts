/**
 * One save at a time where the last choice wins: a pick made while a save is on its way is kept
 * (a later pick replaces an earlier one) and saved when that one lands, unless it is by then the
 * saved value (`done`). A failed save stops the queue and rejects, so the caller can say so.
 */
export function latestWins<T>(save: (v: T) => Promise<void>, done: (v: T) => boolean): (v: T) => Promise<void> {
  let busy = false;
  let wanted: { v: T } | null = null;
  return async (v) => {
    if (busy) {
      wanted = { v };
      return;
    }
    busy = true;
    let next: { v: T } | null = { v };
    try {
      while (next && !done(next.v)) {
        wanted = null;
        await save(next.v);
        next = wanted;
      }
    } finally {
      busy = false;
      wanted = null;
    }
  };
}

/**
 * `latestWins` per key (the id of the thing being saved): a pick belongs to the key it was made
 * for, so it is saved there even if the page has since moved on to another one, and picks for
 * different keys never replace each other.
 */
export function latestWinsBy<T>(save: (key: string, v: T) => Promise<void>, done: (key: string, v: T) => boolean): (key: string, v: T) => Promise<void> {
  const by = new Map<string, (v: T) => Promise<void>>();
  return (key, v) => {
    let one = by.get(key);
    if (!one) {
      one = latestWins<T>((x) => save(key, x), (x) => done(key, x));
      by.set(key, one);
    }
    return one(v);
  };
}
