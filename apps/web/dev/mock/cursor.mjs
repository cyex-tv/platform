// Mock API: cursor paging (ADR 0030). See ../mock-api.mjs.
// The API's cursors are opaque positions; the mock's are the offset the next page starts at,
// which the site never reads either. `page` still works, and `after` wins over it, as in the API.

/** The error the API answers for a cursor it didn't give out. */
export const badCursor = () => ({
  data: null,
  errors: [{ message: 'This cursor isn’t one this list gave out (or the list’s filters changed). Start again from the first page.', extensions: { code: 'bad_cursor' } }],
});

/**
 * One page of `all` for `v` (`page`, `perPage`, `after`): its rows, the page and size as answered, and
 * `next`, where the following page starts (null on the last). Null for a bad cursor.
 */
export function pageOf(all, v, def = 50, max = 100) {
  const perPage = Math.min(max, Math.max(1, v.perPage ?? def));
  const page = Math.max(1, v.page ?? 1);
  let from = (page - 1) * perPage;
  if (v.after) {
    const m = /^mock-(\d+)$/.exec(String(v.after));
    if (!m) return null;
    from = Number(m[1]);
  }
  const items = all.slice(from, from + perPage);
  const next = from + perPage < all.length ? `mock-${from + perPage}` : null;
  return { items, page, perPage, next };
}
