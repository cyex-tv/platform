// Cosmetics end to end in a real browser, through the UI the way staff use it, against the dev
// mock (which keeps and serves the pictures it is sent, as the API and worker do).
//   1. Badge: Admin → Badges → New badge → a wrong picture (says why, nothing is sent) → the right
//      one through the file chooser, then "Make badge" at once while it is still processing → the
//      upload goes out, the badge is made and shows the uploaded picture itself; the same file can
//      be picked again; the badge shows on the profile.
//   2. Paint: Admin → Paints → New paint → Animated → name → Save to cyex.tv → the studio's render
//      is uploaded → give and select it → the profile band, the account menu, the cosmetics picker,
//      the gallery and the admin list draw that render (painted pixels in several colours), in both
//      themes; the picker has no grid/list switch.
// Exits 0 only when every check passes.
//
//   MOCK_ADMIN=1 PORT=9311 SITE_ORIGIN=http://localhost:5711 node dev/mock-api.mjs
//   PUBLIC_API_URL=http://localhost:9311 npx vite dev --port 5711
//   msedge --headless=new --remote-debugging-port=9811 --user-data-dir=<tmp> about:blank
//   SITE=http://localhost:5711 API=http://localhost:9311 CDP=9811 node dev/e2e/cosmetics.mjs
import { mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { paintedPixels, png } from './png.mjs';

const SITE = process.env.SITE ?? 'http://localhost:5711';
const API = process.env.API ?? 'http://localhost:9311';
const ME = '01JD8X0000000000000000MEME';
const OUT = process.env.OUT ?? join(tmpdir(), 'cyex-e2e-cosmetics');
mkdirSync(OUT, { recursive: true });
// Test pictures: a 96 px square badge, and a 100 × 80 one the API refuses.
const BADGE = png(96, 96, (x, y) => [x * 2, y * 2, 255 - x * 2, 255]);
const BADGE_BAD = png(100, 80, (x, y) => [255, x * 2, y * 3, 255]);
writeFileSync(join(OUT, 'badge-96.png'), BADGE);
writeFileSync(join(OUT, 'badge-bad.png'), BADGE_BAD);

async function launch(port) {
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
  const ws = new WebSocket(tabs.find((t) => t.type === 'page').webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener('open', r));
  let id = 0;
  const pend = new Map(), handlers = [];
  ws.addEventListener('message', (e) => {
    const m = JSON.parse(e.data);
    if (m.id && pend.has(m.id)) {
      const { res, rej } = pend.get(m.id);
      pend.delete(m.id);
      m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result);
    } else handlers.forEach((h) => h(m));
  });
  return {
    send: (method, params = {}) => new Promise((res, rej) => { const i = ++id; pend.set(i, { res, rej }); ws.send(JSON.stringify({ id: i, method, params })); }),
    on: (h) => handlers.push(h),
    close: async () => ws.close(),
  };
}

