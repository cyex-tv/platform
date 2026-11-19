import { describe, expect, it } from 'vitest';
import { canChooseName, canDisconnect, isSettingsPath, readMotion, settingsRail } from './settings';

const one = [{ platform: 'TWITCH' }];
const two = [{ platform: 'TWITCH' }, { platform: 'KICK' }];

describe('settings', () => {
  it('lists Account, Appearance, Editors and Billing, the current one marked', () => {
    const rows = settingsRail('/me/settings/appearance')[0].items;
    expect(rows.map((r) => r.label)).toEqual(['Account', 'Appearance', 'Editors', 'Billing']);
    expect(rows.filter((r) => r.current).map((r) => r.label)).toEqual(['Appearance']);
    expect(settingsRail('/me/editors')[0].items.find((r) => r.current)?.href).toBe('/me/editors');
  });
  it('treats /me/settings and the old /me/account as Account', () => {
    for (const p of ['/me/settings', '/me/account']) expect(settingsRail(p)[0].items.find((r) => r.current)?.label).toBe('Account');
    expect(isSettingsPath('/me/account')).toBe(true);
    expect(isSettingsPath('/me/editors')).toBe(true);
    expect(isSettingsPath('/me/settings/billing')).toBe(true);
    expect(isSettingsPath('/me/sets')).toBe(false);
    expect(isSettingsPath('/me/settingsx')).toBe(false);
  });
  it('offers the name choice and Disconnect only with more than one connection', () => {
    expect(canChooseName(one)).toBe(false);
    expect(canDisconnect(one)).toBe(false);
    expect(canChooseName(two)).toBe(true);
    expect(canDisconnect(two)).toBe(true);
  });
  it('reads the motion setting, System for anything unknown', () => {
    expect(readMotion('reduce')).toBe('reduce');
    expect(readMotion('full')).toBe('full');
    expect(readMotion(null)).toBe('system');
    expect(readMotion('yes')).toBe('system');
  });
});
