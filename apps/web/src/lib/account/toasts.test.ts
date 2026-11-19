import { describe, expect, it } from 'vitest';
import { fresh } from './toasts';

const n = (id: string, at: string, read = false) => ({ id, createdAt: at, read });
const since = Date.parse('2026-11-19T10:00:00Z');

describe('fresh notifications', () => {
  it('only unread, unseen and newer than the page', () => {
    const items = [n('a', '2026-11-19T10:05:00Z'), n('b', '2026-11-19T10:04:00Z', true), n('c', '2026-11-19T10:03:00Z'), n('d', '2026-11-19T09:00:00Z')];
    expect(fresh(items, new Set(['c']), since).map((x) => x.id)).toEqual(['a']);
  });
  it('a bad date never pops up', () => {
    expect(fresh([n('x', 'not a date')], new Set(), since)).toEqual([]);
  });
});
