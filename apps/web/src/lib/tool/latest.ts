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
