// Leak check in a real browser (Edge or Chrome over CDP) against the dev mock: signs in, then per
// route navigates there and back to /guidelines (client-side, as a reader clicks) N times, forces
// garbage collection and reads the DOM node count (Performance.getMetrics "Nodes", detached nodes
// included). A route that leaves its page behind grows by hundreds of nodes per visit; one that
// is freed stays near 0. Exits 0 only when every route stays under LEAK_MAX nodes per visit.
//
// It guards a Chromium bug: an <img> with a srcset, or a <picture><source media>, that Svelte has
// moved through a DocumentFragment stays registered as a viewport listener after it is removed,
// and keeps the whole removed page alive with it. Emote pictures and badges therefore pick their
// file in script once mounted (pictureFor in src/lib/emote.ts). Run it on a production build: the
// dev server's output does not always take the same path and can pass where production leaks.
//
//   SITE_ORIGIN=http://localhost:5762 PORT=9361 MOCK_HOST='::' node dev/mock-api.mjs
//   PUBLIC_API_URL=http://localhost:9361 npm run build   (the API origin goes into the CSP)
//   PORT=5762 ORIGIN=http://localhost:5762 PUBLIC_API_URL=http://localhost:9361 node build
//   msedge --headless=new --remote-debugging-port=9861 --user-data-dir=<tmp> about:blank
//   SITE=http://localhost:5762 API=http://localhost:9361 CDP=9861 npm run test:leak
//
// Optional: VISITS (default 8), LEAK_MAX (default 20), ROUTES (comma-separated, replaces the list).
const SITE = process.env.SITE ?? 'http://localhost:5762';
const API = process.env.API ?? 'http://localhost:9361';
const CDP = Number(process.env.CDP ?? 9861);
const VISITS = Number(process.env.VISITS ?? 8);
const MAX = Number(process.env.LEAK_MAX ?? 20);
const ME = '01JD8X0000000000000000MEME';
const ROUTES = process.env.ROUTES
  ? process.env.ROUTES.split(',')
  : [
      `/users/${ME}?s=overview`,
      `/users/${ME}?s=activity`,
      `/users/${ME}?s=uploads`,
      `/users/${ME}?s=sets`,
      `/users/${ME}?s=cosmetics`,
      '/me/notifications',
      '/me/channels',
      '/me/settings',
      '/search',
      '/search?q=pog',
      '/emotes/1',
      '/cosmetics',
    ];

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

// One browser-level socket, one fresh tab attached with a flat session.
const { webSocketDebuggerUrl } = await (await fetch(`http://127.0.0.1:${CDP}/json/version`)).json();
const ws = new WebSocket(webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener('open', r));
let id = 0;
const pend = new Map();
ws.addEventListener('message', (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pend.has(m.id)) {
    const { res, rej } = pend.get(m.id);
    pend.delete(m.id);
    m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result);
  }
});
const raw = (method, params = {}, sessionId) =>
  new Promise((res, rej) => {
    const i = ++id;
    pend.set(i, { res, rej });
    ws.send(JSON.stringify({ id: i, method, params, ...(sessionId ? { sessionId } : {}) }));
  });

const { targetId } = await raw('Target.createTarget', { url: 'about:blank', newWindow: true });
const { sessionId } = await raw('Target.attachToTarget', { targetId, flatten: true });
const S = (m, p) => raw(m, p, sessionId);
let code = 1;
try {
  await S('Page.enable');
  await S('Runtime.enable');
  await S('Performance.enable');
  await S('HeapProfiler.enable');
  await S('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  const ev = async (expr) => {
    const r = await S('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(`${expr.slice(0, 80)}: ${r.exceptionDetails.exception?.description}`);
    return r.result.value;
  };
  // Signs in through the mock (it sets the session and sends the browser back to the site).
  await S('Page.navigate', { url: `${API}/v1/auth/twitch/login?return_to=/help` });
  const end = Date.now() + 20000;
  while (Date.now() < end && !(await ev(`location.origin === ${JSON.stringify(new URL(SITE).origin)} && document.readyState === 'complete'`).catch(() => false))) await wait(200);
  await wait(2500);

  // A client-side navigation, as a click on a link does.
  const go = async (href, ms) => {
    // A dev server may reload the page once while it optimises dependencies: wait for a body.
    for (let t = 0; t < 100 && !(await ev(`!!document.body && document.readyState === 'complete'`).catch(() => false)); t++) await wait(200);
    await ev(`(() => { const a = document.createElement('a'); a.href = ${JSON.stringify(href)}; document.body.appendChild(a); a.click(); a.remove(); })()`);
    await wait(ms);
  };
  const nodes = async () => {
    await S('HeapProfiler.collectGarbage');
    await S('HeapProfiler.collectGarbage');
    await wait(300);
    return (await S('Performance.getMetrics')).metrics.find((m) => m.name === 'Nodes').value;
  };

  // One pass over every route first, so chunks, caches and stores made once are not counted.
  for (const r of ROUTES) await go(r, 1500);
  const rows = [];
  for (const r of ROUTES) {
    await go('/guidelines', 700);
    await go(r, 1800); // warm: caches, lazy chunks and the route's stores are made once
    await go('/guidelines', 700);
    const before = await nodes();
    let imgs = 0;
    for (let i = 0; i < VISITS; i++) {
      await go(r, 1500);
      imgs = await ev('document.images.length'); // shows the page really rendered
      await go('/guidelines', 700);
    }
    const after = await nodes();
    const per = Math.round((after - before) / VISITS);
    const ok = per < MAX;
    rows.push(ok);
    console.log(`${ok ? 'PASS' : 'FAIL'} ${r.padEnd(46)} nodes ${String(before).padStart(6)} -> ${String(after).padStart(6)}  ${String(per).padStart(5)} per visit  (${imgs} images on the page)`);
  }
  const bad = rows.filter((x) => !x).length;
  console.log(`\n${rows.length - bad}/${rows.length} routes under ${MAX} nodes left behind per visit (${VISITS} visits each)`);
  code = bad ? 1 : 0;
} catch (e) {
  console.error(e);
} finally {
  await raw('Target.closeTarget', { targetId }).catch(() => {});
  ws.close();
}
process.exit(code);
