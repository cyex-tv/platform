import { describe, expect, it } from 'vitest';
import { markCurrent } from './rail';

const groups = [
  { caption: 'Library', items: [{ href: '/me', label: 'Your emotes' }, { href: '/me/sets', label: 'Sets' }] },
  { caption: 'Account', items: [{ href: '/me/account', label: 'Account' }] },
];
const current = (path: string, exact: string[] = []) =>
  markCurrent(groups, path, exact).flatMap((g) => g.items).filter((i) => i.current).map((i) => i.href);

describe('markCurrent', () => {
  it('marks the longest matching prefix only', () => {
    expect(current('/me/sets/abc')).toEqual(['/me/sets']);
    expect(current('/me')).toEqual(['/me']);
    expect(current('/me/emotes/1')).toEqual(['/me']);
  });
  it('honours exact hrefs', () => {
    expect(current('/me/emotes/1', ['/me'])).toEqual([]);
    expect(current('/me', ['/me'])).toEqual(['/me']);
  });
  it('does not match a sibling that shares a prefix', () => {
    expect(current('/me/settings')).toEqual(['/me']);
    expect(current('/other')).toEqual([]);
  });
});
