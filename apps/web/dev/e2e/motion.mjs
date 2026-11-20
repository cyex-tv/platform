// Motion smoke check in a real browser (Edge over CDP) against the dev mock: the site's
// Reduced motion setting (Settings > Appearance: System / On / Off, stored as cyexMotion and
// shown as <html data-motion>) crossed with the OS setting (prefers-reduced-motion).
//
//   setting     OS          motion expected
//   reduce      reduce      none
//   reduce      no-pref     none
//   full        reduce      yes   ("Off" keeps motion even when the OS asks for less)
//   full        no-pref     yes
//   system      reduce      none
//   system      no-pref     yes
//
// Per combination it opens the review detail (never animates, in any combination), shows an
// Undo toast, opens the bell menu on desktop and as a bottom sheet on a phone, opens the header
// search list (never animates), and counts RUNNING animations (document.getAnimations(), longer
// than 1 ms) on each. It also checks which file an animated emote's <img> picked (still or
// moving), and that arrow/Home/End keys in the review queue move the page instantly.
// Exits 0 only when every check passes.
//
//   MOCK_ADMIN=1 PORT=9331 SITE_ORIGIN=http://localhost:5731 MOCK_HOST='::' node dev/mock-api.mjs
//   PUBLIC_API_URL=http://localhost:9331 npx vite dev --port 5731
//   msedge --headless=new --remote-debugging-port=9831 --user-data-dir=<tmp> about:blank
//   SITE=http://localhost:5731 API=http://localhost:9331 CDP=9831 node dev/e2e/motion.mjs
const SITE = process.env.SITE ?? 'http://localhost:5731';
const API = process.env.API ?? 'http://localhost:9331';

async function launch(port) {
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
  const ws = new WebSocket(tabs.find((t) => t.type === 'page').webSocketDebuggerUrl);
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
  return { send: (method, params = {}) => new Promise((res, rej) => { const i = ++id; pend.set(i, { res, rej }); ws.send(JSON.stringify({ id: i, method, params })); }), close: () => ws.close() };
}

