// What the public part of a profile shows, as pure helpers (tested).
import type { Platform } from '$lib/gql/graphql';

/** The platform's own page for a login, to open in a new tab. Null for a login that is not a safe path piece. */
export function connectionUrl(platform: Platform, login: string): string | null {
  const l = login.trim();
  if (!/^[A-Za-z0-9_.-]{1,64}$/.test(l)) return null;
  return platform === 'KICK' ? `https://kick.com/${encodeURIComponent(l.toLowerCase())}` : `https://www.twitch.tv/${encodeURIComponent(l.toLowerCase())}`;
}
