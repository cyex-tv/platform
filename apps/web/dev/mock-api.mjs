// A stand-in for api.cyex.tv, for looking at the site without the real stack:
//   node dev/mock-api.mjs            (listens on localhost:8787)
//   PUBLIC_API_URL=http://localhost:8787 npm run dev
// It answers the site's GraphQL operations with made-up emotes and draws their pictures as
// simple shapes. "Signing in" sets a mock cookie; everything signed in lives in memory and
// resets when it restarts. MOCK_ADMIN=1, or opening /mock/admin once, gives the mock account
// every permission (/mock/admin?off clears it).
// Development only; nothing here ships.
import { createServer } from 'node:http';

const PORT = Number(process.env.PORT ?? 8787);
const ORIGIN = process.env.SITE_ORIGIN ?? 'http://localhost:5173';
const BASE = `http://localhost:${PORT}`;
const ADMIN_ENV = process.env.MOCK_ADMIN === '1';

const NAMES = ['catJAM', 'pogHop', 'quietoak', 'lumenWave', 'noraWow', 'pixelmoth', 'byteNod', 'foxLook',
  'oakCry', 'moonHi', 'sipTea', 'clapclap', 'wideSmile', 'thinking', 'rainyDay', 'gg', 'okBuddy', 'sleepy',
  'hype', 'shrug', 'lgtm', 'wave', 'coolCat', 'tinyHeart'];
const HUES = [210, 25, 140, 280, 340, 50, 180, 0];
const ME_ID = '01JD8X0000000000000000MEME';
const ME_SUMMARY = { id: ME_ID, displayName: 'lumen_fox', avatar: null };
const OTHERS = [
  { id: '01JD8X0000000000000000OAK1', displayName: 'quietoak', avatar: null },
  { id: '01JD8X0000000000000000NORA', displayName: 'nora_byte', avatar: null },
];

const emote = (i, owner = OTHERS[i % 2], state = 'APPROVED') => {
  const name = NAMES[i % NAMES.length] + (i >= NAMES.length ? String(i) : '');
  const wide = i % 7 === 3;
  const w = wide ? 64 : 32;
  return {
    id: `01JD8X${String(i).padStart(20, '0')}`,
    name,
    state,
    animated: i % 4 === 1,
    tags: i % 3 === 0 ? ['funny', 'cat'] : [],
    host: `${BASE}/emotes/${i}`,
    createdAt: new Date(Date.UTC(2026, 10, 16, 20, 0, 0) - i * 3600_000).toISOString(),
    flags: { zeroWidth: false, nsfw: false, private: false },
    // Made-up but stable: a few popular ones, a long tail, some in no channel.
    channelCount: i % 11 === 0 ? 0 : (i * 37) % 91 + (i % 5 === 0 ? 400 : 0),
    owner,
    files: [1, 2, 3, 4].map((s) => ({ name: `${s}x.webp`, format: 'WEBP', scale: s, width: w * s, height: 32 * s, size: 900 * s, isStatic: false })),
  };
};
const ALL = Array.from({ length: 140 }, (_, i) => emote(i));
// The mock account's own uploads: a few of each state.
const MINE = Array.from({ length: 9 }, (_, i) =>
  emote(200 + i, ME_SUMMARY, i < 2 ? 'PENDING' : i === 2 ? 'REJECTED' : 'APPROVED'));
let uploads = 0;

const member = (e, name = e.name) => ({ name, zeroWidth: e.flags.zeroWidth, addedAt: e.createdAt, emote: e });
const SETS = [
  { id: '01JD8XSET00000000000000MAIN', name: 'Main', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: [...MINE.slice(3, 7), ...ALL.slice(10, 30)].map((e) => member(e)), editors: [OTHERS[0]] },
  { id: '01JD8XSET0000000000000ALTSS', name: 'Subathon', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: ALL.slice(40, 46).map((e) => member(e)), editors: [] },
  { id: '01JD8XSET000000000000000OAK', name: 'quietoak main', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[0],
    emotes: ALL.slice(60, 70).map((e) => member(e)), editors: [ME_SUMMARY] },
];
const GLOBAL = { id: '01JD8XSET0000000000000GLOBL', name: 'Global', kind: 'GLOBAL', capacity: 200, owner: null,
  emotes: ALL.slice(0, 24).map((e) => member(e)), editors: [] };
