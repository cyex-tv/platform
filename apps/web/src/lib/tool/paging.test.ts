import { describe, expect, it } from 'vitest';
import { appendUnique, pageToken } from './paging';

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
