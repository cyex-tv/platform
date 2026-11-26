// Mock API: the public reads the browser extension makes (GET /v1/sets/global, GET /v1/sets/{id},
// GET /v1/channels/{platform}/{id}, POST /v1/personal-sets/lookup), so a local beta of the extension
// (`CYEX_TV_API=http://localhost:8787 npm run build:beta` in the extension repo) shows the mock's
// emotes, badge and paint in real chat.
//
// Linking real chat to the mock: set these to numeric platform ids, in the environment or in
// `apps/web/dev/.env.mock` (KEY=VALUE lines; the file is gitignored, so your ids stay local):
//   MOCK_TWITCH_CHANNEL / MOCK_KICK_CHANNEL  a channel whose chat gets the mock's "Main" set
//   MOCK_TWITCH_USER / MOCK_KICK_USER        a chatter shown as the mock account (its badge, paint
//                                            and personal set)
import { LINKED, ME_ID, ME_SUMMARY, linkedChatter } from './config.mjs';
import { CHANNELS } from './account.mjs';
import { MINE, member } from './emotes.mjs';
import { GLOBAL, SETS } from './set-data.mjs';
import { findSet } from './sets.mjs';
import { SHARED_SETS } from './editors.mjs';
import { inwardId } from '../snowflake.mjs';

/** What is linked, for the start-up line ("twitch channel", "kick chatter"); never the ids. */
export const linkedNote = () => Object.entries(LINKED).flatMap(([p, l]) => [l.channel && `${p.toLowerCase()} channel`, l.user && `${p.toLowerCase()} chatter`]).filter(Boolean);

// The mock account's personal set: the one made on the site if there is one, else a small built-in one.
const BUILT_IN_PERSONAL = { id: '01JD8XSET00000000000PERSNL', name: 'Personal', kind: 'PERSONAL', capacity: 10, owner: ME_SUMMARY,
  emotes: MINE.slice(0, 4).map((e) => member(e)), visibility: 'LISTED', createdAt: '2026-11-10T12:10:00.000Z' };
const myPersonal = () => [...SETS, ...SHARED_SETS].find((s) => s.kind === 'PERSONAL' && s.owner?.id === ME_ID) ?? BUILT_IN_PERSONAL;

const setBody = (s) => ({ id: s.id, name: s.name, kind: s.kind, capacity: s.capacity, emotes: s.emotes, createdAt: s.createdAt,
  owner: s.owner ?? null, visibility: s.visibility ?? 'LISTED', listed: (s.visibility ?? 'LISTED') === 'LISTED' });

function channelOf(platform, id) {
  const known = CHANNELS.find((c) => c.platform === platform && c.platformChannelId === id);
  if (known) return known;
  if (LINKED[platform]?.channel === id) return { ...CHANNELS[0], platform, platformChannelId: id };
  return null;
}

/**
 * Answers the extension's reads; null when the request isn't one of them.
 * @returns {{ status: number, body: unknown } | null}
 */
export function extensionRead(method, pathname, body) {
  if (method === 'GET' && pathname === '/v1/sets/global') return { status: 200, body: setBody(GLOBAL) };
  let m = pathname.match(/^\/v1\/sets\/([^/]+)$/);
  if (method === 'GET' && m) {
    const id = inwardId(decodeURIComponent(m[1]));
    const s = id === BUILT_IN_PERSONAL.id ? BUILT_IN_PERSONAL : findSet(id);
    return s && s.visibility !== 'PRIVATE' ? { status: 200, body: setBody(s) } : { status: 404, body: { error: 'not_found', message: 'No such set.' } };
  }
  m = pathname.match(/^\/v1\/channels\/(twitch|kick)\/([^/]+)$/);
  if (method === 'GET' && m) {
    const c = channelOf(m[1].toUpperCase(), decodeURIComponent(m[2]));
    if (!c) return { status: 404, body: { error: 'not_found', message: 'That channel isn’t on cyex.tv.' } };
    return { status: 200, body: { platform: c.platform.toLowerCase(), platformChannelId: c.platformChannelId, login: c.login, account: c.account,
      activeSetId: c.activeSetId ?? null, allowPersonalEmotes: c.allowPersonalEmotes } };
  }
  if (method === 'POST' && pathname === '/v1/personal-sets/lookup') {
    const platform = typeof body?.platform === 'string' ? body.platform.toUpperCase() : '';
    const ids = Array.isArray(body?.user_ids) ? body.user_ids : null;
    if (!['TWITCH', 'KICK'].includes(platform) || !ids || !ids.every((i) => typeof i === 'string')) {
      return { status: 400, body: { error: 'bad_request', message: 'Send a platform and user_ids.' } };
    }
    if (ids.length > 100) return { status: 400, body: { error: 'too_many', message: 'Look up at most 100 users at once.' } };
    const mine = [...new Set(ids)].filter((id) => linkedChatter(platform, id) || (platform === 'TWITCH' && id === CHANNELS[0].platformChannelId));
    return { status: 200, body: { sets: mine.map((id) => ({ platformUserId: id, set: setBody(myPersonal()) })) } };
  }
  return null;
}
