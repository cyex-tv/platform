// Public ids. Emotes, accounts and emote sets are identified by Snowflakes: 64-bit numbers written as
// decimal strings ("1314628736410763264"), kept as strings everywhere (never a JS number: they do
// not fit). Everything else (badges, paints, roles, orders, images, audit rows, notifications)
// keeps its 26-character ULID. Until further notice the API still accepts a ULID wherever an
// emote, account or set id is taken, so old links keep working; the site sends those to the new
// address with `legacyRedirect`.

/** A Snowflake: 1 to 19 digits that fit in a signed 64-bit integer. */
export const isSnowflake = (s: string): boolean => /^[1-9]\d{0,18}$/.test(s) && BigInt(s) <= 0x7fffffffffffffffn;

/** A ULID: 26 characters of Crockford base32 (read leniently: any letter or digit, as the API does). */
export const isUlid = (s: string): boolean => /^[0-9A-Za-z]{26}$/.test(s);

/**
 * A Snowflake typed by a person: 15 to 19 digits, no leading zero. Today's are 18 or 19 digits;
 * Twitch and Kick user ids (up to about 11 digits) are not cyex ids, so a search by one stays a search.
 */
export const isTypedSnowflake = (s: string): boolean => /^[1-9]\d{14,18}$/.test(s) && isSnowflake(s);

/** Either kind of id, as typed into a search or lookup field. */
export const looksLikeId = (s: string): boolean => isTypedSnowflake(s) || isUlid(s);

export type IdRoute = 'emotes' | 'users' | 'sets';

/**
 * Where an old link goes: `/<route>/<snowflake>` plus the query string, or null when it stays.
 * `param` is what the address says, `found` the id of the thing the API found for it. A redirect
 * needs all of: the address uses a ULID, something was found, and its own id is a Snowflake.
 * Anything else (a Snowflake in the address, nothing found, an API still answering ULIDs) stays.
 */
export function legacyRedirect(route: IdRoute, param: string, found: string | null | undefined, search = ''): string | null {
  if (!found || found === param || !isUlid(param) || !isSnowflake(found)) return null;
  return `/${route}/${encodeURIComponent(found)}${search}`;
}
