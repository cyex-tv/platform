import { beforeEach, describe, expect, it, vi } from 'vitest';

const gql = vi.fn();
vi.mock('$lib/api', () => ({ gql: (...a: unknown[]) => gql(...a) }));

const { loadActivity, loadEmoteActivity } = await import('./activity');

const entry = (id: string, kind = 'SET_EMOTE_ADDED') => ({
  id,
  kind,
  actor: { id: 'u', displayName: 'U', avatar: null },
  set: { id: 's', name: 'Main' },
  emote: null,
  fromName: null,
  toName: null,
  badge: null,
  paint: null,
  createdAt: '2026-11-17T12:00:00Z',
});

describe('activity paging', () => {
  beforeEach(() => gql.mockReset());

  it('hands back the cursor the API gave, not the last line’s id', async () => {
    gql.mockResolvedValueOnce({ accountActivityPage: { entries: [entry('a2'), entry('a1')], next: 'cursor-9' } });
    const page = await loadActivity('acc', null);
    expect(page.items.map((i) => i.id)).toEqual(['a2', 'a1']);
    expect(page.next).toBe('cursor-9');
    expect(gql.mock.calls[0][1]).toMatchObject({ accountId: 'acc', before: null });
  });

  it('is the end when there is no cursor, even for a short page', async () => {
    gql.mockResolvedValueOnce({ accountActivityPage: { entries: [entry('a1')], next: null } });
    expect((await loadActivity('acc', null)).next).toBeNull();
  });

  it('carries on from the cursor it is given', async () => {
    gql.mockResolvedValueOnce({ accountActivityPage: { entries: [entry('a0')], next: null } });
    await loadActivity('acc', 'cursor-9');
    expect(gql.mock.calls[0][1]).toMatchObject({ before: 'cursor-9' });
  });

  it('follows the cursor through pages the API skipped everything on', async () => {
    gql
      .mockResolvedValueOnce({ accountActivityPage: { entries: [], next: 'c1' } })
      .mockResolvedValueOnce({ accountActivityPage: { entries: [], next: 'c2' } })
      .mockResolvedValueOnce({ accountActivityPage: { entries: [entry('a3')], next: 'c3' } });
    const page = await loadActivity('acc', null);
    expect(page.items.map((i) => i.id)).toEqual(['a3']);
    expect(page.next).toBe('c3');
    expect(gql.mock.calls.map((c) => c[1].before)).toEqual([null, 'c1', 'c2']);
  });

  it('stops after a few empty rounds and leaves the cursor for “Load more”', async () => {
    gql.mockResolvedValue({ accountActivityPage: { entries: [], next: 'more' } });
    const page = await loadActivity('acc', null);
    expect(page.items).toEqual([]);
    expect(page.next).toBe('more');
    expect(gql).toHaveBeenCalledTimes(5);
  });

  it('drops kinds it does not know but keeps paging', async () => {
    gql.mockResolvedValueOnce({ accountActivityPage: { entries: [entry('x1', 'SOMETHING_NEW')], next: 'c1' } });
    gql.mockResolvedValueOnce({ accountActivityPage: { entries: [entry('x0')], next: null } });
    const page = await loadActivity('acc', null);
    expect(page.items.map((i) => i.id)).toEqual(['x0']);
    expect(page.next).toBeNull();
  });

  it('pages the emote feed the same way', async () => {
    gql.mockResolvedValueOnce({ emoteActivityPage: { entries: [entry('e2')], next: 'e2' } });
    const page = await loadEmoteActivity('emote', null);
    expect(page.items).toHaveLength(1);
    expect(page.next).toBe('e2');
    expect(gql.mock.calls[0][1]).toMatchObject({ emoteId: 'emote', before: null });
  });
});