SETS.push(GLOBAL);
const PENDING = Array.from({ length: 7 }, (_, i) => emote(400 + i, OTHERS[i % 2], 'PENDING'));
const REPORTS = [
  { id: 'R1', reason: 'This is a copy of my emote.', reporterId: OTHERS[1].id, state: 'OPEN', createdAt: '2026-11-16T18:00:00.000Z', emote: ALL[5] },
  { id: 'R2', reason: 'Spam', reporterId: OTHERS[0].id, state: 'OPEN', createdAt: '2026-11-16T19:30:00.000Z', emote: ALL[8] },
];
const ROLES = [
  { id: 'ROLE1', name: 'Admin', description: 'Runs the platform.', position: 100, permissions: ['roles.manage', 'users.view', 'users.restrict', 'audit.view'] },
  { id: 'ROLE2', name: 'Moderator', description: 'Reviews uploads and reports.', position: 50, permissions: ['emotes.review', 'emotes.takedown'] },
];
const ACCOUNT_ROLES = { [OTHERS[0].id]: ['ROLE2'] };
const STATUS = {};
const AUDIT = Array.from({ length: 64 }, (_, i) => ({
  id: '01JD8XAUD' + String(1000 - i).padStart(17, '0'), actorId: i % 5 === 0 ? null : ME_ID,
  action: ['emote.approved', 'set.editor_added', 'role.updated', 'account.restricted'][i % 4],
  targetKind: ['emote', 'set', 'role', 'account'][i % 4], targetId: '01JD8X' + String(i).padStart(20, '0'),
  before: i % 4 === 3 ? JSON.stringify({ uploads_blocked: false, blocked: false }) : null,
  after: JSON.stringify({ state: 'APPROVED', n: i }), createdAt: new Date(Date.UTC(2026, 10, 16, 22, 0, 0) - i * 600_000).toISOString(),
}));
const CHANNELS = [
  { platform: 'TWITCH', platformChannelId: '141981764', login: 'lumen_fox', activeSetId: SETS[0].id, allowPersonalEmotes: true, account: ME_SUMMARY },
];
const IDENTITIES = [
  { platform: 'TWITCH', platformUserId: '141981764', login: 'lumen_fox', displayName: 'lumen_fox', avatar: null },
];
const NOTES = [
  { id: 'N1', kind: 'emote.rejected', payload: JSON.stringify({ name: MINE[2].name, reason: 'Looks like a copy of a well-known emote.' }), createdAt: '2026-11-16T21:00:00.000Z', read: false },
];
const PERMISSIONS = ['emotes.review', 'emotes.takedown', 'emotes.edit_any', 'sets.edit_any', 'global_set.edit', 'users.view', 'users.restrict', 'roles.manage', 'audit.view'];

// A public account for `account(id)`. Unknown ids, and the one for a blocked account, give null.
const FAKE_LOGINS = ['quietoak', 'nora_byte', 'lumen_fox', 'pixelmoth', 'sipteaTV', 'byteNod', 'foxlook', 'moonhi'];
const publicAccount = (id) => {
  const known = [ME_SUMMARY, ...OTHERS].find((a) => a.id === id);
  if (known) return known;
  const m = /^01JD8XACC(\d{17})$/.exec(id);
  return m ? { id, displayName: FAKE_LOGINS[Number(m[1]) % FAKE_LOGINS.length], avatar: null } : null;
};
// Channels showing an emote: as many as its channelCount says, newest first, from the mock owners.
const emoteChannels = (e, limit) => {
  const n = Math.min(limit, e.channelCount);
  return Array.from({ length: n }, (_, k) => {
    const owner = [...OTHERS, ME_SUMMARY][k % 3];
    const login = k < 3 ? owner.displayName : `${FAKE_LOGINS[k % FAKE_LOGINS.length]}${k}`;
    return {
      platform: k % 4 === 3 ? 'KICK' : 'TWITCH',
      platformChannelId: String(7000 + Number(e.id.slice(-4)) * 100 + k),
      login,
      account: k < 3 ? owner : { id: `01JD8XACC${String(k).padStart(17, '0')}`, displayName: login, avatar: null },
      activeSetId: SETS[k % SETS.length].id,
      allowPersonalEmotes: true,
    };
  });
};
const sorted = (hits, sort) => {
  if (sort === 'TOP') return [...hits].sort((a, b) => b.channelCount - a.channelCount || a.name.localeCompare(b.name));
  if (sort === 'NEW') return [...hits].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  return hits;
};

