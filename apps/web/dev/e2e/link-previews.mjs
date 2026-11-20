// Link previews in the server-rendered HTML: fetches each public page type the way a crawler does
// (no JavaScript, no cookie) and checks its Open Graph and Twitter tags. Needs the dev mock and
// vite dev (or a preview build) running; not in CI.
//
//   BASE=http://localhost:5173 node dev/e2e/link-previews.mjs
//
// It starts from a channel page and follows its links to the set and the creator, and takes an
// emote from /search, so it needs no ids of its own.
const BASE = (process.env.BASE ?? 'http://localhost:5173').replace(/\/+$/, '');
let failed = 0;

async function html(path) {
  const r = await fetch(`${BASE}${path}`, { redirect: 'follow', headers: { 'User-Agent': 'link-preview-check' } });
  if (!r.ok) throw new Error(`${path}: ${r.status}`);
  return r.text();
}

/** Every <meta>/<link rel=canonical> in the head, as [key, value] pairs (in order; keys repeat). */
function tags(page) {
  const head = page.slice(0, page.indexOf('</head>'));
  const out = [];
  for (const m of head.matchAll(/<meta\s+([^>]*?)\/?>/g)) {
    const attr = Object.fromEntries([...m[1].matchAll(/([\w:-]+)="([^"]*)"/g)].map(([, k, v]) => [k, v]));
    const key = attr.property ?? attr.name;
    if (key) out.push([key, attr.content ?? '']);
  }
  const canonical = head.match(/<link\s+rel="canonical"\s+href="([^"]*)"/);
  if (canonical) out.push(['canonical', canonical[1]]);
  return out;
}

const one = (t, key) => t.filter(([k]) => k === key).map(([, v]) => v);

function check(label, ok, detail = '') {
  if (!ok) failed += 1;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${detail ? `  (${detail})` : ''}`);
}

function expectCard(kind, path, page, want) {
  const t = tags(page);
  for (const key of ['og:title', 'og:description', 'og:url', 'og:image', 'og:site_name', 'twitter:card', 'canonical', 'description']) {
    const values = one(t, key);
    // og:image may repeat (a set's first emotes); everything else exactly once.
    check(`${kind} ${key}`, key === 'og:image' ? values.length >= 1 : values.length === 1, values.join(' | ').slice(0, 120));
  }
  check(`${kind} canonical is the page`, one(t, 'canonical')[0]?.endsWith(path), one(t, 'canonical')[0]);
  check(`${kind} og:url = canonical`, one(t, 'og:url')[0] === one(t, 'canonical')[0]);
  check(`${kind} images are absolute`, one(t, 'og:image').every((u) => /^https?:\/\//.test(u)));
  check(`${kind} card`, one(t, 'twitter:card')[0] === want.card, one(t, 'twitter:card')[0]);
  check(`${kind} title`, want.title.test(one(t, 'og:title')[0] ?? ''), one(t, 'og:title')[0]);
  check(`${kind} robots`, one(t, 'robots').includes('noindex') === want.noindex, one(t, 'robots').join(',') || 'indexed');
}

const channelPath = '/channels/twitch/lumen_fox';
const channel = await html(channelPath);
expectCard('channel', channelPath, channel, { card: 'summary', title: /on Twitch$/, noindex: false });

const setPath = channel.match(/href="(\/sets\/[^"#?]+)"/)?.[1];
const userPath = channel.match(/href="(\/users\/[^"#?]+)"/)?.[1];
check('channel links its set and creator', !!setPath && !!userPath);
if (setPath) {
  const set = await html(setPath);
  expectCard('set', setPath, set, { card: 'summary', title: /\S/, noindex: false });
  check('set leads with emote stills', one(tags(set), 'og:image').every((u) => /\/\dx(_static)?\.webp$/.test(u)));
}
if (userPath) expectCard('profile', userPath, await html(userPath), { card: 'summary', title: /\S/, noindex: false });

const search = await html('/search');
const emotePath = search.match(/href="(\/emotes\/[^"#?]+)"/)?.[1];
check('search lists an emote', !!emotePath);
if (emotePath) {
  const emote = await html(emotePath);
  expectCard('emote', emotePath, emote, { card: 'summary', title: /\S/, noindex: false });
  check('emote image is the 4x still', /\/4x(_static)?\.webp$/.test(one(tags(emote), 'og:image')[0] ?? ''));
}

const missing = await fetch(`${BASE}/emotes/00000000000000000000000000`).then((r) => r.text());
check('a missing emote is not indexed', one(tags(missing), 'robots').includes('noindex'));

// Any other page: the site card, and the page's own description and title stand.
const help = await html('/help');
const ht = tags(help);
check('other pages: the wide site card', one(ht, 'og:image')[0]?.endsWith('/og.png') && one(ht, 'twitter:card')[0] === 'summary_large_image');
check('other pages: one og:title (the page’s own)', one(ht, 'og:title').length === 1 && one(ht, 'og:title')[0] === 'Help');
check('other pages: one description', one(ht, 'description').length === 1);
check('og.png is served', (await fetch(`${BASE}/og.png`)).headers.get('content-type')?.includes('image/png'));

console.log(failed ? `\n${failed} failed` : '\nall passed');
process.exit(failed ? 1 : 0);
