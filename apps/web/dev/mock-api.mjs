// A stand-in for api.cyex.tv, for looking at the site without the real stack:
//   node dev/mock-api.mjs            (listens on 127.0.0.1:8787)
//   PUBLIC_API_URL=http://127.0.0.1:8787 npm run dev
// It answers the site's GraphQL operations with made-up emotes and draws their pictures as
// simple shapes. Development only; nothing here ships.
import { createServer } from 'node:http';

const PORT = Number(process.env.PORT ?? 8787);
const ORIGIN = process.env.SITE_ORIGIN ?? 'http://localhost:5173';
const BASE = `http://127.0.0.1:${PORT}`;

const NAMES = ['catJAM', 'pogHop', 'quietoak', 'lumenWave', 'noraWow', 'pixelmoth', 'byteNod', 'foxLook',
  'oakCry', 'moonHi', 'sipTea', 'clapclap', 'wideSmile', 'thinking', 'rainyDay', 'gg', 'okBuddy', 'sleepy',
  'hype', 'shrug', 'lgtm', 'wave', 'coolCat', 'tinyHeart'];
const HUES = [210, 25, 140, 280, 340, 50, 180, 0];

const emote = (i) => {
  const name = NAMES[i % NAMES.length] + (i >= NAMES.length ? String(i) : '');
  const wide = i % 7 === 3;
  const w = wide ? 64 : 32;
  return {
    id: `01JD8X${String(i).padStart(20, '0')}`,
    name,
    state: 'APPROVED',
    animated: i % 4 === 1,
    tags: i % 3 === 0 ? ['funny', 'cat'] : [],
    host: `${BASE}/emotes/${i}`,
    createdAt: '2026-11-16T20:00:00.000Z',
    flags: { zeroWidth: false, nsfw: false, private: false },
    owner: { id: '01JD8X0000000000000000OWNR', displayName: ['lumen_fox', 'quietoak', 'nora_byte'][i % 3], avatar: null },
    files: [1, 2, 3, 4].map((s) => ({ name: `${s}x.webp`, format: 'WEBP', scale: s, width: w * s, height: 32 * s, size: 900 * s, isStatic: false })),
  };
};
const ALL = Array.from({ length: 140 }, (_, i) => emote(i));

function answer(body) {
  const q = body.query ?? '';
  const v = body.variables ?? {};
  if (q.includes('query Me')) return { data: null, errors: [{ message: 'Sign in to use this.', extensions: { code: 'signed_out' } }] };
  if (q.includes('query GlobalSet')) return { data: { globalSet: { id: 'G', name: 'Global', emotes: ALL.slice(0, 24).map((e) => ({ name: e.name, zeroWidth: false, emote: e })) } } };
  if (q.includes('query Search')) {
    const text = (v.query ?? '').toLowerCase();
    let hits = ALL.filter((e) => !text || e.name.toLowerCase().includes(text) || e.tags.some((t) => t.startsWith(text)));
    if (v.animated === true) hits = hits.filter((e) => e.animated);
    if (v.animated === false) hits = hits.filter((e) => !e.animated);
    const per = v.perPage ?? 50;
    const page = v.page ?? 1;
    return { data: { searchEmotes: { total: hits.length, page, perPage: per, items: hits.slice((page - 1) * per, page * per) } } };
  }
  if (q.includes('query Emote(')) return { data: { emote: ALL.find((e) => e.id === v.id) ?? null } };
  if (q.includes('query Channel')) {
    if (v.login !== 'lumen_fox') return { data: { channelByLogin: null } };
    return { data: { channelByLogin: { platform: v.platform, platformChannelId: '141981764', login: 'lumen_fox', activeSetId: 'S1', allowPersonalEmotes: true, account: { id: 'A', displayName: 'lumen_fox', avatar: null } } } };
  }
  if (q.includes('query Set(')) return { data: { set: { id: v.id, name: 'Main', kind: 'CHANNEL', capacity: 1000, owner: { id: 'A', displayName: 'lumen_fox' }, emotes: ALL.slice(10, 46).map((e) => ({ name: e.name, zeroWidth: false, addedAt: e.createdAt, emote: e })) } } };
  return { data: null, errors: [{ message: 'Not in the mock.', extensions: { code: 'not_implemented' } }] };
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

createServer((req, res) => {
  res.setHeader('access-control-allow-origin', ORIGIN);
  res.setHeader('access-control-allow-credentials', 'true');
  res.setHeader('access-control-allow-headers', 'content-type');
  if (req.method === 'OPTIONS') return res.writeHead(204).end();
  const img = req.url?.match(/^\/emotes\/(\d+)\/(\d)x(?:_static)?\.webp$/);
  if (img) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(picture(Number(img[1]), Number(img[2])));
  }
  if (req.method === 'POST' && req.url === '/v1/gql') {
    let raw = '';
    req.on('data', (c) => (raw += c));
    req.on('end', () => {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify(answer(JSON.parse(raw || '{}'))));
    });
    return;
  }
  res.writeHead(404).end();
}).listen(PORT, '127.0.0.1', () => console.log(`mock api on ${BASE}`));
