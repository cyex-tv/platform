// The account menu's places (2026/11/22): Account (with Editors) is a tab on your profile; the
// Settings row covers only /me/settings.
import { describe, expect, it } from 'vitest';
import { SETTINGS, menuLinks } from './account';

describe('account menu links', () => {
  it('lists Your emotes, Cosmetics and Account, all on your profile', () => {
    const links = menuLinks('a1');
    expect(links.map((l) => l.label)).toEqual(['Your emotes', 'Cosmetics', 'Account']);
    const account = links.find((l) => l.label === 'Account')!;
    expect(account.href).toBe('/users/a1?s=account');
    expect(account.match('/users/a1', new URLSearchParams('s=account'))).toBe(true);
    expect(account.match('/users/a1', new URLSearchParams('s=editors'))).toBe(true);
    expect(account.match('/users/a1', new URLSearchParams('s=sets'))).toBe(false);
  });

  it('marks Settings on /me/settings pages only', () => {
    expect(SETTINGS.href).toBe('/me/settings');
    expect(SETTINGS.match('/me/settings/appearance')).toBe(true);
    expect(SETTINGS.match('/me/editors')).toBe(false);
    expect(SETTINGS.match('/me/account')).toBe(false);
  });
});
