import { describe, expect, it } from 'vitest';
import { canManage, grantOn, grantable, ordered, permissionLabels, PERMISSIONS } from './editors';

describe('account editor permissions', () => {
  it('lists every permission once with a one-line explanation', () => {
    expect(PERMISSIONS.map((p) => p.key)).toEqual(['EMOTES', 'SETS', 'EDITORS', 'PROFILE']);
    for (const p of PERMISSIONS) expect(p.line.length).toBeLessThan(80);
  });

  it('names permissions in order', () => {
    expect(ordered(['EDITORS', 'EMOTES', 'EMOTES'])).toEqual(['EMOTES', 'EDITORS']);
    expect(permissionLabels(['SETS', 'EMOTES'])).toBe('Emotes and Sets');
    expect(permissionLabels(['PROFILE'])).toBe('Profile');
    expect(permissionLabels([])).toBe('Nothing');
    expect(permissionLabels(['EMOTES', 'SETS', 'EDITORS'])).toBe('Emotes, Sets and Editors');
  });

  it('gives the owner everything and an editor their grant', () => {
    const grants = [{ account: { id: 'A' }, permissions: ['EMOTES', 'EDITORS'] as const }];
    expect(grantOn(grants, 'ME', 'ME').size).toBe(4);
    expect([...grantOn(grants, 'A', 'ME')]).toEqual(['EMOTES', 'EDITORS']);
    expect(grantOn(grants, 'B', 'ME').size).toBe(0);
    expect(grantOn(grants, null, 'ME').size).toBe(0);
  });

  it('never offers more than you hold', () => {
    const held = new Set(['EMOTES', 'EDITORS'] as const);
    expect(grantable(held).map((p) => p.key)).toEqual(['EMOTES', 'EDITORS']);
    expect(canManage(held, ['EMOTES'])).toBe(true);
    expect(canManage(held, ['EMOTES', 'SETS'])).toBe(false);
    expect(canManage(new Set(['EMOTES'] as const), ['EMOTES'])).toBe(false);
  });
});
