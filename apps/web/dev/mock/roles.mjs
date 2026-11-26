// Mock API: roles. See ../mock-api.mjs.
import { ME_ID, OTHERS } from './config.mjs';
import { MINE } from './emotes.mjs';
import { PENDING } from './review.mjs';
import { PERMISSIONS } from './editors.mjs';

// The built-in roles (ADR 0010): the owners' hidden top role, Admin and the User role everyone
// holds. The mock's signed-in admin is the owner: it holds the top role and sees it.
export const ROLES = [
  { id: 'ROLEROOT', name: 'Owner', description: 'Every permission. Held by the owners only and shown to nobody else.', position: 1000000, permissions: [], public: false, denies: [], invisible: true, builtIn: 'ROOT' },
  { id: 'ROLE1', name: 'Admin', description: 'Runs cyex.tv day to day.', position: 1000, permissions: ['roles.manage', 'users.view', 'users.restrict', 'audit.view', 'users.notes'], public: false, denies: [], invisible: false, builtIn: 'ADMIN' },
  { id: 'ROLE2', name: 'Moderator', description: 'Reviews uploads and reports.', position: 50, permissions: ['emotes.review', 'emotes.takedown', 'reports.manage', 'users.restrict_uploads', 'users.restrict_tickets'], public: true, denies: [], invisible: false, builtIn: null },
  { id: 'ROLE3', name: 'Artist', description: 'Makes emotes people love.', position: 10, permissions: [], public: true, denies: [], invisible: false, builtIn: null },
  { id: 'ROLE5', name: 'Muted', description: 'Can’t upload emotes or file reports. Sits above User, so its denies beat what everyone is allowed.', position: 5, permissions: [], denies: ['emotes.upload', 'reports.file'], public: false, invisible: true, builtIn: null },
  { id: 'ROLEUSER', name: 'User', description: 'Every account has this role. Its permissions are everyone’s.', position: 0, permissions: ['emotes.upload', 'reports.file'], public: false, denies: [], invisible: false, builtIn: 'USER' },
];
export const ACCOUNT_ROLES = { [OTHERS[0].id]: ['ROLE2', 'ROLE3'], [OTHERS[1].id]: ['ROLE3', 'ROLE5'], [ME_ID]: [] };
// A role as the signed-in owner sees it: the top role holds everything and can't be touched.
export const roleOut = (r) => {
  const root = r.builtIn === 'ROOT';
  const held = (ACCOUNT_ROLES[ME_ID] ?? []).includes(r.id);
  return { ...r, builtIn: r.builtIn ?? null, permissions: root ? PERMISSIONS : r.permissions, denies: r.denies ?? [], invisible: !!r.invisible,
    editable: !root && !held, assignable: !root && r.builtIn !== 'USER' };
};
// Effective permissions (the API's resolver): an account's roles from the highest position down,
// the User role last; the first role with an explicit allow or deny decides, a deny wins a tie,
// none means not granted. The mock's signed-in account is the owner: it holds everything.
export const decisionsFor = (accountId) => {
  const root = ROLES.find((r) => r.builtIn === 'ROOT');
  if (accountId === ME_ID) return PERMISSIONS.map((permission) => ({ permission, granted: true, effect: null, roleId: root.id, roleName: root.name, owner: true }));
  const held = (ACCOUNT_ROLES[accountId] ?? []).map((id) => ROLES.find((r) => r.id === id)).filter(Boolean);
  const user = ROLES.find((r) => r.builtIn === 'USER');
  const rank = [...held.filter((r) => r.builtIn !== 'USER'), user].sort((a, b) => b.position - a.position);
  return PERMISSIONS.map((permission) => {
    for (let i = 0; i < rank.length; ) {
      const tier = rank.filter((r) => r.position === rank[i].position);
      const deny = tier.find((r) => (r.denies ?? []).includes(permission));
      const allow = tier.find((r) => r.permissions.includes(permission));
      if (deny) return { permission, granted: false, effect: 'DENY', roleId: deny.id, roleName: deny.name, owner: false };
      if (allow) return { permission, granted: true, effect: 'ALLOW', roleId: allow.id, roleName: allow.name, owner: false };
      i += tier.length;
    }
    return { permission, granted: false, effect: null, roleId: null, roleName: null, owner: false };
  });
};
// Per-account limits (limits.rs): the defaults the API starts with, and each role's overrides
// (setRoleLimits). The User and top roles keep the defaults. MOCK_PENDING_MAX lowers the cap on
// emotes waiting for review, to see the upload form at it.
const MB = 1024 * 1024;
export const LIMIT_DEFAULTS = [
  ['gql_account', 'REQUESTS', 600, 60], ['account_search', 'REQUESTS', 60, 60], ['quick_search', 'REQUESTS', 240, 60], ['heavy_reads', 'REQUESTS', 120, 60],
  ['uploads_per_hour', 'UPLOADS', 20, 3600], ['pending_emotes', 'UPLOADS', Number(process.env.MOCK_PENDING_MAX) || 20, null],
  ['editors', 'ACCOUNT', 60, 3600], ['cosmetics', 'ACCOUNT', 300, 3600], ['set_clones', 'ACCOUNT', 30, 3600], ['personal_lists', 'ACCOUNT', 120, 60],
  ['emote_transfers', 'ACCOUNT', 60, 3600], ['prefs', 'ACCOUNT', 120, 3600], ['checkout', 'ACCOUNT', 30, 3600], ['redeem', 'ACCOUNT', 30, 3600], ['data_export', 'ACCOUNT', 5, 3600],
  // The most slots an account may give one of its channel sets (a cap, not a rate; ADR 0017 amendment 2026/11/22).
  ['set_capacity_max', 'ACCOUNT', 1000, null],
  // The files an account uploads (crates/media config): the configured numbers and the most a role may set
  // (badges and paints may be raised, up to the ceiling; emote rules are fixed and not listed).
  ['badge_max_bytes', 'MEDIA', 8 * MB, null, 16 * MB], ['badge_max_side', 'MEDIA', 2048, null, 4096], ['badge_max_frames', 'MEDIA', 120, null, 240],
  ['paint_max_bytes', 'MEDIA', 8 * MB, null, 16 * MB], ['paint_max_side', 'MEDIA', 4096, null, 6144],
];
/** The most a role may set a limit to (the API's `max`). */
export const limitMax = (key) => LIMIT_DEFAULTS.find((l) => l[0] === key)?.[4] ?? 1000000;
export const ROLE_LIMITS = { ROLE2: { pending_emotes: 100 } };
export const limitsOut = (o = {}) => LIMIT_DEFAULTS.map(([key, group, d, w, max = 1000000]) => ({ key, group, default: d, max, windowSeconds: w, value: o[key] ?? null }));
/** An account's limit: the most generous override among its roles, else the default. */
export const limitOf = (accountId, key) => {
  const set = (ACCOUNT_ROLES[accountId] ?? []).map((r) => ROLE_LIMITS[r]?.[key]).filter((n) => n != null);
  return set.length ? Math.max(...set) : LIMIT_DEFAULTS.find((l) => l[0] === key)[2];
};
// `limits` (public_limits.rs): the numbers the API enforces. `account` lists the signed-in
// account's own per-account limits with its roles' overrides; empty signed out and on REST.
const kind = (o) => ({ maxDurationMs: null, minSide: null, maxAspect: null, maxFps: null, ...o });
export const MEDIA_LIMITS = {
  emote: kind({ maxBytes: 8 * MB, maxSide: 1000, maxFrames: 1000, maxFps: 50, maxTotalPixels: 1_000_000_000, maxAspect: 4, maxServedSide: 128 }),
  badge: kind({ maxBytes: 8 * MB, maxSide: 2048, maxFrames: 120, maxTotalPixels: 64_000_000, maxDurationMs: 10_000, minSide: 72, maxAspect: 1, maxServedSide: 512 }),
  paint: kind({ maxBytes: 8 * MB, maxSide: 4096, maxFrames: 120, maxTotalPixels: 64_000_000, maxDurationMs: 10_000, minSide: 64, maxAspect: 8, maxServedSide: 512 }),
};
const RATES = [['auth', 30, 60], ['gql', 300, 60], ['gql_account', 600, 60], ['upload', 30, 3600], ['reads', 1200, 60], ['socket', 60, 60]];
const snake = (o) => Object.fromEntries(Object.entries(o).map(([k, v]) => [k.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`), v]));
export const publicLimitsRest = () => ({
  emote: snake(MEDIA_LIMITS.emote),
  badge: snake(MEDIA_LIMITS.badge),
  paint: snake(MEDIA_LIMITS.paint),
  names: { emote: { min: 1, max: 100 }, set: { min: 1, max: 100 }, badge: { min: 1, max: 32 }, paint: { min: 1, max: 32 }, tooltip: { min: 0, max: 64 } },
  tags: { max: 10, max_len: 32 },
  rates: RATES.map(([key, count, window]) => ({ key, count, window_seconds: window })),
  account: [],
});
export const limitsAnswer = (session) => {
  const r = publicLimitsRest();
  return {
    ...MEDIA_LIMITS,
    names: r.names,
    tags: { max: 10, maxLen: 32 },
    rates: r.rates.map(({ key, count, window_seconds }) => ({ key, count, windowSeconds: window_seconds })),
    account: session ? LIMIT_DEFAULTS.map(([key, , d, w]) => ({ key, default: d, effective: limitOf(ME_ID, key), windowSeconds: w })) : [],
  };
};
export const pendingOf = (accountId) => [...MINE, ...PENDING].filter((e) => e.owner?.id === accountId && e.state === 'PENDING').length;
export const rolesOf = (accountId) => [
  ...(accountId === ME_ID ? [ROLES[0]] : []),
  ...(ACCOUNT_ROLES[accountId] ?? []).map((id) => ROLES.find((r) => r.id === id)).filter(Boolean),
].map(roleOut);
if (process.env.MOCK_LONG === '1') {
  for (const n of ['Founder', 'Translator of the Many Tongues', 'Beta tester', 'Community helper', 'Event host', 'Supporter', 'Emote reviewer', 'Contest winner'])
    { const id = 'ROLEX' + ROLES.length; ROLES.push({ id, name: n, description: null, position: 5 + ROLES.length, permissions: [], public: true }); ACCOUNT_ROLES[OTHERS[0].id].push(id); }
}
