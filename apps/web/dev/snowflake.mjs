// Public ids as the real API shows them (ADR: snowflake ids): emotes, accounts and emote sets
// carry a Snowflake, a 64-bit number written as a decimal string:
//   id = ((ms - EPOCH) << 22) | (instance << 12) | seq      (41 bits of ms since 2026-01-01, 10 of instance, 12 of sequence)
// Everything else (badges, paints, roles, orders, images, audit rows, notifications ...) keeps its ULID.
// The mock's data is built with ULID-shaped ids; this module turns the ones of those three kinds into
// snowflakes on the way out (`outward`) and any snowflake or ULID back into the ULID on the way in
// (`inward`), so every input accepts either form, as the API does during the transition.
export const EPOCH_MS = 1767225600000;
const SEED_BASE = Date.UTC(2026, 10, 1); // seed entities get times before this, so ids are the same on every start

export const snowflake = (ms, instance = 1, seq = 0) =>
  ((BigInt(Math.max(0, Math.floor(ms) - EPOCH_MS)) << 22n) | (BigInt(instance & 0x3ff) << 12n) | BigInt(seq & 0xfff)).toString();
export const snowflakeMs = (id) => Number(BigInt(id) >> 22n) + EPOCH_MS;
export const isSnowflake = (s) => typeof s === 'string' && /^[1-9]\d{0,18}$/.test(s) && BigInt(s) <= 0x7fffffffffffffffn;

// Which ULID-shaped mock ids are emotes, accounts or sets.
const EMOTE = /^01JD8X(\d{20})(?:-big\d+)?$/;
const ACCOUNT = /^01JD8X(?:0{16}[0-9A-Z]{4}|0{14}ED\d{4}|ACC\d{17})$/;
const SET = /^01JD8XSET[0-9A-Z]{14,}$/;
export const kindOfLegacy = (s) =>
  typeof s !== 'string' || s.length > 40 ? null : EMOTE.test(s) ? 'emote' : ACCOUNT.test(s) ? 'account' : SET.test(s) ? 'set' : null;

function hash(s) {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h >>> 0;
}
const DAY = 86400_000;
const INSTANCE = { emote: 1, account: 2, set: 3 };

const toSnow = new Map();
const toLegacy = new Map();
let lastLive = 0;
let liveSeq = 0;

// Entities the mock makes while it runs (uploads, new and cloned sets) are "new now"; the rest are
// spread over the weeks before SEED_BASE, emotes in step with their index like their created_at.
function timeFor(legacy, kind) {
  if (kind === 'emote') {
    const m = EMOTE.exec(legacy);
    const i = Number(m[1]);
    if (i >= 700 && i < 800 && !legacy.includes('-big')) return null; // the mock's uploads (POST /v1/emotes)
    return SEED_BASE - i * 0.2 * DAY - (legacy.includes('-big') ? hash(legacy) % 3600_000 : 0);
  }
  if (/^01JD8XSET(?:NEW|CPY)/.test(legacy)) return null;
  return SEED_BASE - (kind === 'account' ? 60 : 30) * DAY + (hash(legacy) % (20 * DAY));
}

export function outwardId(legacy) {
  const known = toSnow.get(legacy);
  if (known) return known;
  const kind = kindOfLegacy(legacy);
  if (!kind) return legacy;
  let ms = timeFor(legacy, kind);
  let seq = hash(legacy) & 0xfff;
  if (ms === null) {
    ms = Date.now();
    liveSeq = ms === lastLive ? liveSeq + 1 : 0;
    lastLive = ms;
    seq = liveSeq;
  }
  let id = snowflake(ms, INSTANCE[kind], seq);
  while (toLegacy.has(id)) id = (BigInt(id) + 1n).toString();
  toSnow.set(legacy, id);
  toLegacy.set(id, legacy);
  return id;
}
// A snowflake the mock handed out becomes the ULID its data is keyed by; anything else (a ULID, a name) is unchanged.
export const inwardId = (s) => (typeof s === 'string' ? toLegacy.get(s) ?? s : s);

const mapKeyed = (x, fn) => {
  if (typeof x === 'string') {
    if (x[0] === '{' && x[x.length - 1] === '}') {
      // A JSON string (notification payloads, audit before/after) with ids inside.
      try {
        const back = JSON.stringify(mapKeyed(JSON.parse(x), fn));
        return back;
      } catch {
        return x;
      }
    }
    return fn(x);
  }
  if (Array.isArray(x)) return x.map((v) => mapKeyed(v, fn));
  if (x && typeof x === 'object') return Object.fromEntries(Object.entries(x).map(([k, v]) => [fn(k), mapKeyed(v, fn)]));
  return x;
};
export const outward = (body) => mapKeyed(body, outwardId);
export const inward = (vars) => mapKeyed(vars, inwardId);

// Hand out the ids of everything already built, so a snowflake typed into the address bar after the
// mock restarted still resolves.
export function registerAll(...roots) {
  const seen = new Set();
  const walk = (x) => {
    if (typeof x === 'string') return void outwardId(x);
    if (!x || typeof x !== 'object' || seen.has(x)) return;
    seen.add(x);
    for (const v of Array.isArray(x) ? x : Object.values(x)) walk(v);
  };
  roots.forEach(walk);
}
