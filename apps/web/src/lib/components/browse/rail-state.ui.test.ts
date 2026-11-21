import { beforeEach, describe, expect, it } from 'vitest';
import { loadShut, railKey, saveShut } from './rail-state';

const SNOW = '1314628736410763264';
const OLD_ME = '01JD8X0000000000000000MEME';
const OLD_NORA = '01JD8X0000000000000000NORA';
const NORA = '1314628736410763265';

beforeEach(() => localStorage.clear());

describe('remembered rail groups', () => {
  it('round-trips for a snowflake account', () => {
    saveShut(SNOW, ['mine', NORA]);
    expect([...loadShut(SNOW)!].sort()).toEqual(['mine', NORA].sort());
  });

  it('drops what was kept under an old ULID account, and the key itself', () => {
    localStorage.setItem(railKey(OLD_ME), JSON.stringify(['mine']));
    expect(loadShut(SNOW)).toBeNull();
    expect(localStorage.getItem(railKey(OLD_ME))).toBeNull();
  });

  it('keeps the groups that still mean something and drops old-account groups', () => {
    localStorage.setItem(railKey(SNOW), JSON.stringify(['mine', OLD_NORA, NORA]));
    expect([...loadShut(SNOW)!].sort()).toEqual(['mine', NORA].sort());
  });

  it('falls back to the default when only old groups were remembered', () => {
    localStorage.setItem(railKey(SNOW), JSON.stringify([OLD_NORA]));
    expect(loadShut(SNOW)).toBeNull();
  });

  it('leaves everything alone while the account is still a ULID', () => {
    localStorage.setItem(railKey(OLD_ME), JSON.stringify(['mine', OLD_NORA]));
    expect([...loadShut(OLD_ME)!].sort()).toEqual(['mine', OLD_NORA].sort());
  });

  it('survives junk and blocked storage', () => {
    localStorage.setItem(railKey(SNOW), '{nope');
    expect(loadShut(SNOW)).toBeNull();
    localStorage.setItem(railKey(SNOW), '"x"');
    expect(loadShut(SNOW)).toBeNull();
  });
});
