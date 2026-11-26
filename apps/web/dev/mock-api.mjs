// A stand-in for api.cyex.tv, for looking at the site without the real stack:
//   node dev/mock-api.mjs            (listens on localhost:8787)
//   PUBLIC_API_URL=http://localhost:8787 npm run dev
// It answers the site's GraphQL operations with Twitch's global emotes (hotlinked from Twitch's own
// CDN, nothing is downloaded or stored here; see TWITCH below). MOCK_FACES=1 uses 300 made-up emotes
// drawn as little procedural faces (SVG) instead. The mock account's own uploads are always faces. "Signing in" sets a mock cookie; everything signed in lives in memory and
// resets when it restarts. MOCK_ADMIN=1, or opening /mock/admin once, gives the mock account
// every permission (/mock/admin?off clears it). MOCK_OPEN=1 is for a shared preview: every
// visitor counts as signed in as the staff account with every permission, no cookie needed.
// Development only; nothing here ships.
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { Kind, parse } from 'graphql';
import { inward, outward, inwardId, registerAll } from './snowflake.mjs';
import { withFileUrls } from './mock/file-urls.mjs';
import { ADMIN_ENV, BASE, ME_ID, ME_SUMMARY, OPEN_ENV, ORIGIN, OWNERS, PORT } from './mock/config.mjs';
import { avatar, picture } from './mock/art.mjs';
import { ALL, MINE, emote, nextUpload, trendAdds, trendPool, twitchUrl, wideOf } from './mock/emotes.mjs';
import { CHANNELS, IDENTITIES, NOTES, PENDING_DELETION, RESTORE_COOKIE, accountHub, endConsoleToken, mintConsoleToken, setPendingDeletion, setRestoreCookie } from './mock/account.mjs';
import { SETS } from './mock/set-data.mjs';
import { FILE_EMOTES, NSFW_REFUSAL, PENDING, UNLISTED_REFUSAL } from './mock/review.mjs';
import { limitOf, limitsAnswer, pendingOf, publicLimitsRest } from './mock/roles.mjs';
import { grant } from './mock/entitlements.mjs';
import { ACCOUNT_EDITORS, EDITOR_CHANNELS, EDITOR_OF, PUBLIC_SETS, SHARED_SETS, myEditableChannels } from './mock/editors.mjs';
import { shopAnswer } from './mock/shop.mjs';
import { BADGE_ART, ORIGINAL_PX, BADGE_LADDER, C_IMAGES, PAINT_LADDER, UPLOADED, cosmeticsAnswer, ladderFiles, nextCosmeticSeq, releasedCatalog, userCosmeticsOut } from './mock/cosmetics.mjs';
import { activityAnswer, activityPageAnswer, emoteActivityAnswer, emoteActivityPageAnswer } from './mock/activity.mjs';
import { channelByLogin, emoteChannels, publicAccount, sorted } from './mock/directory.mjs';
import { accountSets, cloneSet, emoteGrant, err, findEmote, findSet, mayView, setOut } from './mock/sets.mjs';
import { accountCosmetics, accountProfile, quickSearch, searchAccounts } from './mock/people.mjs';
import { PERSONAL_FIELDS, personalLists } from './mock/personal.mjs';
import { topPeopleAnswer } from './mock/top-people.mjs';
import { META, noteOff, siteAnswer } from './mock/site.mjs';
import { UPLOAD_FILES, mosaicPng, multipart, probeImage } from './mock/uploads.mjs';
import { signedIn } from './mock/signed-in/index.mjs';
function newFields(q, v, session, admin = false) {
  if (/\b(mySessions|myConsoleToken|createConsoleToken|unreadNotificationCount|revokeSession|revokeOtherSessions|signOutEverywhere|exportMyData|deleteAccount|markNotificationsRead)\b/.test(q))
    if (!session) return err('Sign in to use this.', 'signed_out');
  // Each handler answers one new field; an operation may ask for several, and for older ones too.
  const id = v.accountId ?? v.id ?? '';
  const parts = [];
  const hub = session ? accountHub(q, v) : null;
  if (hub?._cookie || hub?.errors) return hub;
  if (hub) parts.push(hub);
  const has = (name) => new RegExp(`\\b${name}\\s*\\(`).test(q);
  if (PERSONAL_FIELDS.some((f) => new RegExp(`\\b${f}\\b`).test(q))) {
    if (!session) return err('Sign in to use this.', 'signed_out');
    const lists = personalLists(q, v);
    if (lists.errors) return lists;
    parts.push(lists);
  }
  // The undo window (ADR 0019): only the restore cookie sees and undoes the deletion.
  if (/\bpendingDeletion\b/.test(q)) return { data: { pendingDeletion: !session && RESTORE_COOKIE ? PENDING_DELETION : null } };
  if (/\brestoreAccount\b/.test(q)) {
    if (session || !RESTORE_COOKIE || !PENDING_DELETION) return err('Sign in to the deleted account to restore it.', 'signed_out');
    setPendingDeletion(null);
    return { data: { restoreAccount: true }, _cookie: 'mock_session=1; Path=/; HttpOnly; SameSite=Lax' };
  }
  if (has('accountProfile')) parts.push({ data: { accountProfile: accountProfile(id) } });
  if (has('accountCosmetics')) {
    const list = accountCosmetics(v.accountIds ?? v.ids);
    parts.push(list === null ? err('Look up at most 100 accounts at once.', 'too_many') : { data: { accountCosmetics: list } });
  }
  if (/\blimits\s*\{/.test(q)) parts.push({ data: { limits: limitsAnswer(session) } });
  if (has('searchAccounts')) parts.push(searchAccounts(v));
  if (has('quickSearch')) parts.push(quickSearch(v));
  const top = topPeopleAnswer(q, v, session, admin);
  if (top?.errors) return top;
  if (top) parts.push(top);
  if (/\bmyEditableChannels\b/.test(q)) parts.push(session ? { data: { myEditableChannels: myEditableChannels() } } : err('Sign in to use this.', 'signed_out'));
  if (has('cloneSet')) parts.push(session ? cloneSet(v, session, admin) : err('Sign in to use this.', 'signed_out'));
  if (has('accountSets')) parts.push({ data: { accountSets: accountSets(id, v.preview) } });
  const site = siteAnswer(q, v, session, admin);
  if (site?.errors) return site;
  if (site) parts.push(site);
  if (!parts.length) return null;
  const failed = parts.find((p) => p.errors);
  if (failed) return failed;
  return { data: Object.assign({}, ...parts.map((p) => p.data)) };
}

// New fields merge with whatever else the same operation asks for (the older handlers below).
function answer(body, session, admin) {
  const added = newFields(body.query ?? '', body.variables ?? {}, session, admin);
  if (!added) return baseAnswer(body, session, admin);
  if (added.errors || !added.data) return added;
  const base = baseAnswer(body, session, admin);
  return { ...added, data: base?.data && !base.errors ? { ...base.data, ...added.data } : added.data };
}

function baseAnswer(body, session, admin) {
  const q = body.query ?? '';
  const v = body.variables ?? {};
  const cosmetic = cosmeticsAnswer(q, v, session, admin);
  if (cosmetic) return cosmetic;
  const shop = shopAnswer(q, v, session, admin);
  if (shop) return shop;
  if (/accountActivityPage\s*\(/.test(q)) return activityPageAnswer(v, session, admin);
  if (/emoteActivityPage\s*\(/.test(q)) return emoteActivityPageAnswer(v);
  if (/accountActivity\s*\(/.test(q)) return activityAnswer(v);
  if (/emoteActivity\s*\(/.test(q)) return emoteActivityAnswer(v);
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
      const offset = Math.max(0, v.offset ?? 0);
      data.emoteChannels = e && e.state === 'APPROVED' && !e.flags.private && offset <= 10000 ? emoteChannels(e, limit, offset) : [];
    }
    return { data };
  }
  if (q.includes('query Me') || /^\s*mutation/.test(q) || q.includes('query My') || q.includes('query EditSet'))
    return err('Sign in to use this.', 'signed_out');
  if (q.includes('query EmoteTrending')) {
    const e = ALL.find((x) => x.id === v.id);
    const window = v.window ?? 'WEEK';
    const rank = e ? trendPool(window).indexOf(e) + 1 : 0;
    return { data: { emoteTrending: rank > 0 && rank <= 100 ? { rank, channels: trendAdds(e, window), window } : null } };
  }
  if (q.includes('query Search')) {
    // As the API: the text (exact name or rough match), then type, owner, flags (all must be set; the
    // retired NSFW flag finds nothing, and `nsfw` changes nothing) and tags (all must be there).
    const text = (v.query ?? '').trim().toLowerCase();
    const terms = text.split(/\s+/).filter(Boolean);
    const flags = new Set(v.flags ?? []);
    const tags = [...new Set((v.tags ?? []).map((t) => t.trim().replace(/^#+/, '').toLowerCase()).filter(Boolean))].slice(0, 10);
    // Unlisted and private emotes are never in search or browse (ADR 0020).
    let hits = ALL.filter((e) => e.state === 'APPROVED' && e.visibility === 'PUBLIC');
    hits = v.exact && text
      ? hits.filter((e) => e.name.toLowerCase() === text)
      : hits.filter((e) => terms.every((t) => e.name.toLowerCase().includes(t) || e.tags.some((g) => g.includes(t))));
    if (v.animated === true) hits = hits.filter((e) => e.animated);
    if (v.animated === false) hits = hits.filter((e) => !e.animated);
    if (v.owner) hits = hits.filter((e) => e.owner?.id === v.owner);
    if (flags.has('NSFW')) hits = [];
    if (flags.has('ZERO_WIDTH')) hits = hits.filter((e) => e.flags.zeroWidth);
    if (flags.has('WIDE')) hits = hits.filter((e) => e.flags.wide);
    if (tags.length) hits = hits.filter((e) => tags.every((t) => e.tags.some((g) => g.toLowerCase() === t)));
    hits = sorted(hits, v.sort ?? 'RELEVANCE', v.window ?? 'WEEK');
    const per = v.perPage ?? 50;
    const page = v.page ?? 1;
    return { data: { searchEmotes: { total: hits.length, page, perPage: per, items: hits.slice((page - 1) * per, page * per) } } };
  }
  if (q.includes('query Emote(')) return { data: { emote: ALL.find((e) => e.id === v.id) ?? null } };
  if (q.includes('query Channel')) {
    const c = channelByLogin(v.platform, v.login);
    return { data: { channelByLogin: c } };
  }
  if (q.includes('query Set(') || /\bset\s*\(\s*id:/.test(q)) {
    // Unknown ids and private sets you can't see are null.
    const s = findSet(v.id);
    return { data: { set: s && mayView(s, session) ? setOut(s) : null } };
  }
  return err('Not in the mock.', 'not_implemented');
}

function read(req) {
  return new Promise((resolve) => {
    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => resolve(Buffer.concat(chunks)));
  });
}

// Any local dev port may call the mock (several dev servers run at once); redirects go back to the
// site that sent the visitor, else to SITE_ORIGIN.
const LOCAL = /^http:\/\/(localhost|127\.0\.0\.1):\d+$/;
function siteOf(req) {
  const origin = req.headers.origin;
  if (origin && (LOCAL.test(origin) || OPEN_ENV)) return origin; // a shared preview is reached from other hosts
  try {
    const ref = new URL(req.headers.referer ?? '').origin;
    if (LOCAL.test(ref)) return ref;
  } catch {}
  return ORIGIN;
}

// Ids of emotes, accounts and sets are snowflakes on the wire (dev/snowflake.mjs); everything here is keyed by their ULIDs.
registerAll(ALL, MINE, PENDING, SETS, SHARED_SETS, PUBLIC_SETS, OWNERS, CHANNELS, EDITOR_CHANNELS, EDITOR_OF, ACCOUNT_EDITORS);

createServer(async (req, res) => {
  const SITE = siteOf(req);
  res.setHeader('access-control-allow-origin', SITE);
  res.setHeader('access-control-allow-credentials', 'true');
  res.setHeader('access-control-allow-headers', 'content-type');
  if (req.method === 'OPTIONS') return res.writeHead(204).end();
  const session = OPEN_ENV || /(?:^|;\s*)mock_session=1/.test(req.headers.cookie ?? '');
  setRestoreCookie(/(?:^|;\s*)mock_restore=1/.test(req.headers.cookie ?? ''));
  const url = new URL(req.url ?? '/', BASE);
  const admin = ADMIN_ENV || /(?:^|;\s*)mock_admin=1/.test(req.headers.cookie ?? '');
  // /mock/notify: one new unread notification now (for the header's toasts); the bell's poll picks it up.
  // ?kind=cosmetic.granted for another kind; a kind turned off in Settings → Notifications is skipped, as the API does.
  if (url.pathname === '/mock/notify') {
    const kind = url.searchParams.get('kind') || 'emote.approved';
    if (!noteOff(kind))
      NOTES.unshift({ id: `01JD8XLIVE${String(Date.now()).padStart(16, '0')}`, kind, payload: JSON.stringify({ emote_id: MINE[0].id, name: MINE[0].name, kind: 'paint' }), createdAt: new Date().toISOString(), read: false });
    res.writeHead(200, { 'content-type': 'text/plain' });
    return res.end('ok');
  }
  if (url.pathname === '/mock/admin') {
    const off = url.searchParams.has('off');
    const cookie = off ? 'mock_admin=; Path=/; Max-Age=0' : 'mock_admin=1; Path=/; SameSite=Lax';
    res.writeHead(302, { location: SITE + '/admin', 'set-cookie': cookie });
    return res.end();
  }

  const tw = url.pathname.match(/^\/twitch\/(\d+)\/(\d)x(_static)?\.webp$/);
  if (tw) {
    const to = twitchUrl(Number(tw[1]), Number(tw[2]), Boolean(tw[3]));
    if (!to) return res.writeHead(404).end();
    res.writeHead(302, { location: to, 'cache-control': 'max-age=3600' });
    return res.end();
  }
  const img = url.pathname.match(/^\/emotes\/(\d+)\/(\d)x(_static)?\.(?:webp|avif)$/);
  if (img) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(picture(Number(img[1]), Number(img[2]), Boolean(img[3])));
  }
  const paintArt = url.pathname.match(/^\/cosmetics\/paints\/(sample-mosaic(?:-old)?)\/(?:[124]x|512)\.webp$/);
  if (paintArt) {
    const px = PAINT_LADDER.find(([stem]) => url.pathname.endsWith(`/${stem}.webp`))[2];
    res.writeHead(200, { 'content-type': 'image/png', 'cache-control': 'no-store' });
    return res.end(mosaicPng(px));
  }
  const badgeArt = url.pathname.match(/^\/cosmetics\/badges\/([a-z-]+)\/([124]x|128|256|512|original)\.webp$/);
  if (badgeArt && BADGE_ART[badgeArt[1]]) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    // `original` is the picture's own size (ORIGINAL_PX in mock/cosmetics.mjs; 96 for uploads made earlier).
    const px = badgeArt[2] === 'original' ? (ORIGINAL_PX[badgeArt[1]] ?? 96) : BADGE_LADDER.find(([stem]) => stem === badgeArt[2])[2];
    return res.end(BADGE_ART[badgeArt[1]](px / 18));
  }
  if (req.method === 'GET' && url.pathname === '/v1/limits') return json(res, 200, publicLimitsRest());
  if (req.method === 'GET' && url.pathname === '/v1/cosmetics') {
    const c = releasedCatalog(admin);
    return json(res, 200, restOf(c));
  }
  if (req.method === 'GET' && url.pathname === '/v1/cosmetics/users') {
    const ids = (url.searchParams.get('ids') ?? '').split(',').filter(Boolean);
    const out = userCosmeticsOut((url.searchParams.get('platform') ?? '').toUpperCase(), ids);
    if (!out) return json(res, 400, { error: 'too_many', message: 'Look up at most 100 users at once.' });
    return json(res, 200, { users: out.map((u) => ({ platform_user_id: u.platformUserId, badge_id: u.badgeId, paint_id: u.paintId })) });
  }
  if (req.method === 'POST' && url.pathname === '/v1/cosmetics/images') {
    const body = await read(req);
    if (!session) return json(res, 401, { error: 'signed_out', message: 'Sign in to upload.' });
    if (!admin) return json(res, 403, { error: 'forbidden', message: 'This needs cosmetics.manage.' });
    const parts = multipart(body, req.headers['content-type']);
    if (!parts) return json(res, 400, { error: 'bad_form', message: 'The form couldn’t be read.' });
    const kind = parts.kind?.data.toString('utf8').trim();
    if (kind === undefined) return json(res, 400, { error: 'bad_form', message: 'Say what it is for: kind=badge or kind=paint.' });
    if (kind !== 'badge' && kind !== 'paint') return json(res, 400, { error: 'bad_kind', message: 'kind is badge or paint.' });
    const file = parts.file?.data;
    if (!file?.length) return json(res, 400, { error: 'bad_form', message: 'Attach the image as `file`.' });
    if (file.length > 8 * 1024 * 1024) return json(res, 413, { error: 'too_large', message: 'The file is over 8 MB, the most an upload can be.' });
    // The uploaded picture itself is kept and served back (as the worker's files would be), so what
    // was uploaded is what the site shows. The checks are the API's (cosmetics::check_dimensions).
    const probe = probeImage(file);
    if (!probe) return json(res, 415, { error: 'unsupported_format', message: 'Use a PNG, APNG, JPEG, GIF or WebP.' });
    if (kind === 'badge' && !(probe.width === probe.height && probe.width >= 72))
      return json(res, 422, { error: 'bad_dimensions', message: 'Badge pictures are square and at least 72×72 px.' });
    if (kind === 'paint' && probe.height < 64) return json(res, 422, { error: 'bad_dimensions', message: 'Paint pictures are at least 64 px tall.' });
    if (kind === 'paint' && probe.width > probe.height * 8) return json(res, 422, { error: 'bad_dimensions', message: 'Paint pictures are at most 8 times as wide as they are tall (8:1).' });
    // The real POST answers in the REST shape, and the same bytes give the same picture again
    // (the API keys pictures by their SHA-256), already ready.
    const sha = createHash('sha256').update(file).digest('hex');
    const again = UPLOADED.get(sha);
    if (again) return json(res, 202, restOf(again));
    const id = `01JD8XIMG${String(100 + nextCosmeticSeq()).padStart(17, '0')}`;
    // A badge's original is its own size, at most 512 px (bigger is scaled down to 512).
    const fit = Math.min(1, 512 / Math.max(probe.width, probe.height));
    const files = kind === 'badge' ? ladderFiles(probe.animated, file.length, BADGE_LADDER, [Math.round(probe.width * fit), Math.round(probe.height * fit)]) : PAINT_LADDER.flatMap(([stem, scale, px]) => {
      const h = Math.min(px, probe.height); // never above the original
      const w = Math.round((probe.width / probe.height) * h);
      return (probe.animated ? [false, true] : [false]).map((still) => ({ name: `${stem}${still ? '_static' : ''}.webp`, format: 'WEBP', scale, width: w, height: h, size: file.length, isStatic: still }));
    });
    UPLOAD_FILES.set(id, { bytes: file, mime: probe.mime });
    const img = { id, kind: kind.toUpperCase(), host: `${BASE}/cosmetics/uploads/${id}`, animated: probe.animated, width: probe.width, height: probe.height,
      ready: false, files: [], createdAt: new Date().toISOString() };
    C_IMAGES.push(img);
    UPLOADED.set(sha, img);
    // The files are "made" a moment later, as the worker would.
    setTimeout(() => Object.assign(img, { ready: true, files }), Number(process.env.MOCK_IMAGE_DELAY ?? 2000));
    return json(res, 202, restOf(img));
  }
  const uploaded = url.pathname.match(/^\/cosmetics\/uploads\/([0-9A-Z]+)\/(?:[124]x|128|256|512|original)(?:_static)?\.(?:webp|avif)$/);
  if (uploaded) {
    const u = UPLOAD_FILES.get(uploaded[1]);
    if (!u) return res.writeHead(404).end();
    res.writeHead(200, { 'content-type': u.mime, 'cache-control': 'no-store' });
    return res.end(u.bytes);
  }
  const av = url.pathname.match(/^\/avatars\/(\d+)\.svg$/);
  if (av) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(avatar(Number(av[1])));
  }
  // App sign-in (ADR 0021), as the API answers it: authorize -> the site's /authorize consent page ->
  // consent (cookie) -> the loopback redirect with a code -> token. Codes and tokens are fakes.
  if (url.pathname.startsWith('/v1/auth/app/')) {
    const loopback = (raw) => /^http:\/\/(127\.0\.0\.1|\[::1\])(:\d{1,5})?\/callback$/.test(raw ?? '');
    const back = (redirect, pairs) => `${redirect}?${new URLSearchParams(pairs)}`;
    if (req.method === 'GET' && url.pathname === '/v1/auth/app/authorize') {
      const p = url.searchParams;
      if (p.get('client_id') !== 'cyex-deck') return json(res, 400, { error: 'unknown_client', message: 'Unknown client_id.' });
      if (!loopback(p.get('redirect_uri'))) return json(res, 400, { error: 'bad_redirect_uri', message: 'redirect_uri must be http://127.0.0.1:<port>/callback or http://[::1]:<port>/callback.' });
      const state = p.get('state') ?? '';
      const challenge = p.get('code_challenge') ?? '';
      if (p.get('code_challenge_method') !== 'S256' || !/^[A-Za-z0-9_-]{43}$/.test(challenge) || !state) {
        res.writeHead(303, { location: back(p.get('redirect_uri'), { error: 'invalid_request', error_description: 'Bad request.', ...(state ? { state } : {}) }) });
        return res.end();
      }
      const q = new URLSearchParams({ client_id: 'cyex-deck', redirect_uri: p.get('redirect_uri'), code_challenge: challenge, code_challenge_method: 'S256', state });
      res.writeHead(303, { location: `${SITE}/authorize?${q}` });
      return res.end();
    }
    if (req.method === 'GET' && url.pathname.startsWith('/v1/auth/app/clients/')) {
      if (decodeURIComponent(url.pathname.slice('/v1/auth/app/clients/'.length)) !== 'cyex-deck') return json(res, 404, { error: 'unknown_client', message: 'Unknown client_id.' });
      return json(res, 200, { clientId: 'cyex-deck', name: 'cyex deck', scopes: [
        { id: 'sets.emotes', description: 'Manage emotes in sets you can edit' },
        { id: 'cosmetics.select', description: 'Pick your cosmetics' },
      ] });
    }
    if (req.method === 'POST' && url.pathname === '/v1/auth/app/consent') {
      if (!session) return json(res, 401, { error: 'signed_out', message: 'Sign in on cyex.tv to allow an app.' });
      const body = JSON.parse((await read(req)).toString('utf8') || '{}');
      if (body.clientId !== 'cyex-deck') return json(res, 400, { error: 'unknown_client', message: 'Unknown client_id.' });
      if (!loopback(body.redirectUri)) return json(res, 400, { error: 'bad_redirect_uri', message: 'redirect_uri must be a loopback /callback.' });
      const location = body.allow
        ? back(body.redirectUri, { code: `mockcode${String(Date.now()).padStart(35, '0')}`, state: body.state ?? '' })
        : back(body.redirectUri, { error: 'access_denied', state: body.state ?? '' });
      return json(res, 200, { location });
    }
    if (req.method === 'POST' && url.pathname === '/v1/auth/app/token') {
      const form = new URLSearchParams((await read(req)).toString('utf8'));
      if (form.get('client_id') !== 'cyex-deck') return json(res, 401, { error: 'invalid_client', error_description: 'Unknown client_id.', message: 'Unknown client_id.' });
      const grant = form.get('grant_type');
      if (grant !== 'authorization_code' && grant !== 'refresh_token') return json(res, 400, { error: 'unsupported_grant_type', error_description: 'grant_type must be authorization_code or refresh_token.', message: 'grant_type must be authorization_code or refresh_token.' });
      const fake = () => `mock${Math.random().toString(36).slice(2).padEnd(39, '0')}`.slice(0, 43);
      return json(res, 200, { access_token: fake(), token_type: 'Bearer', expires_in: 3600, refresh_token: fake(), scope: 'sets.emotes cosmetics.select' });
    }
    if (req.method === 'POST' && url.pathname === '/v1/auth/app/revoke') return res.writeHead(200).end();
  }
  if (req.method === 'GET' && /^\/v1\/auth\/(twitch|kick)\/login$/.test(url.pathname)) {
    if (url.searchParams.get('link') === 'true' && !IDENTITIES.some((i) => i.platform === 'KICK') && url.pathname.includes('kick'))
      IDENTITIES.push({ platform: 'KICK', platformUserId: '4815162', login: 'lumen_fox', displayName: 'lumen_fox', avatar: null });
    const back = url.searchParams.get('return_to') ?? '/';
    // A deleted account within its 30 days lands on /restore with a restore-only cookie.
    if (PENDING_DELETION) {
      res.writeHead(302, { location: SITE + '/restore', 'set-cookie': 'mock_restore=1; Path=/; HttpOnly; SameSite=Lax' });
      return res.end();
    }
    res.writeHead(302, { location: SITE + (back.startsWith('/') ? back : '/'), 'set-cookie': 'mock_session=1; Path=/; HttpOnly; SameSite=Lax' });
    return res.end();
  }
  if (req.method === 'POST' && url.pathname === '/v1/auth/token/export') {
    if (!session) return json(res, 401, { error: 'signed_out', message: 'Sign in to use this.' });
    const t = mintConsoleToken();
    res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' });
    return res.end(JSON.stringify({ token: t.token, token_type: t.tokenType, expires_at: t.expiresAt, session_id: t.sessionId }));
  }
  if (req.method === 'POST' && url.pathname === '/v1/auth/logout') {
    endConsoleToken();
    res.writeHead(204, { 'set-cookie': ['mock_session=; Path=/; Max-Age=0', 'mock_restore=; Path=/; Max-Age=0'] });
    return res.end();
  }
  // A route that never answers, for timeout checks.
  if (req.method === 'GET' && (url.pathname === '/health' || url.pathname === '/health/ready')) return res.writeHead(200, { 'content-type': 'application/json' }).end('{"status":"ok"}');
  if (req.method === 'GET' && url.pathname === '/v1/meta') return res.writeHead(200, { 'content-type': 'application/json', 'x-cyex-api-version': META.api_version }).end(JSON.stringify(META));
  if (url.pathname === '/mock/hang') return;
  if (req.method === 'POST' && url.pathname === '/v1/emotes') {
    // UTF-8, as the API reads it: names in any script and emoji arrive whole.
    const body = await read(req);
    const raw = body.toString('utf8');
    // The file's SHA-256, as the API keys files: the same bytes again are "the same file".
    const fileBytes = multipart(body, req.headers['content-type'])?.file?.data;
    const fileSha = fileBytes?.length ? createHash('sha256').update(fileBytes).digest('hex') : null;
    if (!session) return json(res, 401, { error: 'signed_out', message: 'Sign in to upload.' });
    const field = (k) => raw.match(new RegExp(`name="${k}"\\r\\n\\r\\n([^\\r]*)`))?.[1];
    const name = field('name') ?? 'upload';
    // Names that stand in for the API's refusals, to see how the upload form explains them.
    // Never answers (the request is read, then left open), to see the stall timeout and Cancel.
    if (name === 'hang') return;
    if (name === 'blocked') return json(res, 422, { error: 'blocked', message: 'This image can’t be uploaded to cyex.tv.' });
    if (name === 'ratelimited') return json(res, 429, { error: 'rate_limited', message: 'That’s a lot of uploads. Try again in an hour.' });
    if (name === 'toobig') return json(res, 413, { error: 'too_large', message: 'The file is over 8 MB, the most an upload can be.' });
    if (name === 'broken') return json(res, 422, { error: 'invalid_image', message: 'That file isn’t a valid image. Try a PNG, GIF, WebP or JPEG that opens elsewhere.' });
    // The flags, as the API reads them (POST /v1/emotes): true/1/on, false/0/off/empty.
    const flag = (k) => {
      const v = (field(k) ?? '').trim().toLowerCase();
      if (['true', '1', 'on'].includes(v)) return true;
      if (['false', '0', 'off', ''].includes(v)) return false;
      return null;
    };
    const flags = { zeroWidth: flag('zero_width'), private: flag('private'), nsfw: flag('nsfw') };
    if (Object.values(flags).includes(null)) return json(res, 400, { error: 'bad_form', message: 'A flag is true or false.' });
    // NSFW emotes aren't allowed; `nsfw=false` is still taken (ADR 0020, amendment 2026/11/22).
    if (flags.nsfw) return json(res, 400, { error: 'nsfw_not_allowed', message: NSFW_REFUSAL });
    flags.nsfw = false;
    const visField = (field('visibility') ?? '').trim().toLowerCase();
    if (visField && !['public', 'unlisted', 'private'].includes(visField)) return json(res, 400, { error: 'bad_form', message: 'Visibility is public or private.' });
    // Uploads are public unless private; only staff (the mock admin) may upload unlisted.
    if (visField === 'unlisted' && !admin) return json(res, 400, { error: 'bad_visibility', message: UNLISTED_REFUSAL });
    const visibility = visField ? visField.toUpperCase() : flags.private ? 'PRIVATE' : 'PUBLIC';
    if (field('private') !== undefined && visField && flags.private !== (visibility === 'PRIVATE'))
      return json(res, 400, { error: 'bad_form', message: 'visibility and private say different things.' });
    flags.private = visibility === 'PRIVATE';
    // Upload as (ADR 0014): an account where you hold EMOTE_UPLOAD on every set owns it.
    const ownerId = inwardId((field('owner_id') ?? '').trim());
    const forOther = ownerId && ownerId !== ME_ID;
    if (forOther && !emoteGrant(ownerId, 'EMOTE_UPLOAD'))
      return json(res, 403, { error: 'forbidden', message: 'You can upload for another account only as its editor with the upload permission on every set.' });
    // At most pending_emotes of the owner's emotes wait for review (upload.rs).
    const cap = limitOf(forOther ? ownerId : ME_ID, 'pending_emotes');
    if (name === 'toomanypending' || pendingOf(forOther ? ownerId : ME_ID) >= cap) {
      const waiting = cap === 1 ? '1 emote' : `${cap} emotes`;
      return json(res, 409, { error: 'too_many_pending', message: forOther
        ? `That account has ${waiting} waiting for review. Upload more once some are reviewed.`
        : `You have ${waiting} waiting for review. Upload more once some are reviewed.` });
    }
    await new Promise((r) => setTimeout(r, 700)); // long enough to see the progress
    const owner = forOther ? EDITOR_OF.find((g) => g.account.id === ownerId).account : ME_SUMMARY;
    const e = emote(700 + nextUpload(), owner, 'PENDING');
    e.name = name;
    e.flags = { ...flags, wide: false };
    e.visibility = visibility;
    e.tags = (field('tags') ?? '').split(',').map((t) => t.trim().toLowerCase()).filter(Boolean);
    e.host = '';
    e.files = [];
    e.createdAt = new Date().toISOString();
    if (forOther) PENDING.unshift(e);
    else MINE.unshift(e);
    if (fileSha) (FILE_EMOTES[fileSha] ??= []).push(e);
    // The worker "finishes" the files a few seconds later.
    setTimeout(() => {
      Object.assign(e, { host: emote(700).host, files: emote(700).files });
      e.flags.wide = wideOf(e);
    }, 4000);
    return json(res, 202, restOf(e));
  }
  if (req.method === 'POST' && url.pathname === '/v1/gql') {
    const raw = (await read(req)).toString('utf8');
    // A malformed request (bad JSON, no query, a batch) gets an error, never a crashed mock.
    try {
      const body = JSON.parse(raw || '{}');
      if (typeof body?.query !== 'string') return json(res, 400, { data: null, errors: [{ message: 'Send one query.', extensions: { code: 'bad_request' } }] });
      if (body.variables) body.variables = inward(body.variables);
      const out = answer(body, session, admin);
      if (out?._cookie) {
        res.setHeader('set-cookie', out._cookie);
        delete out._cookie;
      }
      return json(res, 200, pruneAnswer(body.query, withFileUrls(out)));
    } catch (e) {
      return json(res, 400, { data: null, errors: [{ message: String(e?.message ?? e), extensions: { code: 'bad_request' } }] });
    }
  }
  res.writeHead(404).end();
}).listen(PORT, process.env.MOCK_HOST ?? 'localhost', () => console.log(`mock api on ${BASE}${OPEN_ENV ? ' (open: everyone is staff)' : ADMIN_ENV ? ' (admin)' : ''}`));

// The REST API's JSON: snake_case keys and lower-case enum values (the GraphQL shapes are camelCase
// with upper-case enums).
const REST_ENUM_KEYS = new Set(['kind', 'shape', 'format', 'state', 'visibility', 'processing']);
function restOf(x, key = '') {
  return withFileUrls(restShape(x, key));
}

function restShape(x, key = '') {
  if (Array.isArray(x)) return x.map((v) => restShape(v, key));
  if (x && typeof x === 'object') {
    return Object.fromEntries(Object.entries(x).map(([k, v]) => [k.replace(/[A-Z]/g, (m) => '_' + m.toLowerCase()), restShape(v, k)]));
  }
  return typeof x === 'string' && REST_ENUM_KEYS.has(key) && /^[A-Z_]+$/.test(x) ? x.toLowerCase() : x;
}

// Cuts an answer down to what the query selected, as a GraphQL server does: the mock's objects carry
// more than any one query asks for.
function prune(value, sets, frags) {
  if (Array.isArray(value)) return value.map((v) => prune(v, sets, frags));
  if (value === null || typeof value !== 'object' || !sets.length) return value;
  const fields = new Map();
  const collect = (set) => {
    for (const sel of set.selections) {
      if (sel.kind === Kind.FIELD) {
        const key = sel.alias?.value ?? sel.name.value;
        const f = fields.get(key) ?? { name: sel.name.value, sets: [] };
        if (sel.selectionSet) f.sets.push(sel.selectionSet);
        fields.set(key, f);
      } else if (sel.kind === Kind.INLINE_FRAGMENT) collect(sel.selectionSet);
      else if (frags.has(sel.name.value)) collect(frags.get(sel.name.value));
    }
  };
  sets.forEach(collect);
  const out = {};
  for (const [key, f] of fields) {
    const src = key in value ? value[key] : value[f.name];
    if (src !== undefined) out[key] = prune(src, f.sets, frags);
  }
  return out;
}
function pruneAnswer(query, out) {
  if (!out?.data || typeof out.data !== 'object') return out;
  const doc = parse(query);
  const frags = new Map(doc.definitions.filter((d) => d.kind === Kind.FRAGMENT_DEFINITION).map((d) => [d.name.value, d.selectionSet]));
  const op = doc.definitions.find((d) => d.kind === Kind.OPERATION_DEFINITION);
  return op ? { ...out, data: prune(out.data, [op.selectionSet], frags) } : out;
}

function json(res, status, body) {
  res.writeHead(status, { 'content-type': 'application/json' });
  res.end(JSON.stringify(outward(body)));
}
