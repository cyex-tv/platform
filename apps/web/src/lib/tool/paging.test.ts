import { describe, expect, it } from 'vitest';
import { appendUnique, pageToken, totalAfterPage } from './paging';

describe('appendUnique', () => {
  it('drops rows already shown and keeps order', () => {
    const out = appendUnique([{ id: 'a' }, { id: 'b' }], [{ id: 'b' }, { id: 'c' }, { id: 'c' }]);
    expect(out.map((r) => r.id)).toEqual(['a', 'b', 'c']);
  });
});

describe('pageToken', () => {
  it('marks an older request stale after a reset or a newer request', () => {
    const t = pageToken();
    const a = t.begin();
    expect(a()).toBe(true);
    t.reset();
    expect(a()).toBe(false);
    const b = t.begin();
    const c = t.begin();
    expect(b()).toBe(false);
    expect(c()).toBe(true);
  });
});

describe('totalAfterPage', () => {
  it('ends the list on an empty later page, however high the count said', () => {
    expect(totalAfterPage(60, 0, 75, false)).toBe(60);
    expect(totalAfterPage(60, 10, 75, false)).toBe(75);
    expect(totalAfterPage(0, 0, 0, true)).toBe(0);
  });
});
