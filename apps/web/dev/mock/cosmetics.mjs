// Mock API: cosmetics. See ../mock-api.mjs.
import { BASE, ME_ID, ME_SUMMARY, OTHERS } from './config.mjs';
import { flagsOn } from './editors.mjs';
import { SHOP_ITEMS } from './shop.mjs';
import { publicAccount } from './directory.mjs';
import { err } from './sets.mjs';

// ---- Chat cosmetics (ADR 0015) ----------------------------------------------------------
// SAMPLE DATA ONLY: two made-up badges (simple SVG drawn here) and three sample paints, so the
// site can be built before any real cosmetics exist. Pictures are served by this mock as SVG under
// /cosmetics/badges/<key>/<file>.webp on the badge size ladder (1x, 2x, 4x, 128, 256, 512); the
// star is a picture made before the ladder (1x, 2x and 4x only) until "Make larger sizes"
// (remakeCosmeticImage) fills it in. Uploaded badge and paint pictures (POST /v1/cosmetics/images)
// are kept as sent and served back at /cosmetics/uploads/<id>/<file>[_static].webp: the same bytes
// for every size (nothing is transcoded here), so a paint made in the studio shows its own render.
// Operations are recognised by their root field; arguments are
// read from variables named like the arguments ($id, $input, $badgeId, $paintId, $ownerId,
// $accountId, $kind, $cosmeticId, $platform, $userIds, $reason, $page, $perPage).
export const BADGE_ART = {
  'sample-star': (k) => `<svg xmlns="http://www.w3.org/2000/svg" width="${18 * k}" height="${18 * k}" viewBox="0 0 18 18"><circle cx="9" cy="9" r="8.5" fill="hsl(268 70% 55%)"/><path d="M9 3.2l1.7 3.6 3.9.5-2.9 2.7.8 3.9L9 12l-3.5 1.9.8-3.9-2.9-2.7 3.9-.5z" fill="hsl(46 95% 62%)"/></svg>`,
  'sample-shield': (k) => `<svg xmlns="http://www.w3.org/2000/svg" width="${18 * k}" height="${18 * k}" viewBox="0 0 18 18"><path d="M9 1l7 2.6v5c0 4.2-3 7.3-7 8.4-4-1.1-7-4.2-7-8.4v-5z" fill="hsl(160 60% 38%)"/><path d="M5.6 9.2l2.3 2.3 4.6-4.7" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
};
export const C_TIME = '2026-11-17T12:00:00.000Z';
// The badge size ladder (crates/media Profile::BADGE): file stem, scale and px.
export const BADGE_LADDER = [['1x', 1, 18], ['2x', 2, 36], ['4x', 4, 72], ['128', 7, 128], ['256', 14, 256], ['512', 28, 512]];
export const ladderFiles = (animated = false, size = 300, sizes = BADGE_LADDER) =>
  sizes.flatMap(([stem, scale, px]) =>
    (animated ? [false, true] : [false]).map((still) => ({ name: `${stem}${still ? '_static' : ''}.webp`, format: 'WEBP', scale, width: px, height: px, size, isStatic: still })));
const cImage = (key, kind = 'BADGE', full = true) => ({
  id: `01JD8XIMG${key.replace(/[^a-z]/g, '').toUpperCase().padEnd(17, '0').slice(0, 17)}`,
  kind, host: `${BASE}/cosmetics/${kind === 'BADGE' ? 'badges' : 'paints'}/${key}`, animated: false,
  width: 512, height: 512, ready: true, createdAt: C_TIME,
  files: ladderFiles(false, 300, full ? BADGE_LADDER : BADGE_LADDER.slice(0, 3)),
});
// The paint size ladder (crates/media Profile::PAINT): 64, 128, 256 and 512 px tall, 4:1 here.
export const PAINT_LADDER = [['1x', 1, 64], ['2x', 2, 128], ['4x', 4, 256], ['512', 8, 512]];
const paintFiles = (animated = false, size = 3000, sizes = PAINT_LADDER) =>
  sizes.flatMap(([stem, scale, px]) =>
    (animated ? [false, true] : [false]).map((still) => ({ name: `${stem}${still ? '_static' : ''}.webp`, format: 'WEBP', scale, width: px * 4, height: px, size, isStatic: still })));
const pImage = (key, full = true) => ({
  id: `01JD8XIMG${key.replace(/[^a-z]/g, '').toUpperCase().padEnd(17, '0').slice(0, 17)}`,
  kind: 'PAINT', host: `${BASE}/cosmetics/paints/${key}`, animated: false, width: 2048, height: 512, ready: true, createdAt: C_TIME,
  files: paintFiles(false, 3000, full ? PAINT_LADDER : PAINT_LADDER.slice(0, 3)),
});
export const UPLOADED = new Map(); // SHA-256 of an uploaded file -> its picture
export const C_IMAGES = [cImage('sample-star', 'BADGE', false), cImage('sample-shield'), pImage('sample-mosaic'), pImage('sample-mosaic-old', false)];
export const BADGES = [
  { id: '01JD8XBDG000000000000000001', name: 'Sample: Early', tooltip: 'Sample badge (mock data)', image: C_IMAGES[0], createdAt: C_TIME, updatedAt: C_TIME },
  { id: '01JD8XBDG000000000000000002', name: 'Sample: Helper', tooltip: 'Sample badge (mock data)', image: C_IMAGES[1], createdAt: C_TIME, updatedAt: C_TIME },
];
const paintOf = (id, name, kind, rest) => ({ id, name, kind, angle: 90, repeat: false, shape: 'CIRCLE', color: null, stops: [], shadows: [], image: null, createdAt: C_TIME, updatedAt: C_TIME, ...rest });
export const PAINTS = [
  paintOf('01JD8XPNT000000000000000001', 'Sample Sunset', 'LINEAR_GRADIENT', { angle: 90, color: '#FF7A45',
    stops: [{ at: 0, color: '#FF7A45' }, { at: 0.5, color: '#FF3D7F' }, { at: 1, color: '#8F5BFF' }] }),
  paintOf('01JD8XPNT000000000000000002', 'Sample Aurora', 'RADIAL_GRADIENT', { shape: 'ELLIPSE', color: '#3DDC97',
    stops: [{ at: 0, color: '#3DDC97' }, { at: 0.6, color: '#3BA7FF' }, { at: 1, color: '#7B61FF' }] }),
  paintOf('01JD8XPNT000000000000000003', 'Sample Ember', 'LINEAR_GRADIENT', { angle: 45, repeat: true, color: '#FFB020',
    stops: [{ at: 0, color: '#FFB020' }, { at: 0.25, color: '#FF5A1F' }, { at: 0.5, color: '#FFB020' }],
    shadows: [{ x: 0, y: 1, radius: 2, color: '#00000099' }] }),
  // Picture paints: the mock draws their pictures as PNGs of the file's own height with detail that
  // scales with it, so a file too small for where it is drawn looks soft. The second is made
  // before the 512 px size (Make larger sizes).
  paintOf('01JD8XPNT000000000000000004', 'Sample Mosaic', 'IMAGE', { color: '#FF7A45', image: C_IMAGES[2] }),
  paintOf('01JD8XPNT000000000000000005', 'Sample Mosaic (old)', 'IMAGE', { color: '#3BA7FF', image: C_IMAGES[3] }),
];
// Who owns what (entitlements) and what each account shows. Two of the made-up channel accounts
// (emoteChannels, k = 4 and 6) wear paints too, so an emote's channel list shows several.
const CHANNEL_ACC = (k) => `01JD8XACC${String(k).padStart(17, '0')}`;
export const C_OWNED = {
  [CHANNEL_ACC(4)]: { BADGE: [], PAINT: [PAINTS[0].id] },
  [CHANNEL_ACC(6)]: { BADGE: [], PAINT: [PAINTS[1].id] },
  [ME_ID]: { BADGE: [BADGES[0].id, BADGES[1].id], PAINT: [PAINTS[0].id, PAINTS[1].id, PAINTS[3].id] },
  [OTHERS[0].id]: { BADGE: [BADGES[1].id], PAINT: [PAINTS[2].id] },
  [OTHERS[4].id]: { BADGE: [], PAINT: [PAINTS[1].id] },
};
export const C_SELECTED = {
  [CHANNEL_ACC(4)]: { badgeId: null, paintId: PAINTS[0].id },
  [CHANNEL_ACC(6)]: { badgeId: null, paintId: PAINTS[1].id },
  [ME_ID]: { badgeId: BADGES[0].id, paintId: PAINTS[3].id },
  [OTHERS[0].id]: { badgeId: BADGES[1].id, paintId: PAINTS[2].id },
  [OTHERS[4].id]: { badgeId: null, paintId: PAINTS[1].id },
};
export const C_GRANTS = Object.entries(C_OWNED).flatMap(([account, kinds]) => Object.entries(kinds).flatMap(([kind, list]) =>
  list.map((cosmeticId) => ({ accountId: account, kind, cosmeticId, source: 'GRANT', reason: 'Sample grant (mock data)', grantedBy: ME_SUMMARY, grantedAt: C_TIME }))));
// Chatters the batch lookup knows (platform user id -> account).
const C_CHATTERS = { '141981764': ME_ID, '55500001': OTHERS[0].id, '55500003': OTHERS[4].id };
let cosmeticSeq = 0;
// The next number for an id made here (badges, paints and uploaded pictures share it).
export const nextCosmeticSeq = () => cosmeticSeq++;
export const cosmeticCatalog = () => ({ badges: BADGES, paints: PAINTS });
// What the public reads list (GraphQL cosmetics, GET /v1/cosmetics): only cosmetics held by an
// account or in a shop item on sale. Staff with a cosmetics.* permission (the mock's admin) get all.
export const releasedCatalog = (staff) => {
  if (staff) return cosmeticCatalog();
  const now = Date.now();
  const onSale = SHOP_ITEMS.filter((i) => i.visible !== false && !i.deleted && (!i.activeFrom || Date.parse(i.activeFrom) <= now) && (!i.activeTo || Date.parse(i.activeTo) > now));
  const held = (kind, id) => Object.values(C_OWNED).some((o) => o[kind].includes(id));
  const sold = (key, id) => onSale.some((i) => (i[key] ?? []).includes(id));
  return { badges: BADGES.filter((b) => held('BADGE', b.id) || sold('badgeIds', b.id)), paints: PAINTS.filter((p) => held('PAINT', p.id) || sold('paintIds', p.id)) };
};
const ownedOut = (accountId) => {
  const owned = C_OWNED[accountId] ?? { BADGE: [], PAINT: [] };
  const sel = C_SELECTED[accountId] ?? { badgeId: null, paintId: null };
  return { account: publicAccount(accountId) ?? { id: accountId, displayName: '', avatar: null },
    badges: BADGES.filter((b) => owned.BADGE.includes(b.id)), paints: PAINTS.filter((p) => owned.PAINT.includes(p.id)),
    badgeId: sel.badgeId, paintId: sel.paintId };
};
export const userCosmeticsOut = (platform, ids = []) => {
  if (ids.length > 100) return null;
  return [...new Set(ids)].filter((id) => platform === 'TWITCH' && C_CHATTERS[id]).sort().map((id) => {
    const sel = C_SELECTED[C_CHATTERS[id]] ?? {};
    return { platformUserId: id, badgeId: sel.badgeId ?? null, paintId: sel.paintId ?? null };
  }).filter((u) => u.badgeId || u.paintId);
};
const entitlementOut = (g) => ({ account: publicAccount(g.accountId) ?? ME_SUMMARY, kind: g.kind, cosmeticId: g.cosmeticId,
  source: g.source, reason: g.reason, grantedBy: g.grantedBy, grantedAt: g.grantedAt });
const dropSelections = (kind, id) => {
  for (const sel of Object.values(C_SELECTED)) if (sel[kind === 'BADGE' ? 'badgeId' : 'paintId'] === id) sel[kind === 'BADGE' ? 'badgeId' : 'paintId'] = null;
};
const hexColor = (c) => typeof c === 'string' && /^#([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(c);
function checkPaint(input) {
  if (!input || typeof input.name !== 'string' || !input.name.trim() || input.name.trim().length > 32) return ['Names are 1 to 32 characters.', 'bad_name'];
  if (input.angle != null && (input.angle < 0 || input.angle > 360)) return ['The angle is 0 to 360 degrees.', 'bad_angle'];
  if (input.color && !hexColor(input.color)) return ['Use #RRGGBB or #RRGGBBAA.', 'bad_color'];
  const stops = input.stops ?? [];
  if (input.kind === 'IMAGE') {
    if (stops.length) return ['IMAGE paints take no stops.', 'bad_stops'];
    if (!input.imageId) return ['IMAGE paints need an imageId.', 'bad_image'];
  } else {
    if (input.imageId) return ['Only IMAGE paints take an image.', 'bad_image'];
    if (stops.length < 2 || stops.length > 10) return ['Gradients take 2 to 10 stops.', 'bad_stops'];
    let last = 0;
    for (const st of stops) {
      if (!(st.at >= 0 && st.at <= 1) || st.at < last) return ['Stops go from 0 to 1, each at or after the one before.', 'bad_stops'];
      if (!hexColor(st.color)) return ['Use #RRGGBB or #RRGGBBAA.', 'bad_color'];
      last = st.at;
    }
  }
  const shadows = input.shadows ?? [];
  if (shadows.length > 4) return ['At most 4 shadows.', 'bad_shadows'];
  for (const sh of shadows) {
    if (Math.abs(sh.x) > 16 || Math.abs(sh.y) > 16 || sh.radius < 0 || sh.radius > 16) return ['Shadow offsets are -16 to 16 px, the radius 0 to 16 px.', 'bad_shadows'];
    if (!hexColor(sh.color)) return ['Use #RRGGBB or #RRGGBBAA.', 'bad_color'];
  }
  return null;
}
const upper = (c) => (c ? c.toUpperCase() : c);
const paintFrom = (id, input, before) => ({
  ...(before ?? { createdAt: new Date().toISOString() }), id, name: input.name.trim(), kind: input.kind,
  angle: input.angle ?? 90, repeat: input.repeat ?? false, shape: input.shape ?? 'CIRCLE', color: upper(input.color) || null,
  stops: (input.stops ?? []).map((st) => ({ at: st.at, color: upper(st.color) })),
  shadows: (input.shadows ?? []).map((sh) => ({ ...sh, color: upper(sh.color) })),
  image: input.imageId ? C_IMAGES.find((i) => i.id === input.imageId) ?? null : null, updatedAt: new Date().toISOString(),
});

// The cosmetics root fields. Returns null when the operation has none.
export function cosmeticsAnswer(q, v, session, admin) {
  const has = (field) => new RegExp(`\\b${field}\\s*[({]`).test(q) || new RegExp(`\\b${field}\\s*$`, 'm').test(q);
  const mutation = /^\s*mutation/.test(q);
  const staff = (perm) => (!session ? err('Sign in to use this.', 'signed_out') : admin ? null : err(`This needs ${perm}.`, 'forbidden'));
  if (!mutation) {
    const fields = ['cosmetics', 'userCosmetics', 'myCosmetics', 'cosmeticImage', 'cosmeticEntitlements'].filter(has);
    if (!fields.length) return null;
    const data = {};
    for (const f of fields) {
      if (f === 'cosmetics') data.cosmetics = releasedCatalog(admin);
      if (f === 'userCosmetics') {
        const out = userCosmeticsOut(v.platform ?? 'TWITCH', v.userIds ?? []);
        if (!out) return err('Look up at most 100 users at once.', 'too_many');
        data.userCosmetics = out;
      }
      if (f === 'myCosmetics') {
        if (!session) return err('Sign in to use this.', 'signed_out');
        const who = v.accountId || ME_ID;
        if (who !== ME_ID && !flagsOn(who).includes('PROFILE')) return err('Only the account and its editors with the profile permission can do this.', 'forbidden');
        data.myCosmetics = ownedOut(who);
      }
      if (f === 'cosmeticImage') {
        const denied = staff('cosmetics.manage');
        if (denied) return denied;
        data.cosmeticImage = C_IMAGES.find((i) => i.id === v.id) ?? null;
      }
      if (f === 'cosmeticEntitlements') {
        const denied = staff('cosmetics.grant');
        if (denied) return denied;
        const rows = C_GRANTS.filter((g) => g.kind === (v.kind ?? 'BADGE') && (!v.accountId || g.accountId === v.accountId) && (!v.cosmeticId || g.cosmeticId === v.cosmeticId));
        const per = Math.min(100, Math.max(1, v.perPage ?? 50));
        const page = Math.max(1, v.page ?? 1);
        data.cosmeticEntitlements = { total: rows.length, items: rows.slice((page - 1) * per, page * per).map(entitlementOut) };
      }
    }
    return { data };
  }
  if (has('selectCosmetics')) {
    if (!session) return err('Sign in to use this.', 'signed_out');
    const who = v.ownerId || ME_ID;
    if (who !== ME_ID && !flagsOn(who).includes('PROFILE')) return err('Only the account and its editors with the profile permission can do this.', 'forbidden');
    const owned = C_OWNED[who] ?? { BADGE: [], PAINT: [] };
    const sel = (C_SELECTED[who] ??= { badgeId: null, paintId: null });
    const next = { ...sel };
    if ('badgeId' in v) next.badgeId = v.badgeId || null;
    if ('paintId' in v) next.paintId = v.paintId || null;
    if ((next.badgeId && !owned.BADGE.includes(next.badgeId)) || (next.paintId && !owned.PAINT.includes(next.paintId)))
      return err('You can only show cosmetics the account owns.', 'not_owned');
    Object.assign(sel, next);
    return { data: { selectCosmetics: ownedOut(who) } };
  }
  if (has('remakeCosmeticImage')) {
    const denied = staff('cosmetics.manage');
    if (denied) return denied;
    const img = C_IMAGES.find((i) => i.id === v.id);
    if (!img) return err('Not found.', 'not_found');
    // The job runs again: a badge gets every size of the ladder a moment later.
    if (img.kind === 'BADGE')
      setTimeout(() => Object.assign(img, { files: ladderFiles(img.animated, img.files[0]?.size ?? 300) }), Number(process.env.MOCK_IMAGE_DELAY ?? 2000));
    // A paint gets the 512 px tall file (4:1 pictures here, as the originals the mock keeps).
    if (img.kind === 'PAINT' && !img.files.some((f) => f.name === '512.webp'))
      setTimeout(() => Object.assign(img, { files: paintFiles(img.animated, img.files[0]?.size ?? 3000) }), Number(process.env.MOCK_IMAGE_DELAY ?? 2000));
    return { data: { remakeCosmeticImage: img } };
  }
  for (const [field, perm] of [['createBadge', 'manage'], ['updateBadge', 'manage'], ['deleteBadge', 'manage'], ['createPaint', 'manage'],
    ['updatePaint', 'manage'], ['deletePaint', 'manage'], ['grantCosmetic', 'grant'], ['revokeCosmetic', 'grant']]) {
    if (!has(field)) continue;
    const denied = staff(`cosmetics.${perm}`);
    if (denied) return denied;
    const input = v.input ?? {};
    switch (field) {
      case 'createBadge':
      case 'updateBadge': {
        const before = field === 'updateBadge' ? BADGES.find((b) => b.id === v.id) : null;
        if (field === 'updateBadge' && !before) return err('Not found.', 'not_found');
        const name = input.name ?? before?.name;
        if (typeof name !== 'string' || !name.trim() || name.trim().length > 32) return err('Names are 1 to 32 characters.', 'bad_name');
        if (BADGES.some((b) => b !== before && b.name.toLowerCase() === name.trim().toLowerCase())) return err('A badge with that name exists already.', 'name_taken');
        const tooltip = input.tooltip ?? before?.tooltip;
        if (typeof tooltip !== 'string' || !tooltip.trim() || tooltip.trim().length > 64) return err('Tooltips are 1 to 64 characters.', 'bad_tooltip');
        const image = input.imageId ? C_IMAGES.find((i) => i.id === input.imageId && i.kind === 'BADGE') : before?.image;
        if (!image) return err('No badge picture with that ID.', 'bad_image');
        if (!image.ready) return err('That picture\u2019s files are still being made. Try again in a moment.', 'image_not_ready');
        const now = new Date().toISOString();
        const badge = { ...(before ?? { id: `01JD8XBDG${String(100 + nextCosmeticSeq()).padStart(17, '0')}`, createdAt: now }), name: name.trim(), tooltip: tooltip.trim(), image, updatedAt: now };
        if (before) BADGES[BADGES.indexOf(before)] = badge;
        else BADGES.push(badge);
        return { data: { [field]: badge } };
      }
      case 'deleteBadge': {
        const i = BADGES.findIndex((b) => b.id === v.id);
        if (i >= 0) {
          BADGES.splice(i, 1);
          for (const o of Object.values(C_OWNED)) o.BADGE = o.BADGE.filter((id) => id !== v.id);
          dropSelections('BADGE', v.id);
        }
        return { data: { deleteBadge: i >= 0 } };
      }
      case 'createPaint':
      case 'updatePaint': {
        const before = field === 'updatePaint' ? PAINTS.find((x) => x.id === v.id) : null;
        if (field === 'updatePaint' && !before) return err('Not found.', 'not_found');
        const bad = checkPaint(input);
        if (bad) return err(...bad);
        if (PAINTS.some((x) => x !== before && x.name.toLowerCase() === input.name.trim().toLowerCase())) return err('A paint with that name exists already.', 'name_taken');
        if (input.imageId && !C_IMAGES.some((i) => i.id === input.imageId && i.kind === 'PAINT')) return err('No paint picture with that ID.', 'bad_image');
        const paint = paintFrom(before?.id ?? `01JD8XPNT${String(100 + nextCosmeticSeq()).padStart(17, '0')}`, input, before);
        if (before) PAINTS[PAINTS.indexOf(before)] = paint;
        else PAINTS.push(paint);
        return { data: { [field]: paint } };
      }
      case 'deletePaint': {
        const i = PAINTS.findIndex((x) => x.id === v.id);
        if (i >= 0) {
          PAINTS.splice(i, 1);
          for (const o of Object.values(C_OWNED)) o.PAINT = o.PAINT.filter((id) => id !== v.id);
          dropSelections('PAINT', v.id);
        }
        return { data: { deletePaint: i >= 0 } };
      }
      case 'grantCosmetic': {
        const { accountId, kind, cosmeticId, reason } = input;
        if (!publicAccount(accountId) || !(kind === 'BADGE' ? BADGES : PAINTS).some((c) => c.id === cosmeticId)) return err('Not found.', 'not_found');
        if (typeof reason !== 'string' || !reason.trim()) return err('Give a reason of 1 to 500 characters.', 'bad_reason');
        const owned = (C_OWNED[accountId] ??= { BADGE: [], PAINT: [] });
        if (owned[kind].includes(cosmeticId)) return err('They own that already.', 'already_owned');
        owned[kind].push(cosmeticId);
        const g = { accountId, kind, cosmeticId, source: 'GRANT', reason: reason.trim(), grantedBy: ME_SUMMARY, grantedAt: new Date().toISOString() };
        C_GRANTS.unshift(g);
        return { data: { grantCosmetic: entitlementOut(g) } };
      }
      case 'revokeCosmetic': {
        const owned = C_OWNED[v.accountId];
        const had = Boolean(owned?.[v.kind]?.includes(v.cosmeticId));
        if (had) {
          owned[v.kind] = owned[v.kind].filter((id) => id !== v.cosmeticId);
          const i = C_GRANTS.findIndex((g) => g.accountId === v.accountId && g.kind === v.kind && g.cosmeticId === v.cosmeticId);
          if (i >= 0) C_GRANTS.splice(i, 1);
          const sel = C_SELECTED[v.accountId];
          const key = v.kind === 'BADGE' ? 'badgeId' : 'paintId';
          if (sel?.[key] === v.cosmeticId) sel[key] = null;
        }
        return { data: { revokeCosmetic: had } };
      }
    }
  }
  return null;
}
