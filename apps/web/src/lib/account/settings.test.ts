import { describe, expect, it } from 'vitest';
import { canChooseName, canDisconnect, isSettingsPath, movedToProfile, readMotion, settingsRail, SETTINGS_HOME } from './settings';

const one = [{ platform: 'TWITCH' }];
const two = [{ platform: 'TWITCH' }, { platform: 'KICK' }];

describe('settings', () => {
  it('lists Appearance, Notifications, Hidden and Billing (Account and Editors moved to the profile), the current one marked', () => {
    const rows = settingsRail('/me/settings/hidden')[0].items;
    expect(rows.map((r) => r.label)).toEqual(['Appearance', 'Notifications', 'Hidden', 'Billing']);
    expect(rows.filter((r) => r.current).map((r) => r.label)).toEqual(['Hidden']);
  });
  it('opens /me/settings on Appearance', () => {
    expect(SETTINGS_HOME).toBe('/me/settings/appearance');
    expect(settingsRail('/me/settings')[0].items.find((r) => r.current)?.label).toBe('Appearance');
  });
  it('sends the old Account and Editors addresses to the profile tabs, the hash kept', () => {
    expect(movedToProfile('/me/settings/account', 'a1')).toBe('/users/a1?s=account');
    expect(movedToProfile('/me/account', 'a1', '#conn-h')).toBe('/users/a1?s=account#conn-h');
    expect(movedToProfile('/me/editors', 'a1')).toBe('/users/a1?s=editors');
    expect(movedToProfile('/me/editors/', 'a 1')).toBe('/users/a%201?s=editors');
    // Notifications is a profile tab too: its filter and pick come along, nothing else does.
    expect(movedToProfile('/me/notifications', 'a1')).toBe('/users/a1?s=notifications');
    expect(movedToProfile('/me/notifications', 'a1', '', '?f=editors&n=N%201&x=1')).toBe('/users/a1?s=notifications&f=editors&n=N%201');
    expect(movedToProfile('/me/settings/notifications', 'a1')).toBeNull();
    for (const p of ['/me/settings', '/me/settings/appearance', '/me/settings/billing', '/me/accounts', '/me/editorsx', '/me/sets']) expect(movedToProfile(p, 'a1')).toBeNull();
  });
  it('knows which pages are Settings', () => {
    expect(isSettingsPath('/me/account')).toBe(false);
    expect(isSettingsPath('/me/editors')).toBe(false);
    expect(isSettingsPath('/me/settings/account')).toBe(false);
    expect(isSettingsPath('/me/settings')).toBe(true);
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