const findEmote = (id) => MINE.find((e) => e.id === id) ?? ALL.find((e) => e.id === id);
const findSet = (id) => SETS.find((s) => s.id === id);
const err = (message, code) => ({ data: null, errors: [{ message, extensions: { code } }] });
const setOut = (s) => ({ ...s, editors: undefined });

function signedIn(body, v, ADMIN) {
  const op = (body.query.match(/(?:query|mutation)\s+(\w+)/) ?? [])[1];
  switch (op) {
    case 'Me':
      return { data: { me: { id: ME_ID, createdAt: '2026-11-10T12:00:00.000Z', permissions: ADMIN ? PERMISSIONS : [], identities: IDENTITIES } } };
    case 'MyEmotes': {
      const per = v.perPage ?? 50;
      const page = v.page ?? 1;
      return { data: { myEmotes: { total: MINE.length, items: MINE.slice((page - 1) * per, page * per) } } };
    }
    case 'UpdateEmote': {
      const e = MINE.find((x) => x.id === v.id);
      if (!e) return err('Only the emote’s owner can change it.', 'forbidden');
      if (v.input.name !== undefined) e.name = v.input.name;
      if (v.input.tags) e.tags = v.input.tags;
      for (const k of ['zeroWidth', 'private', 'nsfw']) if (v.input[k] !== undefined) e.flags[k] = v.input[k];
      return { data: { updateEmote: e } };
    }
    case 'DeleteEmote': {
      const i = MINE.findIndex((x) => x.id === v.id);
      if (i < 0) return err('Not found.', 'not_found');
      MINE.splice(i, 1);
      for (const s of SETS) s.emotes = s.emotes.filter((m) => m.emote.id !== v.id);
      return { data: { deleteEmote: true } };
    }
    case 'MySets':
      return { data: { mySets: SETS.map(setOut) } };
    case 'MySetNames':
      return { data: { mySets: SETS.map((s) => ({ id: s.id, name: s.name, kind: s.kind, emotes: s.emotes.map((m) => ({ emote: { id: m.emote.id } })) })) } };
    case 'ReportEmote':
      if (!v.reason?.trim()) return err('Give a reason of 1 to 500 characters; the uploader sees it.', 'bad_reason');
      return { data: { reportEmote: { id: 'R' + Date.now() } } };
    case 'EditSet': {
      const s = findSet(v.id);
      if (!s) return { data: { set: null, setEditors: [] } };
      return { data: { set: setOut(s), setEditors: s.editors } };
    }
    case 'CreateSet': {
      if (v.input.kind === 'PERSONAL' && SETS.some((s) => s.kind === 'PERSONAL' && s.owner.id === ME_ID))
        return err('You have a personal set already.', 'personal_set_exists');
      const s = { id: `01JD8XSETNEW${String(SETS.length).padStart(14, '0')}`, name: v.input.name, kind: v.input.kind,
        capacity: v.input.kind === 'PERSONAL' ? 50 : 1000, owner: ME_SUMMARY, emotes: [], editors: [] };
      SETS.push(s);
      return { data: { createSet: { id: s.id } } };
    }
    case 'UpdateSet': {
      const s = findSet(v.id);
      if (v.input.name) s.name = v.input.name;
      return { data: { updateSet: { id: s.id, name: s.name } } };
    }
    case 'DeleteSet': {
      if (CHANNELS.some((c) => c.activeSetId === v.id)) return err('A channel’s active set can’t be deleted.', 'set_in_use');
      SETS.splice(SETS.findIndex((s) => s.id === v.id), 1);
      return { data: { deleteSet: true } };
    }
    case 'AddSetEmote': {
      const s = findSet(v.setId);
      const e = findEmote(v.emoteId);
      if (s.emotes.length >= s.capacity) return err('The set is full.', 'set_full');
      if (s.emotes.some((m) => m.name === e.name)) return err('Another emote in this set has that name.', 'name_taken');
      s.emotes.push(member(e, v.input?.name ?? e.name));
      return { data: { addSetEmote: { name: e.name } } };
    }
    case 'UpdateSetEmote': {
      const m = findSet(v.setId).emotes.find((x) => x.emote.id === v.emoteId);
      if (v.input.name) m.name = v.input.name;
      if (v.input.zeroWidth !== undefined) m.zeroWidth = v.input.zeroWidth;
      return { data: { updateSetEmote: { name: m.name, zeroWidth: m.zeroWidth } } };
    }
    case 'RemoveSetEmote': {
      const s = findSet(v.setId);
      s.emotes = s.emotes.filter((m) => m.emote.id !== v.emoteId);
      return { data: { removeSetEmote: true } };
    }
    case 'AddSetEditor': {
      const s = findSet(v.setId);
      if (s.editors.some((e) => e.id === v.accountId)) return { data: { addSetEditor: false } };
      s.editors.push([...OTHERS, ME_SUMMARY].find((a) => a.id === v.accountId));
      return { data: { addSetEditor: true } };
    }
    case 'RemoveSetEditor': {
      const s = findSet(v.setId);
      s.editors = s.editors.filter((e) => e.id !== v.accountId);
      if (v.accountId === ME_ID) SETS.splice(SETS.indexOf(s), 1);
      return { data: { removeSetEditor: true } };
    }
    case 'FindAccount': {
      const a = OTHERS.find((o) => o.displayName === v.login);
      return { data: { channelByLogin: a ? { login: v.login, account: a } : null } };
    }
    case 'MyChannels':
      return { data: { myChannels: CHANNELS, mySets: SETS.map((s) => ({ id: s.id, name: s.name, kind: s.kind, owner: { id: s.owner.id } })) } };
    case 'UpdateChannel': {
      const c = CHANNELS.find((x) => x.platform === v.platform && x.platformChannelId === v.id);
      if (v.input.activeSetId !== undefined) c.activeSetId = v.input.activeSetId || null;
      if (v.input.allowPersonalEmotes !== undefined) c.allowPersonalEmotes = v.input.allowPersonalEmotes;
      return { data: { updateChannelSettings: { activeSetId: c.activeSetId, allowPersonalEmotes: c.allowPersonalEmotes } } };
    }
    case 'Unlink': {
      if (IDENTITIES.length < 2) return err('The last identity can’t be removed.', 'last_identity');
      IDENTITIES.splice(IDENTITIES.findIndex((i) => i.platform === v.platform), 1);
      return { data: { unlinkIdentity: { id: ME_ID } } };
    }
    case 'MyNotifications':
      return { data: { myNotifications: NOTES } };
    case 'MarkRead': {
      const n = NOTES.filter((x) => !x.read).length;
      NOTES.forEach((x) => (x.read = true));
      return { data: { markNotificationsRead: n } };
    }
    case 'Emote':
      return { data: { emote: findEmote(v.id) ?? PENDING.find((e) => e.id === v.id) ?? null } };
    case 'AdminCounts':
      return { data: { reviewQueue: { total: PENDING.length }, reportQueue: { total: REPORTS.length }, globalSet: { id: GLOBAL.id } } };
    case 'ReviewQueue':
      if (!ADMIN) return err('This needs emotes.review.', 'forbidden');
      return { data: { reviewQueue: { total: PENDING.length, items: PENDING.slice(0, v.perPage ?? 50) } } };
    case 'Approve':
    case 'Reject': {
      const e = PENDING.splice(PENDING.findIndex((x) => x.id === v.id), 1)[0];
      e.state = op === 'Approve' ? 'APPROVED' : 'REJECTED';
      return { data: { [op === 'Approve' ? 'approveEmote' : 'rejectEmote']: { id: e.id, state: e.state } } };
    }
    case 'ReportQueue':
      return { data: { reportQueue: { total: REPORTS.length, items: REPORTS } } };
    case 'ResolveReport': {
      const r = REPORTS.splice(REPORTS.findIndex((x) => x.id === v.id), 1)[0];
      return { data: { resolveReport: { id: r.id, state: v.action === 'DISMISS' ? 'DISMISSED' : 'ACTIONED' } } };
    }
    case 'Roles':
      return { data: { roles: ROLES, permissions: PERMISSIONS } };
    case 'AllRoles':
      return { data: { roles: ROLES } };
    case 'CreateRole': {
      const r = { id: 'ROLE' + (ROLES.length + 1), ...v.input, description: v.input.description ?? null };
      ROLES.push(r);
      return { data: { createRole: r } };
    }
    case 'UpdateRole': {
      const r = ROLES.find((x) => x.id === v.id);
      Object.assign(r, v.input);
      if (r.description === '') r.description = null;
      return { data: { updateRole: r } };
    }
    case 'DeleteRole':
      ROLES.splice(ROLES.findIndex((x) => x.id === v.id), 1);
      return { data: { deleteRole: true } };
    case 'User': {
      const a = [...OTHERS, ME_SUMMARY].find((o) => o.displayName === v.login);
      return { data: { channelByLogin: a ? { platform: v.platform, platformChannelId: '5550' + a.id.slice(-3), login: v.login, account: a } : null } };
    }
    case 'UserDetails': {
      const st = STATUS[v.accountId] ?? { uploadsBlocked: false, blocked: false };
      const roles = (ACCOUNT_ROLES[v.accountId] ?? []).map((id) => ROLES.find((r) => r.id === id)).filter(Boolean);
      return { data: { ...(v.status ? { accountStatus: st } : {}), ...(v.roles ? { accountRoles: roles } : {}) } };
    }
    case 'Restrict': {
      if (v.accountId === ME_ID) return err('The owner’s account can’t be restricted.', 'owner_protected');
      const st = (STATUS[v.accountId] ??= { uploadsBlocked: false, blocked: false });
      if (v.uploadsBlocked !== undefined && v.uploadsBlocked !== null) st.uploadsBlocked = v.uploadsBlocked;
      if (v.blocked !== undefined && v.blocked !== null) st.blocked = v.blocked;
      return { data: { restrictAccount: st } };
    }
    case 'AssignRole':
      (ACCOUNT_ROLES[v.accountId] ??= []).push(v.roleId);
      return { data: { assignRole: true } };
    case 'UnassignRole':
      ACCOUNT_ROLES[v.accountId] = (ACCOUNT_ROLES[v.accountId] ?? []).filter((id) => id !== v.roleId);
      return { data: { unassignRole: true } };
    case 'Audit': {
      let rows = AUDIT.filter((a) => (!v.targetKind || a.targetKind === v.targetKind) && (!v.targetId || a.targetId === v.targetId));
      if (v.before) rows = rows.filter((a) => a.id < v.before);
      return { data: { auditLog: rows.slice(0, v.limit ?? 50) } };
    }
    default:
      return null;
  }
}

