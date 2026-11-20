// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const gql = vi.fn();
vi.mock('$lib/api', () => ({ gql: (...a: unknown[]) => gql(...a) }));

const { forgetInbox, inbox, loadInbox, markRead, refreshCount, watchInbox } = await import('./inbox.svelte');
const { session } = await import('$lib/tool/session.svelte');

const note = (id: string, read = false) => ({ id, kind: 'x', payload: '{}', createdAt: '', read });
/** Answers by the query's text, like the API would. */
function api(handlers: { count?: () => unknown; list?: () => unknown; mark?: () => unknown }) {
  gql.mockImplementation(async (doc: unknown) => {
    const text = String(doc);
    if (text.includes('unreadNotificationCount')) return handlers.count?.();
    if (text.includes('myNotifications')) return handlers.list?.();
    return handlers.mark?.();
  });
}

beforeEach(() => {
  gql.mockReset();
  forgetInbox();
  session.me = null;
});
afterEach(() => vi.useRealTimers());

describe('the unread count', () => {
  it('is known once asked, and a failure keeps the number it had', async () => {
    api({ count: () => ({ unreadNotificationCount: 4 }) });
    await refreshCount();
    expect([inbox.count, inbox.known]).toEqual([4, true]);
    gql.mockRejectedValue(new Error('down'));
    await refreshCount();
    expect([inbox.count, inbox.known]).toEqual([4, true]);
  });
});

describe('the list', () => {
  it('loads newest first and counts the unread', async () => {
    api({ list: () => ({ myNotifications: [note('a'), note('b', true), note('c')] }) });
    await loadInbox();
    expect(inbox.items?.map((n) => n.id)).toEqual(['a', 'b', 'c']);
    expect([inbox.count, inbox.known, inbox.loading, inbox.error]).toEqual([2, true, false, null]);
  });

  it('keeps the old list on a failure and says why', async () => {
    api({ list: () => ({ myNotifications: [note('a')] }) });
    await loadInbox();
    gql.mockRejectedValue(new Error('offline'));
    await loadInbox();
    expect(inbox.items).toHaveLength(1);
    expect(inbox.error).toBe('offline');
    expect(inbox.loading).toBe(false);
  });
});

describe('marking read', () => {
  it('marks some in the list and recounts', async () => {
    api({ list: () => ({ myNotifications: [note('a'), note('b'), note('c')] }), mark: () => ({}) });
    await loadInbox();
    expect(await markRead(['a', 'b'])).toBe(true);
    expect(inbox.items?.map((n) => n.read)).toEqual([true, true, false]);
    expect(inbox.count).toBe(1);
  });

  it('marks all, and a second press while it is on its way sends nothing', async () => {
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    api({ list: () => ({ myNotifications: [note('a'), note('b')] }), mark: () => gate });
    await loadInbox();
    const first = markRead();
    expect(inbox.marking).toBe(true);
    expect(await markRead()).toBe(false);
    release();
    expect(await first).toBe(true);
    expect(inbox.marking).toBe(false);
    expect(inbox.count).toBe(0);
    expect(gql.mock.calls.filter(([d]) => !String(d).includes('myNotifications'))).toHaveLength(1);
  });

  it('a failed mark leaves everything as it was', async () => {
    api({ list: () => ({ myNotifications: [note('a')] }) });
    await loadInbox();
    gql.mockRejectedValue(new Error('nope'));
    expect(await markRead(['a'])).toBe(false);
    expect(inbox.items?.[0].read).toBe(false);
    expect(inbox.count).toBe(1);
    expect(inbox.marking).toBe(false);
  });

  it('with no list loaded, asks for the count instead', async () => {
    api({ count: () => ({ unreadNotificationCount: 0 }), mark: () => ({}) });
    expect(await markRead(['a'])).toBe(true);
    expect(gql.mock.calls.some(([d]) => String(d).includes('unreadNotificationCount'))).toBe(true);
  });
});

describe('watching', () => {
  const counts = () => gql.mock.calls.filter(([d]) => String(d).includes('unreadNotificationCount')).length;
  const setVisibility = (state: 'visible' | 'hidden') => {
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => state });
    document.dispatchEvent(new Event('visibilitychange'));
  };

  it('asks now, then every minute while signed in and visible', async () => {
    vi.useFakeTimers();
    api({ count: () => ({ unreadNotificationCount: 1 }) });
    session.me = { id: 'A' } as typeof session.me;
    setVisibility('visible');
    const stop = watchInbox();
    expect(counts()).toBe(1);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(counts()).toBe(2);
    await vi.advanceTimersByTimeAsync(120_000);
    expect(counts()).toBe(4);
    stop();
  });

  it('skips a tick while the tab is hidden or signed out, and asks again when the tab comes back', async () => {
    vi.useFakeTimers();
    api({ count: () => ({ unreadNotificationCount: 1 }) });
    session.me = { id: 'A' } as typeof session.me;
    setVisibility('hidden');
    const stop = watchInbox();
    expect(counts()).toBe(1);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(counts()).toBe(1);
    setVisibility('visible');
    await vi.advanceTimersByTimeAsync(0);
    expect(counts()).toBe(2);
    session.me = null;
    await vi.advanceTimersByTimeAsync(60_000);
    expect(counts()).toBe(2);
    stop();
  });

  it('stops for good when cleaned up', async () => {
    vi.useFakeTimers();
    api({ count: () => ({ unreadNotificationCount: 1 }) });
    session.me = { id: 'A' } as typeof session.me;
    setVisibility('visible');
    const stop = watchInbox();
    stop();
    await vi.advanceTimersByTimeAsync(300_000);
    setVisibility('visible');
    expect(counts()).toBe(1);
  });
});

describe('forgetting', () => {
  it('clears the count and the list (signing out)', async () => {
    api({ list: () => ({ myNotifications: [note('a')] }) });
    await loadInbox();
    forgetInbox();
    expect([inbox.count, inbox.known, inbox.items, inbox.error]).toEqual([0, false, null, null]);
  });
});
