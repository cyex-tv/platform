// Mock API: config. See ../mock-api.mjs.
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';


export const PORT = Number(process.env.PORT ?? 8787);
export const ORIGIN = process.env.SITE_ORIGIN ?? 'http://localhost:5173';
export const BASE = `http://localhost:${PORT}`;
export const OPEN_ENV = process.env.MOCK_OPEN === '1';
export const ADMIN_ENV = process.env.MOCK_ADMIN === '1' || OPEN_ENV;
export const FACES = process.env.MOCK_FACES === '1';
// MOCK_PADDLE=1: checkout leaves orders PENDING with a client token (for Paddle.js, or a stand-in for it in tests).
export const PADDLE_MOCK = process.env.MOCK_PADDLE === '1';
export const PADDLE_DELAY = Number(process.env.MOCK_PADDLE_DELAY ?? 5000);

export const ME_ID = '01JD8X0000000000000000MEME';
const OWNER_DEFS = [
  ['lumen_fox', ME_ID], ['quietoak', '01JD8X0000000000000000OAK1'], ['nora_byte', '01JD8X0000000000000000NORA'],
  ['pixelmoth'], ['byteNod'], ['sunnyfern'], ['mossgate'], ['kiwi_rae'], ['tidalwren'], ['emberlynx'], ['sleepyhollis'], ['orbit_jules'],
];
export const OWNERS = OWNER_DEFS.map(([displayName, id], n) => ({
  id: id ?? `01JD8X0000000000000000U${String(n).padStart(3, '0')}`, displayName, avatar: `${BASE}/avatars/${n}.svg`,
}));
// MOCK_LONG=1: a very long name and an unbreakable one, to look at how names and role chips wrap.
if (process.env.MOCK_LONG === '1') {
  OWNERS[1].displayName = 'Quietoak the Wandering Bard of the Northern Reaches and Keeper of Many Small Emotes';
  OWNERS[2].displayName = 'N'.repeat(60);
}
export const ME_SUMMARY = OWNERS[0];
export const OTHERS = OWNERS.slice(1);

// Real chat linked to the mock, for the extension's local beta (see ./extension.mjs): numeric ids
// from the environment or the gitignored apps/web/dev/.env.mock.
const FILE = join(dirname(fileURLToPath(import.meta.url)), '..', '.env.mock');
const fileEnv = existsSync(FILE)
  ? Object.fromEntries(readFileSync(FILE, 'utf8').split(/\r?\n/).map((l) => l.trim()).filter((l) => l && !l.startsWith('#') && l.includes('='))
      .map((l) => [l.slice(0, l.indexOf('=')).trim(), l.slice(l.indexOf('=') + 1).trim()]))
  : {};

function idOf(name) {
  const v = (process.env[name] ?? fileEnv[name] ?? '').trim();
  if (!v) return null;
  if (!/^\d{1,20}$/.test(v)) throw new Error(`${name} must be a numeric platform id, not "${v}"`);
  return v;
}

/** The real channel and chatter ids linked to the mock, per platform. */
export const LINKED = {
  TWITCH: { channel: idOf('MOCK_TWITCH_CHANNEL'), user: idOf('MOCK_TWITCH_USER') },
  KICK: { channel: idOf('MOCK_KICK_CHANNEL'), user: idOf('MOCK_KICK_USER') },
};

/** The mock account behind a linked chatter id, else undefined. */
export const linkedChatter = (platform, id) => (LINKED[platform]?.user === id ? ME_ID : undefined);
