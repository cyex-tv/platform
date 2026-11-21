import { describe, expect, it } from 'vitest';
import { countLogins } from './grant-logins';

describe('countLogins', () => {
  it('counts logins, with prefixes and @, once each', () => {
    const c = countLogins('nora\nkick:lumen_fox, @Nora twitch:cat_jammer');
    expect(c.logins).toEqual(['nora', 'kick:lumen_fox', 'twitch:cat_jammer']);
    expect(c.valid).toBe(3);
    expect(c.odd).toEqual([]);
  });
  it('names entries that cannot be a login', () => {
    const c = countLogins('nora https://x.example/a? bad!name');
    expect(c.valid).toBe(1);
    expect(c.odd).toEqual(['https://x.example/a?', 'bad!name']);
  });
  it('counts what is over the batch', () => {
    const c = countLogins('a b c d', 3);
    expect(c.logins).toHaveLength(3);
    expect(c.extra).toBe(1);
  });
  it('is empty for nothing', () => {
    expect(countLogins('  \n ')).toEqual({ logins: [], valid: 0, odd: [], extra: 0 });
  });
});
