import { describe, expect, it } from 'vitest';
import type { RoleLimitFieldsFragment } from '$lib/gql/graphql';
import { amount, changed, grouped, info, per, toInput, toValues, valueProblem } from './role-limits';

const l = (key: string, group: RoleLimitFieldsFragment['group'], value: number | null = null, windowSeconds: number | null = 60): RoleLimitFieldsFragment => ({
  key, group, default: 20, windowSeconds, value,
});

describe('role limits', () => {
  it('groups the limits as Requests, Uploads, Account actions, in that order', () => {
    const g = grouped([l('prefs', 'ACCOUNT'), l('pending_emotes', 'UPLOADS', null, null), l('gql_account', 'REQUESTS')]);
    expect(g.map((x) => x.label)).toEqual(['Requests', 'Uploads', 'Account actions']);
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
    expect(amount(600, 60)).toBe('600 a minute');
    expect(amount(1200, 3600)).toBe('1.200 an hour');
    expect(amount(20, null)).toBe('20 at once');
    expect(info('pending_emotes').label).toBe('Waiting for review');
    expect(info('some_new_limit').label).toBe('some new limit');
  });
});
