import { beforeEach, describe, expect, it, vi } from 'vitest';

const gql = vi.fn();
vi.mock('$lib/api', () => ({ gql: (...a: unknown[]) => gql(...a) }));

const { forgetMe, isAdmin, loadMe, session } = await import('./session.svelte');

const me = (id: string, permissions: string[] = []) => ({ me: { id, createdAt: '', permissions, identities: [] } });

describe('loadMe', () => {
  beforeEach(() => {
    forgetMe();
    gql.mockReset();
  });

  it('shares one request between callers', async () => {
    gql.mockResolvedValue(me('a'));
    const [x, y] = await Promise.all([loadMe(), loadMe()]);
    await loadMe();
    expect(gql).toHaveBeenCalledTimes(1);
    expect(x).toBe(y);
    expect(session.me?.id).toBe('a');
  });

  it('asks again when fresh, and the fresh answer wins', async () => {
    gql.mockResolvedValueOnce(me('a')).mockResolvedValueOnce(me('b'));
    await loadMe();
    expect((await loadMe({ fresh: true })).id).toBe('b');
    expect(gql).toHaveBeenCalledTimes(2);
    expect(session.me?.id).toBe('b');
  });

  it("doesn't keep a failure", async () => {
    gql.mockRejectedValueOnce(new Error('down')).mockResolvedValueOnce(me('a'));
    await expect(loadMe()).rejects.toThrow('down');
    expect((await loadMe()).id).toBe('a');
    expect(gql).toHaveBeenCalledTimes(2);
  });

  it('forgets after sign-out', async () => {
    gql.mockResolvedValue(me('a'));
    await loadMe();
    forgetMe();
    expect(session.me).toBeNull();
    await loadMe();
    expect(gql).toHaveBeenCalledTimes(2);
  });
});

describe('isAdmin', () => {
  it('is true for any admin permission', () => {
    expect(isAdmin(null)).toBe(false);
    expect(isAdmin({ permissions: ['emotes.upload'] })).toBe(false);
    expect(isAdmin({ permissions: ['audit.view'] })).toBe(true);
  });
});
