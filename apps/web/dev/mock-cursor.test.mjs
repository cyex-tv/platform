// The mock's cursor paging (ADR 0030): `next` walks a list to its end, page by page, as the API's
// does; `page` still works; a cursor the mock didn't give out is `bad_cursor`.
import { describe, expect, it } from 'vitest';
import { badCursor, pageOf } from './mock/cursor.mjs';

const list = Array.from({ length: 23 }, (_, i) => ({ id: `E${i}` }));

describe('mock cursor paging', () => {
  it('walks every row once by next, and ends with null', () => {
    const seen = [];
    let after = null;
    for (let i = 0; i < 10; i++) {
      const p = pageOf(list, { perPage: 5, after });
      seen.push(...p.items.map((e) => e.id));
      if (!p.next) break;
      after = p.next;
    }
    expect(seen).toEqual(list.map((e) => e.id));
  });

  it('numbered pages still work, the cap holds, and after wins over page', () => {
    expect(pageOf(list, { page: 2, perPage: 5 }).items[0].id).toBe('E5');
    expect(pageOf(list, { perPage: 500 }, 50, 10).perPage).toBe(10);
    const first = pageOf(list, { perPage: 5 });
    expect(pageOf(list, { perPage: 5, page: 4, after: first.next }).items[0].id).toBe('E5');
  });

  it('a made-up cursor is bad_cursor', () => {
    expect(pageOf(list, { after: 'junk' })).toBeNull();
    expect(badCursor().errors[0].extensions.code).toBe('bad_cursor');
  });
});