function answer(body, session, admin) {
  const q = body.query ?? '';
  const v = body.variables ?? {};
  if (session) {
    const out = signedIn(body, v, admin);
    if (out) return out;
  }
  if (/\baccount\s*\(/.test(q) || /\bemoteChannels\s*\(/.test(q)) {
    const data = {};
    if (/\baccount\s*\(/.test(q)) data.account = publicAccount(v.id ?? v.accountId ?? '');
    if (/\bemoteChannels\s*\(/.test(q)) {
      const e = findEmote(v.id ?? v.emoteId);
      const limit = Math.min(50, Math.max(1, v.limit ?? 8));
      data.emoteChannels = e && e.state === 'APPROVED' && !e.flags.private ? emoteChannels(e, limit) : [];
    }
    return { data };
  }
  if (q.includes('query Me') || /^\s*mutation/.test(q) || q.includes('query My') || q.includes('query EditSet'))
    return err('Sign in to use this.', 'signed_out');
  if (q.includes('query GlobalSet')) return { data: { globalSet: { id: GLOBAL.id, name: 'Global', emotes: GLOBAL.emotes } } };
  if (q.includes('query Search')) {
    const text = (v.query ?? '').toLowerCase();
    let hits = ALL.filter((e) => !text || e.name.toLowerCase().includes(text) || e.tags.some((t) => t.startsWith(text)));
    if (v.animated === true) hits = hits.filter((e) => e.animated);
    if (v.animated === false) hits = hits.filter((e) => !e.animated);
    hits = sorted(hits, v.sort ?? 'RELEVANCE');
    const per = v.perPage ?? 50;
    const page = v.page ?? 1;
    return { data: { searchEmotes: { total: hits.length, page, perPage: per, items: hits.slice((page - 1) * per, page * per) } } };
  }
  if (q.includes('query Emote(')) return { data: { emote: ALL.find((e) => e.id === v.id) ?? null } };
  if (q.includes('query Channel') || q.includes('query FindAccount')) {
    if (v.login !== 'lumen_fox') return { data: { channelByLogin: null } };
    return { data: { channelByLogin: { ...CHANNELS[0], platform: v.platform } } };
  }
  if (q.includes('query Set(')) {
    const s = findSet(v.id) ?? SETS[0];
    return { data: { set: setOut(s) } };
  }
  return err('Not in the mock.', 'not_implemented');
}

function picture(i, scale) {
  const hue = HUES[i % HUES.length];
  const wide = i % 7 === 3;
  const w = (wide ? 64 : 32) * scale;
  const h = 32 * scale;
  const shape = i % 3 === 0
    ? `<circle cx="${w / 2}" cy="${h / 2}" r="${h * 0.42}" fill="hsl(${hue} 70% 55%)"/>`
    : `<rect x="${w * 0.12}" y="${h * 0.12}" width="${w * 0.76}" height="${h * 0.76}" rx="${h * 0.2}" fill="hsl(${hue} 70% 55%)"/>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">${shape}<circle cx="${w * 0.4}" cy="${h * 0.42}" r="${h * 0.06}" fill="#0d0f14"/><circle cx="${w * 0.6}" cy="${h * 0.42}" r="${h * 0.06}" fill="#0d0f14"/></svg>`;
}

function read(req) {
  return new Promise((resolve) => {
    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => resolve(Buffer.concat(chunks)));
  });
}

createServer(async (req, res) => {
  res.setHeader('access-control-allow-origin', ORIGIN);
  res.setHeader('access-control-allow-credentials', 'true');
  res.setHeader('access-control-allow-headers', 'content-type');
  if (req.method === 'OPTIONS') return res.writeHead(204).end();
  const session = /(?:^|;\s*)mock_session=1/.test(req.headers.cookie ?? '');
  const url = new URL(req.url ?? '/', BASE);
  const admin = ADMIN_ENV || /(?:^|;\s*)mock_admin=1/.test(req.headers.cookie ?? '');
  if (url.pathname === '/mock/admin') {
    const off = url.searchParams.has('off');
    const cookie = off ? 'mock_admin=; Path=/; Max-Age=0' : 'mock_admin=1; Path=/; SameSite=Lax';
    res.writeHead(302, { location: ORIGIN + '/admin', 'set-cookie': cookie });
    return res.end();
  }

  const img = url.pathname.match(/^\/emotes\/(\d+)\/(\d)x(?:_static)?\.webp$/);
  if (img) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(picture(Number(img[1]), Number(img[2])));
  }
  if (req.method === 'GET' && /^\/v1\/auth\/(twitch|kick)\/login$/.test(url.pathname)) {
    if (url.searchParams.get('link') === 'true' && !IDENTITIES.some((i) => i.platform === 'KICK') && url.pathname.includes('kick'))
      IDENTITIES.push({ platform: 'KICK', platformUserId: '4815162', login: 'lumen_fox', displayName: 'lumen_fox', avatar: null });
    const back = url.searchParams.get('return_to') ?? '/';
    res.writeHead(302, { location: ORIGIN + (back.startsWith('/') ? back : '/'), 'set-cookie': 'mock_session=1; Path=/; HttpOnly; SameSite=Lax' });
    return res.end();
  }
  if (req.method === 'POST' && url.pathname === '/v1/auth/logout') {
    res.writeHead(204, { 'set-cookie': 'mock_session=; Path=/; Max-Age=0' });
    return res.end();
  }
  if (req.method === 'POST' && url.pathname === '/v1/emotes') {
    const raw = (await read(req)).toString('latin1');
    if (!session) return json(res, 401, { error: 'signed_out', message: 'Sign in to upload.' });
    const name = raw.match(/name="name"\r\n\r\n([^\r]*)/)?.[1] ?? 'upload';
    if (name === 'blocked') return json(res, 422, { error: 'blocked', message: 'This image was taken down before and can’t be uploaded again.' });
    const e = emote(300 + uploads++, ME_SUMMARY, 'PENDING');
    e.name = name;
    e.host = '';
    e.files = [];
    e.createdAt = new Date().toISOString();
    MINE.unshift(e);
    // The worker "finishes" the files a few seconds later.
    setTimeout(() => Object.assign(e, { host: emote(300).host, files: emote(300).files }), 4000);
    return json(res, 202, e);
  }
  if (req.method === 'POST' && url.pathname === '/v1/gql') {
    const raw = (await read(req)).toString('utf8');
    return json(res, 200, answer(JSON.parse(raw || '{}'), session, admin));
  }
  res.writeHead(404).end();
}).listen(PORT, 'localhost', () => console.log(`mock api on ${BASE}${ADMIN_ENV ? ' (admin)' : ''}`));

function json(res, status, body) {
  res.writeHead(status, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
}
