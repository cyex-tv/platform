import { describe, expect, it, vi } from 'vitest';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
import { loginHint, queryOf, searchable, step, topList, type Person } from './user-filter';
import { pastedAccount } from '$lib/admin/people';

const p = (id: string, count?: number): Person => ({ account: { id, displayName: id }, identities: [], ...(count === undefined ? {} : { count }) });

describe('user filter logic', () => {
  it('reads the typed text as the people search does', () => {
    expect(queryOf('  @@lumen_fox ')).toBe('lumen_fox');
    expect(queryOf('x'.repeat(120))).toHaveLength(100);
    expect(searchable('a')).toBe(false);
    expect(searchable('ab')).toBe(true);
  });

  it('puts the pinned people first, each once, five at most, with the count the list has', () => {
    const counted = [p('A', 9), p('B', 5), p('C', 3), p('D', 2), p('E', 1), p('F', 1)];
    const out = topList(counted, [p('C')]);
    expect(out.map((x) => x.account.id)).toEqual(['C', 'A', 'B', 'D', 'E']);
    expect(out[0].count).toBe(3);
  });

  it('uses the fallback only when nobody is counted', () => {
    expect(topList([], [p('O')], [p('E1'), p('O')]).map((x) => x.account.id)).toEqual(['O', 'E1']);
    expect(topList([p('A', 1)], [], [p('E1')]).map((x) => x.account.id)).toEqual(['A']);
  });

  it('steps through the rows and wraps', () => {
    expect(step(-1, 1, 3)).toBe(0);
    expect(step(-1, -1, 3)).toBe(2);
    expect(step(2, 1, 3)).toBe(0);
    expect(step(0, -1, 3)).toBe(2);
    expect(step(0, 1, 0)).toBe(-1);
  });

  it('names the first login and its platform', () => {
    expect(loginHint({ ...p('A'), identities: [{ platform: 'KICK', login: 'nora' }] })).toBe('nora on Kick');
    expect(loginHint(p('A'))).toBe('');
  });

  it('reads a pasted ID or admin link', () => {
    expect(pastedAccount('https://cyex.tv/admin/users/01jd8x0000000000000000meme/')).toBe('01JD8X0000000000000000MEME');
    expect(pastedAccount('nora')).toBeNull();
  });
});
