import { describe, expect, it } from 'vitest';
import type { SetMenuQuery } from '$lib/gql/graphql';
import { menuRights, NO_RIGHTS } from './set-menu';

const ME = 'ME';
const OWNER = 'OWNER';
const range = { min: 1, max: 1000, ownerMax: 1000, used: 0, staff: false };
const ch = (owner: string, id: string, active: string | null = null) => ({
  platform: 'TWITCH' as const, platformChannelId: id, login: `login${id}`, activeSetId: active, account: { id: owner },
});
const data = (over: Partial<SetMenuQuery> = {}): SetMenuQuery => ({
  setCapacityRange: range, editorOf: [], myChannels: [ch(ME, '1')], editorChannels: [ch(OWNER, '2', 'S1')], ...over,
}) as SetMenuQuery;
const grant = (permissions: string[], allSets = true, setIds: string[] = []) =>
  ({ account: { id: OWNER }, permissions, allSets, setIds }) as SetMenuQuery['editorOf'][number];

describe('set menu rights', () => {
  it('gives the owner everything, with their own channels', () => {
    const r = menuRights({ id: 'S0', name: 'Main', kind: 'CHANNEL', ownerId: ME }, data(), ME, false);
    expect(r).toMatchObject({ rename: true, capacity: true, remove: true, edit: true });
    expect(r.channels.map((c) => c.platformChannelId)).toEqual(['1']);
  });

  it('lets an editor with Sets manage the set and pick it on the owner’s channels without Channel settings', () => {
    const set = { id: 'S1', name: 'Theirs', kind: 'CHANNEL', ownerId: OWNER };
    const r = menuRights(set, data({ editorOf: [grant(['SETS'])] }), ME, false);
    expect(r.remove).toBe(true);
    expect(r.channels).toEqual([{ platform: 'TWITCH', platformChannelId: '2', login: 'login2', active: true }]);
  });

  it('lets Channel settings alone pick the active set, but not delete', () => {
    const set = { id: 'S1', name: 'Theirs', kind: 'CHANNEL', ownerId: OWNER };
    const r = menuRights(set, data({ setCapacityRange: null, editorOf: [grant(['CHANNELS'])] }), ME, false);
    expect(r).toMatchObject({ rename: false, capacity: false, remove: false, edit: false });
    expect(r.channels).toHaveLength(1);
  });

  it('gives nothing outside a grant’s sets, and no channels for a personal set', () => {
    const outside = menuRights({ id: 'S9', name: 'x', kind: 'CHANNEL', ownerId: OWNER }, data({ setCapacityRange: null, editorOf: [grant(['SETS'], false, ['S1'])] }), ME, false);
    expect(outside).toMatchObject({ remove: false, channels: [] });
    const personal = menuRights({ id: 'P', name: 'p', kind: 'PERSONAL', ownerId: ME }, data(), ME, false);
    expect(personal.channels).toEqual([]);
  });

  it('knows nothing signed out or before the answer', () => {
    expect(menuRights({ id: 'S0', name: 'Main', kind: 'CHANNEL', ownerId: ME }, null, ME, false)).toBe(NO_RIGHTS);
    expect(menuRights({ id: 'S0', name: 'Main', kind: 'CHANNEL', ownerId: ME }, data(), null, false)).toBe(NO_RIGHTS);
  });

  it('lets staff delete any set', () => {
    const r = menuRights({ id: 'S5', name: 'x', kind: 'CHANNEL', ownerId: 'SOMEONE' }, data(), ME, true);
    expect(r.remove).toBe(true);
    expect(r.channels).toEqual([]);
  });
});