const b = await launch(Number(process.env.CDP ?? 9811));
const reqs = [];
const logs = [];
let chooser = null;
b.on((m) => {
  if (m.method === 'Network.requestWillBeSent') reqs.push({ method: m.params.request.method, url: m.params.request.url, body: m.params.request.postData ?? '' });
  if (m.method === 'Page.fileChooserOpened') chooser = m.params;
  if (m.method === 'Runtime.exceptionThrown') logs.push('EXC ' + (m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text));
  if (m.method === 'Log.entryAdded' && /Content Security|Refused/.test(m.params.entry.text)) logs.push('CSP ' + m.params.entry.text);
});
await b.send('Network.enable'); await b.send('Runtime.enable'); await b.send('Log.enable'); await b.send('Page.enable'); await b.send('DOM.enable');
await b.send('Network.setCookie', { name: 'mock_session', value: '1', url: API });
await b.send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 1000, deviceScaleFactor: 2, mobile: false });
await b.send('Page.setInterceptFileChooserDialog', { enabled: true });
const ev = async (expr) => {
  const r = await b.send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(`${expr.slice(0, 80)}: ${r.exceptionDetails.exception?.description}`);
  return r.result.value;
};
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
async function until(expr, ms = 15000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (await ev(expr)) return true;
    await wait(200);
  }
  return false;
}
async function click(sel) {
  const r = await ev(`(() => { const el = ${sel}; if (!el) return null; el.scrollIntoView({ block: 'center' }); const b = el.getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })()`);
  if (!r) throw new Error('no element: ' + sel);
  for (const type of ['mousePressed', 'mouseReleased']) await b.send('Input.dispatchMouseEvent', { type, x: r[0], y: r[1], button: 'left', clickCount: 1 });
}
async function typeInto(sel, text) {
  await click(sel);
  await ev(`(${sel}).select?.()`);
  await b.send('Input.insertText', { text });
}
async function choose(sel, file) {
  chooser = null;
  await click(sel);
  if (!(await until('true', 100)) || !(await (async () => { for (let i = 0; i < 30 && !chooser; i++) await wait(100); return chooser; })()))
    throw new Error('no file chooser opened for ' + sel);
  await b.send('DOM.setFileInputFiles', { files: [join(OUT, file)], backendNodeId: chooser.backendNodeId });
}
const shot = async (name) => writeFileSync(join(OUT, `${name}.png`), Buffer.from((await b.send('Page.captureScreenshot', { format: 'png' })).data, 'base64'));
const gql = async (query, variables) => (await fetch(`${API}/v1/gql`, { method: 'POST', headers: { cookie: 'mock_session=1', origin: SITE, 'content-type': 'application/json' }, body: JSON.stringify({ query, variables }) })).json();
const results = [];
const check = (name, ok, detail = '') => { results.push({ name, ok: !!ok, detail }); console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` (${detail})` : ''}`); };
const theme = async (t) => {
  await b.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: t }] });
  await ev(`try { localStorage.setItem('theme', '${t}') } catch {}; document.documentElement.dataset.theme = '${t}'`);
};
async function go(path, t = 'dark') {
  await b.send('Page.navigate', { url: SITE + path });
  await until(`document.readyState === 'complete'`);
  await wait(2500);
  await theme(t);
  await wait(300);
}
/** Painted pixels and colour variety of an element, measured from a screenshot (measure.py's rules). */
async function measure(sel, name) {
  const r = await ev(`(() => { const e = ${sel}; if (!e) return null; e.scrollIntoView({ block: 'center' }); const b = e.getBoundingClientRect(); return { x: b.x + scrollX, y: b.y + scrollY, w: b.width, h: b.height, bg: getComputedStyle(e).backgroundImage }; })()`);
  if (!r || r.w < 2) return { ok: false, why: 'not on the page', r };
  const { data } = await b.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip: { x: r.x - 3, y: r.y - 3, width: r.w + 6, height: r.h + 6, scale: 1 } });
  const buf = Buffer.from(data, 'base64');
  writeFileSync(join(OUT, `clip-${name}.png`), buf);
  const m = paintedPixels(buf);
  return { ...m, bg: r.bg };
}

// ---------------------------------------------------------------- 1. badge
await theme('dark');
await go('/admin/cosmetics/badges');
await until(`[...document.querySelectorAll('button')].some((b) => /New badge/.test(b.textContent))`, 30000);
await click(`[...document.querySelectorAll('button')].find((b) => /New badge/.test(b.textContent))`);
await until(`!!document.querySelector('dialog[open]')`);
const badgeName = `E2E badge ${Date.now() % 100000}`;
await typeInto(`document.querySelectorAll('dialog[open] input.ui-input')[0]`, badgeName);
await typeInto(`document.querySelectorAll('dialog[open] input.ui-input')[1]`, 'Made through the UI');
const pickSel = `[...document.querySelectorAll('dialog[open] label')].find((l) => l.querySelector('input[type=file]'))`;
let before = reqs.length;
await choose(pickSel, 'badge-bad.png');
await until(`!!document.querySelector('dialog[open] .ui-error')`, 4000);
const wrongMsg = await ev(`document.querySelector('dialog[open] .ui-error')?.textContent ?? ''`);
check('a non-square badge picture says why on the page', /square and at least 72/.test(wrongMsg), wrongMsg);
check('…and sends nothing', !reqs.slice(before).some((r) => r.url.endsWith('/v1/cosmetics/images')));
before = reqs.length;
await choose(pickSel, 'badge-96.png');
await until(`/Uploading|Making/.test(document.querySelector('dialog[open] [role=status]')?.textContent ?? '')`, 3000);
// "Make badge" straight away, while the picture is still being made: it waits, then saves.
await click(`document.querySelector('dialog[open] button[type=submit]')`);
check('the picture upload request goes out', reqs.slice(before).some((r) => r.method === 'POST' && r.url === `${API}/v1/cosmetics/images`));
const made = await until(`!document.querySelector('dialog[open]') && [...document.querySelectorAll('.tile .nm')].some((e) => e.textContent.includes(${JSON.stringify(badgeName)}))`, 20000);
check('Make badge while processing waits and makes the badge', made);
check('createBadge was sent with the uploaded picture', reqs.slice(before).some((r) => r.url.endsWith('/v1/gql') && r.body.includes('createBadge')));
await shot('badge-list');
const badgeImg = await ev(`(() => { const li = [...document.querySelectorAll('.tile')].find((l) => l.querySelector('.nm')?.textContent.includes(${JSON.stringify(badgeName)})); const i = li?.querySelector('img'); return i ? { src: i.src, w: i.naturalWidth } : null; })()`);
check('the badge shows the uploaded picture (not a sample)', badgeImg?.src.includes('/cosmetics/uploads/') && badgeImg.w > 0, badgeImg?.src);
if (badgeImg) {
  const served = Buffer.from(await (await fetch(badgeImg.src)).arrayBuffer());
  check('the served badge file is the uploaded file', served.equals(BADGE));
}
// the same file again, from a fresh dialog: a new change event, a new request
await click(`[...document.querySelectorAll('button')].find((b) => /New badge/.test(b.textContent))`);
await until(`!!document.querySelector('dialog[open]')`);
check('a new dialog starts without the last picture', /Choose a picture/.test(await ev(`document.querySelector('dialog[open] label.ui-btn')?.textContent ?? ''`)));
before = reqs.length;
await choose(pickSel, 'badge-96.png');
await wait(800);
check('picking the same file again sends it again', reqs.slice(before).some((r) => r.url === `${API}/v1/cosmetics/images`));
await click(`[...document.querySelectorAll('dialog[open] button')].find((b) => /Cancel/.test(b.textContent))`);
// the badge everywhere: give it, show it, look at the header and the profile
const badgeId = (await gql('{ cosmetics { badges { id name } } }')).data.cosmetics.badges.find((x) => x.name === badgeName)?.id;
await gql('mutation($input: GrantInput!) { grantCosmetic(input: $input) { kind } }', { input: { accountId: ME, kind: 'BADGE', cosmeticId: badgeId, reason: 'e2e' } });
await gql('mutation($badgeId: ID) { selectCosmetics(badgeId: $badgeId) { badgeId } }', { badgeId });
await go(`/users/${ME}`);
const bandBadge = await ev(`document.querySelector('h1 img.badge')?.src ?? ''`);
check('the profile band shows the new badge', bandBadge.includes('/cosmetics/uploads/'), bandBadge);

// ---------------------------------------------------------------- 2. paint from the studio
await go('/admin/cosmetics/paints/studio');
await click(`[...document.querySelectorAll('button, [role=radio]')].find((b) => /Animated or picture/.test(b.textContent))`);
await until(`/Rendering preview|Paused/.test(document.body.innerText) || !!document.querySelector('.ps-stage .ps-painted[style*="blob:"]')`, 20000);
const paintName = `Icy Wave ${Date.now() % 100000}`;
await typeInto(`document.querySelector('#ps-name')`, paintName);
before = reqs.length;
await click(`[...document.querySelectorAll('button')].find((b) => /Save to cyex.tv/.test(b.textContent))`);
const savedOk = await until(`/Saved/.test(document.querySelector('#ps-status')?.textContent ?? '')`, 60000);
check('the studio saves an animated paint', savedOk, await ev(`document.querySelector('#ps-status')?.textContent ?? ''`));
check('the studio render was uploaded', reqs.slice(before).some((r) => r.method === 'POST' && r.url === `${API}/v1/cosmetics/images`));
await shot('studio-saved');
await until(`(document.querySelector('.ps-stage .ps-painted')?.getAttribute('style') ?? '').includes('url(')`, 10000);
const studioCss = await ev(`document.querySelector('.ps-stage .ps-painted')?.getAttribute('style') ?? ''`);
const paintRow = (await gql('{ cosmetics { paints { id name kind image { id host animated files { name } } } } }')).data.cosmetics.paints.find((p) => p.name === paintName);
check('the saved paint is an image paint with an uploaded picture', paintRow?.kind === 'IMAGE' && paintRow.image?.host.includes('/cosmetics/uploads/'), paintRow?.image?.host);
await gql('mutation($input: GrantInput!) { grantCosmetic(input: $input) { kind } }', { input: { accountId: ME, kind: 'PAINT', cosmeticId: paintRow.id, reason: 'e2e' } });
await gql('mutation($paintId: ID) { selectCosmetics(paintId: $paintId) { paintId } }', { paintId: paintRow.id });
check('the studio preview and the site draw with the same declarations (one renderer)', /background-clip: text/.test(studioCss) && /text-fill-color: transparent/.test(studioCss) && /linear-gradient\(currentcolor, currentcolor\)/i.test(studioCss), studioCss.slice(0, 120));

const surfaces = [
  ['band', `/users/${ME}`, `document.querySelector('h1 .painted')`],
  ['menu', `/users/${ME}`, `(document.querySelector('header .who')?.click(), null)`],
  ['picker', `/users/${ME}?s=cosmetics`, `[...document.querySelectorAll('section[aria-labelledby="paint-h"] .painted')].find((e) => e.textContent.includes(${JSON.stringify(paintName)}))`],
  ['admin-paints', '/admin/cosmetics/paints', `[...document.querySelectorAll('.tile')].find((t) => t.querySelector('.nm')?.textContent.includes(${JSON.stringify(paintName)}))?.querySelector('.painted')`],
];
for (const t of ['dark', 'light']) {
  for (const [name, path, sel] of surfaces) {
    await go(path, t);
    let target = sel;
    if (name === 'menu') {
      await ev(sel);
      await wait(500);
      target = `[...document.querySelectorAll('.painted')].find((e) => e.closest('[role=menu], .menu, [popover], dialog'))`;
    }
    if (name !== 'menu') await until(`!!(${target})`, 8000);
    const m = await measure(target, `${name}-${t}`);
    const fromUpload = typeof m.bg === 'string' && m.bg.includes(paintRow.image.host) && paintRow.image.host.includes('/cosmetics/uploads/');
    check(`${t} ${name}: draws the studio render`, m.ok !== false && fromUpload && m.cover >= 0.08 && m.hues + m.lums >= 3, m.ok === false ? m.why : `cover=${m.cover} hues=${m.hues} lums=${m.lums}`);
  }
  await go(`/users/${ME}?s=cosmetics`, t);
  await shot(`picker-${t}`);
  check(`${t} picker: no grid/list switch`, !(await ev(`!!document.querySelector('[aria-label="Grid"], [aria-label="List"]')`)));
}
for (const l of [...new Set(logs)]) console.log(l);
check('no exceptions or CSP blocks on the way', !logs.length, `${logs.length} logged`);
const failed = results.filter((r) => !r.ok);
console.log(`\n${results.length - failed.length}/${results.length} passed`);
await b.close();
process.exit(failed.length ? 1 : 0);
