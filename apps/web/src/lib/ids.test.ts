import { describe, expect, it } from 'vitest';
import { isSnowflake, isUlid, legacyRedirect, looksLikeId } from './ids';

const SNOW = '1314628736410763264';
const ULID = '01JD8X0000000000000000MEME';

describe('ids', () => {
  it('knows a snowflake: digits that fit in a signed 64-bit integer', () => {
    expect(isSnowflake(SNOW)).toBe(true);
    expect(isSnowflake('1')).toBe(true);
    expect(isSnowflake('9223372036854775807')).toBe(true);
    expect(isSnowflake('9223372036854775808')).toBe(false);
    expect(isSnowflake('12345678901234567890')).toBe(false);
    expect(isSnowflake('0')).toBe(false);
    expect(isSnowflake('012')).toBe(false);
    expect(isSnowflake('')).toBe(false);
    expect(isSnowflake('12a')).toBe(false);
    expect(isSnowflake(ULID)).toBe(false);
  });
  it('knows a ULID and either', () => {
    expect(isUlid(ULID)).toBe(true);
    expect(isUlid(SNOW)).toBe(false);
    expect(isUlid('01JD8X0000000000000000MEM')).toBe(false);
    expect(looksLikeId(SNOW)).toBe(true);
    expect(looksLikeId(ULID)).toBe(true);
    expect(looksLikeId('lumen_fox')).toBe(false);
    expect(looksLikeId('123456789')).toBe(false); // a Twitch user id
  });
});

describe('legacyRedirect', () => {
  it('sends a ULID address to the snowflake one and keeps the query', () => {
    expect(legacyRedirect('emotes', ULID, SNOW, '?tab=channels')).toBe(`/emotes/${SNOW}?tab=channels`);
    expect(legacyRedirect('users', ULID, SNOW)).toBe(`/users/${SNOW}`);
    expect(legacyRedirect('sets', ULID, SNOW, '?a=1&b=2')).toBe(`/sets/${SNOW}?a=1&b=2`);
  });
  it('leaves a snowflake address alone', () => {
    expect(legacyRedirect('emotes', SNOW, SNOW, '?tab=x')).toBeNull();
  });
  it('leaves an unknown id to the not-found page', () => {
    expect(legacyRedirect('emotes', ULID, null)).toBeNull();
    expect(legacyRedirect('emotes', ULID, undefined)).toBeNull();
  });
  it('does not redirect while the API still answers with the ULID, or for other kinds of address', () => {
    expect(legacyRedirect('emotes', ULID, ULID)).toBeNull();
    expect(legacyRedirect('emotes', ULID, 'not-an-id')).toBeNull();
    expect(legacyRedirect('users', 'lumen_fox', SNOW)).toBeNull();
  });
});
