import { describe, expect, it } from 'vitest';
import { hasSubscription, headerLinks, sortChannels, type ChannelRole } from './nav';

const labels = (me: { permissions: string[] } | null, admin: boolean) => headerLinks(me, admin).map((l) => l.label);

describe('headerLinks', () => {
  it('signed out: Emotes only, as before', () => {
    expect(labels(null, false)).toEqual(['Emotes']);
    expect(labels(null, true)).toEqual(['Emotes']); // never Admin without an account
  });
  it('signed in: Emotes and Store', () => {
    expect(labels({ permissions: [] }, false)).toEqual(['Emotes', 'Store']);
  });
  it('staff also get Admin, last', () => {
    expect(labels({ permissions: ['emotes.review'] }, true)).toEqual(['Emotes', 'Store', 'Admin']);
  });
  it('no subscriptions exist yet: Subscription never shows', () => {
    expect(hasSubscription({ permissions: [] })).toBe(false);
    expect(hasSubscription(null)).toBe(false);
    expect(labels({ permissions: [] }, true)).not.toContain('Subscription');
  });
  it('links point at the right places and mark their sections', () => {
    const [emotes, store, admin] = headerLinks({ permissions: [] }, true);
    expect([emotes.href, store.href, admin.href]).toEqual(['/search', '/shop', '/admin']);
    expect(store.match('/shop')).toBe(true);
    expect(store.match('/shopping')).toBe(false);
    expect(admin.match('/admin/users')).toBe(true);
    expect(emotes.match('/emotes/abc')).toBe(true);
  });
});

const ch = (role: ChannelRole | string, login: string, platform = 'TWITCH') => ({ role: role as ChannelRole, channel: { login, platform } });
const order = (list: ReturnType<typeof ch>[]) => sortChannels(list).map((c) => `${c.role}:${c.channel.login}:${c.channel.platform}`);

describe('sortChannels', () => {
  it('orders by role: owner, account editor, set editor', () => {
    expect(order([ch('SET_EDITOR', 'a'), ch('ACCOUNT_EDITOR', 'b'), ch('OWNER', 'z')])).toEqual([
      'OWNER:z:TWITCH',
      'ACCOUNT_EDITOR:b:TWITCH',
      'SET_EDITOR:a:TWITCH',
    ]);
  });
  it('alphabetical inside a role, ignoring case', () => {
    expect(order([ch('ACCOUNT_EDITOR', 'quietoak'), ch('ACCOUNT_EDITOR', 'Ana'), ch('ACCOUNT_EDITOR', 'bob')])).toEqual([
      'ACCOUNT_EDITOR:Ana:TWITCH',
      'ACCOUNT_EDITOR:bob:TWITCH',
      'ACCOUNT_EDITOR:quietoak:TWITCH',
    ]);
  });
  it('the same login on two platforms: Kick then Twitch', () => {
    expect(order([ch('OWNER', 'lumen', 'TWITCH'), ch('OWNER', 'lumen', 'KICK')])).toEqual(['OWNER:lumen:KICK', 'OWNER:lumen:TWITCH']);
  });
  it('an unknown role goes last and nothing is mutated', () => {
    const list = [ch('SOMETHING_NEW', 'a'), ch('SET_EDITOR', 'b')];
    expect(order(list)).toEqual(['SET_EDITOR:b:TWITCH', 'SOMETHING_NEW:a:TWITCH']);
    expect(list[0].channel.login).toBe('a');
  });
});
