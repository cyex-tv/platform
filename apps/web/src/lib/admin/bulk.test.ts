import { describe, expect, it } from 'vitest';
import { batchSummary, loginProblem, parseLogins } from './bulk';

describe('batch summaries', () => {
  it('says how many it did', () => {
    expect(batchSummary({ done: ['a', 'b'], failed: [] }, 'approved')).toBe('2 uploads approved.');
    expect(batchSummary({ done: ['a'], failed: [] }, 'rejected')).toBe('1 upload rejected.');
    expect(batchSummary({ done: [], failed: [] }, 'approved')).toBe('Nothing approved.');
  });
  it('names what it left alone, grouped by why', () => {
    const failed = [
      { id: 'x', code: 'not_pending', message: '' },
      { id: 'y', code: 'not_pending', message: '' },
      { id: 'z', code: 'not_found', message: '' },
    ];
    expect(batchSummary({ done: ['a'], failed }, 'approved')).toBe('1 upload approved. 2 were already reviewed and 1 is gone.');
    expect(batchSummary({ done: [], failed: [{ id: 'r', code: 'not_open', message: '' }] }, 'dismissed', ['report', 'reports'])).toBe(
      'Nothing dismissed. 1 was already closed.',
    );
    expect(batchSummary({ done: ['a'], failed: [{ id: 'q', code: 'odd', message: '' }] }, 'approved')).toBe('1 upload approved. 1 didn’t go through.');
  });
});

describe('pasted logins', () => {
  it('splits on lines, commas and spaces, once each, in order', () => {
    expect(parseLogins(' alice\nBob, @alice ;kick:carol\n\n bob  twitch:dave ').logins).toEqual(['alice', 'Bob', 'kick:carol', 'twitch:dave']);
    expect(parseLogins('').logins).toEqual([]);
  });
  it('keeps at most the cap and counts the rest', () => {
    const text = Array.from({ length: 105 }, (_, i) => `u${i}`).join('\n');
    const r = parseLogins(text);
    expect(r.logins).toHaveLength(100);
    expect(r.extra).toBe(5);
  });
  it('says why a login has no account', () => {
    expect(loginProblem('not_found')).toBe('No account with that login');
    expect(loginProblem('ambiguous')).toMatch(/twitch:name/);
    expect(loginProblem('bad_login')).toBe('Not a login');
    expect(loginProblem(null)).toBe('');
  });
});