const b = await launch(Number(process.env.CDP ?? 9831));
await b.send('Runtime.enable'); await b.send('Page.enable'); await b.send('Network.enable');
await b.send('Network.setCookie', { name: 'mock_session', value: '1', url: API });
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
    await wait(150);
  }
  return false;
}
async function click(sel) {
  const r = await ev(`(() => { const el = ${sel}; if (!el) return null; const b = el.getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })()`);
  if (!r) throw new Error('no element: ' + sel);
  await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: r[0], y: r[1] });
  for (const type of ['mousePressed', 'mouseReleased']) await b.send('Input.dispatchMouseEvent', { type, x: r[0], y: r[1], button: 'left', clickCount: 1 });
}
async function key(k, code = k, text) {
  const base = { key: k, code, windowsVirtualKeyCode: { Enter: 13, Escape: 27, End: 35, Home: 36, ArrowDown: 40 }[k] ?? k.toUpperCase().charCodeAt(0) };
  await b.send('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', ...base, text });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base });
}
const results = [];
const check = (name, ok, detail = '') => { results.push({ name, ok: !!ok, detail }); console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` (${detail})` : ''}`); };

// Running animations longer than 1 ms, optionally only those on or inside `scope` (a selector).
const running = (scope) => `(() => document.getAnimations().filter((a) => {
  if (a.playState !== 'running' || (a.effect?.getComputedTiming().duration ?? 0) <= 1) return false;
  ${scope ? `const t = a.effect?.target; return !!t && (t.matches?.(${JSON.stringify(scope)}) || !!t.closest?.(${JSON.stringify(scope)}));` : 'return true;'}
}).map((a) => (a.animationName || a.transitionProperty || 'anim') + '@' + (a.effect?.target?.className?.baseVal ?? a.effect?.target?.className ?? a.effect?.target?.tagName)))()`;

async function setup(pref, os, phone) {
  await b.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: os }] });
  await b.send('Emulation.setDeviceMetricsOverride', phone ? { width: 375, height: 812, deviceScaleFactor: 2, mobile: true } : { width: 1440, height: 1000, deviceScaleFactor: 1, mobile: false });
  await b.send('Page.navigate', { url: SITE + '/robots.txt' });
  await wait(400);
  await ev(`try { ${pref === 'system' ? `localStorage.removeItem('cyexMotion')` : `localStorage.setItem('cyexMotion', '${pref}')`}; localStorage.setItem('cyexTheme', 'dark'); } catch {}`);
}
async function go(path) {
  await b.send('Page.navigate', { url: SITE + path });
  await until(`document.readyState === 'complete'`);
  await wait(1500);
}

const COMBOS = [
  ['system', 'no-preference', true], // first: it finds the animated emotes the others are judged by
  ['reduce', 'reduce', false], ['reduce', 'no-preference', false],
  ['full', 'reduce', true], ['full', 'no-preference', true],
  ['system', 'reduce', false],
];
const stillIdx = {};

for (const [pref, os, on] of COMBOS) {
  const tag = `[${pref} + OS ${os}]`;
  const say = (name, got, expectOn, always0 = false) => {
    const n = got.length;
    check(`${tag} ${name}: ${always0 ? 'no animation' : on ? 'animates' : 'still'}`, always0 || !on ? n === 0 : n > 0, `${n} running${n ? ': ' + got.slice(0, 4).join(', ') : ''}`);
  };

  // ---- Desktop: review queue
  await setup(pref, os, false);
  await go('/admin/review');
  check(`${tag} <html data-motion> follows the setting`, (await ev(`document.documentElement.dataset.motion ?? ''`)) === (pref === 'system' ? '' : pref));
  check(`${tag} matchMedia(reduce) answers by the setting`, (await ev(`matchMedia('(prefers-reduced-motion: reduce)').matches`)) === !on);
  if (!(await until(`!!document.querySelector('[data-card]')`))) { check(`${tag} review queue loads`, false); continue; }

  // Arrow keys: the page moves at once, never a smooth scroll.
  await ev(`window.scrollTo(0, 0); document.querySelector('[data-card]').focus(); 0`);
  await key('End');
  await wait(60);
  const early = await ev(`scrollY`);
  await wait(900);
  const late = await ev(`scrollY`);
  check(`${tag} End key in the review queue scrolls instantly`, late > 0 && early === late, `after 60 ms ${early}, after 1 s ${late}`);

  // Detail dialog: no animation at all, in any combination.
  await ev(`document.querySelector('[data-card]').focus(); 0`);
  await key('Enter', 'Enter', '\r');
  const opened = await until(`!!document.querySelector('dialog.detail[open]')`, 4000);
  check(`${tag} review detail opens`, opened);
  if (opened) {
    await wait(30);
    say('review detail dialog', await ev(running('dialog.detail')), on, true);
    const t = await ev(`getComputedStyle(document.querySelector('dialog.detail')).transitionDuration + '|' + getComputedStyle(document.querySelector('dialog.detail')).opacity + '|' + getComputedStyle(document.querySelector('dialog.detail')).scale`);
    check(`${tag} review detail has no transition, full opacity, no scale`, /^(0s|1e-05s)\|1\|(none|1)$/.test(t), t);
    await key('Escape');
    await until(`!document.querySelector('dialog.detail[open]')`, 3000);
    await ev(`document.querySelector('[data-card]').focus(); 0`);
  }

  // Undo toast (approve with A).
  await ev(`document.querySelector('[data-card]').focus(); 0`);
  await key('a', 'KeyA', 'a');
  await until(`!!document.querySelector('.toast')`, 3000);
  await wait(60);
  say('Undo toast', await ev(running('.toast')), on);
  const line = await ev(`!!document.querySelector('.toast .line')`);
  check(`${tag} Undo toast has the countdown line`, line);
  await key('z', 'KeyZ', 'z');
  await wait(200);

  // Header search list: opens with no animation.
  await click(`document.querySelector('input[type=search]')`);
  await b.send('Input.insertText', { text: 'a' });
  await until(`!document.querySelector('.drop')?.hidden`, 4000);
  await wait(40);
  say('header search list', await ev(running('.drop')), on, true);
  await ev(`document.activeElement?.blur(); 0`);
  await key('Escape');

  // A menu: the bell's popover on desktop.
  await go('/admin/review');
  await click(`document.querySelector('button.bell')`);
  await until(`document.querySelector('button.bell')?.getAttribute('aria-expanded') === 'true'`, 3000);
  await wait(50);
  say('bell menu', await ev(running('.pop')), on);

  // Animated emotes: which file the <img> chose.
  await go('/search');
  await until(`document.querySelectorAll('img[data-still]').length > 8`, 8000);
  await wait(800);
  const pics = await ev(`[...document.querySelectorAll('img[data-still]')].slice(0, 24).map((i) => ({ src: i.currentSrc || '' }))`);
  if (pref === 'system' && os === 'no-preference') {
    pics.forEach((p, i) => { stillIdx[i] = true; });
  }
  const ids = Object.keys(stillIdx).map(Number);
  if (pref === 'system' && os === 'no-preference') check(`${tag} the page has animated emotes with a still`, ids.length > 0, `${ids.length} of ${pics.length}`);
  else if (ids.length) {
    const wrong = ids.filter((i) => pics[i] && pics[i].src.includes('_static') === on);
    check(`${tag} animated emotes load the ${on ? 'moving' : 'still'} file`, wrong.length === 0, `${ids.length - wrong.length}/${ids.length} right`);
  }
  // No <source media> anywhere: Chromium leaks template-cloned ones (see dev/e2e/leak.mjs).
  check(`${tag} no <picture><source media> on the page`, !(await ev(`!!document.querySelector('picture source[media]')`)));

  // ---- Phone: the bell as a bottom sheet
  await setup(pref, os, true);
  await go('/');
  await click(`document.querySelector('button.bell.phone')`);
  const sheet = await until(`!!document.querySelector('dialog[open]')`, 3000);
  await wait(40);
  if (sheet) say('bottom sheet (phone)', await ev(running('dialog')), on);
  else check(`${tag} bottom sheet opens (phone)`, false);
}

console.log(`\n${results.filter((r) => r.ok).length} passed, ${results.filter((r) => !r.ok).length} failed`);
b.close();
process.exit(results.every((r) => r.ok) ? 0 : 1);
