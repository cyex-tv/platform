import { describe, expect, it } from 'vitest';
import type { RoleLimitFieldsFragment } from '$lib/gql/graphql';
import { LIMIT_INFO, amount, changed, grouped, info, overrides, per, toInput, toValues, valueProblem } from './role-limits';

const l = (key: string, group: RoleLimitFieldsFragment['group'], value: number | null = null, windowSeconds: number | null = 60): RoleLimitFieldsFragment => ({
  key, group, default: 20, max: 1_000_000, windowSeconds, value,
});

describe('role limits', () => {
  it('groups the limits as Requests, Uploads, File sizes, Account actions, in that order', () => {
    const g = grouped([l('prefs', 'ACCOUNT'), l('badge_max_side', 'MEDIA', null, null), l('pending_emotes', 'UPLOADS', null, null), l('gql_account', 'REQUESTS')]);
    expect(g.map((x) => x.label)).toEqual(['Requests', 'Uploads', 'File sizes', 'Account actions']);
    expect(grouped([l('prefs', 'ACCOUNT')]).map((x) => x.label)).toEqual(['Account actions']);
  });

  it('an empty box keeps the default; a number from 1 to a million sets it', () => {
    expect(valueProblem('')).toBeNull();
    expect(valueProblem('  ')).toBeNull();
    expect(valueProblem('25')).toBeNull();
    expect(valueProblem('1000000')).toBeNull();
    expect(valueProblem('0')).toMatch(/^1 to/);
    expect(valueProblem('1000001')).toMatch(/^1 to/);
    expect(valueProblem('2.5')).toMatch(/whole number/);
    expect(valueProblem('-3')).toMatch(/whole number/);
    expect(valueProblem('lots')).toMatch(/whole number/);
  });

  it('sends only the boxes with a number, and knows when nothing changed', () => {
    const before = toValues([l('pending_emotes', 'UPLOADS', 100, null), l('prefs', 'ACCOUNT')]);
    expect(before).toEqual({ pending_emotes: '100', prefs: '' });
    expect(toInput({ ...before, prefs: ' 7 ', redeem: 'x' })).toEqual([{ key: 'pending_emotes', value: 100 }, { key: 'prefs', value: 7 }]);
    expect(changed(before, { ...before })).toBe(false);
    expect(changed(before, { ...before, pending_emotes: ' 100' })).toBe(false);
    expect(changed(before, { ...before, pending_emotes: '' })).toBe(true);
    expect(changed(before, { ...before, prefs: '3' })).toBe(true);
  });

  it('says what a limit allows in words', () => {
    expect(per(60)).toBe('a minute');
    expect(per(3600)).toBe('an hour');
    expect(per(30)).toBe('per 30 seconds');
    expect(per(null)).toBeNull();
    expect(amount(l('gql_account', 'REQUESTS'), 600)).toBe('600 a minute');
    expect(amount(l('checkout', 'ACCOUNT', null, 3600), 1200)).toBe('1.200 an hour');
    expect(amount(l('pending_emotes', 'UPLOADS', null, null), 20)).toBe('20 at once');
    expect(amount(l('badge_max_side', 'MEDIA', null, null), 4096)).toBe('4.096 px');
    expect(amount(l('paint_max_bytes', 'MEDIA', null, null), 8 * 1024 * 1024)).toBe('8 MB');
    expect(amount(l('paint_max_bytes', 'MEDIA', null, null), 2.5 * 1024 * 1024)).toBe('2,5 MB');
    // Emote rules are fixed for everyone: no emote limit is described here.
    expect(Object.keys(LIMIT_INFO).filter((k) => k.startsWith('emote_max_'))).toEqual([]);
    expect(info('pending_emotes').label).toBe('Waiting for review');
    expect(info('some_new_limit').label).toBe('some new limit');
  });

  it('file sizes are typed in MB and sent as bytes, every limit up to its own most', () => {
    const bytes = { key: 'badge_max_bytes', max: 16 * 1024 * 1024 };
    expect(valueProblem('12', bytes)).toBeNull();
    expect(valueProblem('2,5', bytes)).toBeNull();
    expect(valueProblem('2.5', bytes)).toBeNull();
    expect(valueProblem('16', bytes)).toBeNull();
    expect(valueProblem('17', bytes)).toBe('0,01 to 16.');
    expect(valueProblem('lots', bytes)).toMatch(/number of MB/);
    const side = { key: 'badge_max_side', max: 1000 };
    expect(valueProblem('1000', side)).toBeNull();
    expect(valueProblem('1001', side)).toBe('1 to 1.000.');
    expect(valueProblem('0', side)).toBe('1 to 1.000.');
    const refs = [bytes, side];
    const before = toValues([l('badge_max_bytes', 'MEDIA', 12 * 1024 * 1024, null), l('badge_max_side', 'MEDIA')]);
    expect(before).toEqual({ badge_max_bytes: '12', badge_max_side: '' });
    expect(toInput({ badge_max_bytes: '2,5', badge_max_side: '512' }, refs)).toEqual([
      { key: 'badge_max_bytes', value: 2.5 * 1024 * 1024 },
      { key: 'badge_max_side', value: 512 },
    ]);
    // Past its own most a box isn't sent.
    expect(toInput({ badge_max_side: '1001' }, refs)).toEqual([]);
    expect(changed(before, { ...before, badge_max_bytes: '12,0' }, refs)).toBe(false);
    expect(changed(before, { ...before, badge_max_side: '256' }, refs)).toBe(true);
  });

  it('lists only the limits a role overrides', () => {
    const got = overrides([l('pending_emotes', 'UPLOADS', 100, null), l('prefs', 'ACCOUNT')]);
    expect(got.map((x) => x.key)).toEqual(['pending_emotes']);
  });
});
