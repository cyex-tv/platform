// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

const gql = vi.fn();
vi.mock('$lib/api', () => ({ gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));

const { forgetPrefs, loadPrefs, notePrefs } = await import('./prefs.svelte');

const pref = (kind: string) => ({ kind, notify: true, toast: true, required: false });
function later<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

beforeEach(() => {
  gql.mockReset();
  forgetPrefs();
});

describe('loading the prefs', () => {
  it('drops an answer that lands after signing out', async () => {
    const slow = later<unknown>();
    gql.mockReturnValueOnce(slow.promise);
    const loading = loadPrefs();
    forgetPrefs();
    slow.resolve({ myNotificationPrefs: [pref('editor.added')] });
    await loading;
    expect(notePrefs.items).toBeNull();
  });

  it('lets only the newest of two loads win', async () => {
    const first = later<unknown>();
    const second = later<unknown>();
    gql.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const a = loadPrefs(true);
    const b = loadPrefs(true);
    second.resolve({ myNotificationPrefs: [pref('new')] });
    await b;
    first.resolve({ myNotificationPrefs: [pref('old')] });
    await a;
    expect(notePrefs.items?.map((p) => p.kind)).toEqual(['new']);
  });
});
