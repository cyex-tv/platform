import { describe, expect, it } from 'vitest';
import { elsewhere } from './error-links';

describe('error page links', () => {
  const hrefs = (p: string) => elsewhere(p).map((w) => w.href);

  it('puts the closest place first', () => {
    expect(hrefs('/shop/old-item')[0]).toBe('/shop');
    expect(hrefs('/redeem/x')[0]).toBe('/shop');
    expect(hrefs('/developers/v0')[0]).toBe('/help');
    expect(hrefs('/emotes/123')[0]).toBe('/search');
    expect(hrefs('/no-such-page')[0]).toBe('/search');
  });

  it('lists each place once and never the front page (the problem links there)', () => {
    for (const p of ['/shop/x', '/help/x', '/users/x', '/zzz']) {
      const h = hrefs(p);
      expect(new Set(h).size).toBe(h.length);
      expect(h).toHaveLength(4);
      expect(h).not.toContain('/');
    }
  });
});
