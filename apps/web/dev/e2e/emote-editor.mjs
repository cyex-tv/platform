// The emote editor in a real browser (Edge over CDP) against the dev mock: the upload dialog's
// two modes, opening a GIF and an MP4 in the editor (frames, timing, a seamless loop), drawing,
// the upload rules gating "Continue to upload", the one-click fixes, and the exported file going
// through the normal upload. With SHOTS=<dir> it also saves the screenshots of each state at
// 1920 dark, 1440 light and 375 dark. Exits 0 only when every check passes.
//
//   MOCK_OPEN=1 PORT=5450 SITE_ORIGIN=http://localhost:5451 node dev/mock-api.mjs
//   PUBLIC_API_URL=http://localhost:5450 npx vite dev --port 5451
//   msedge --headless=new --remote-debugging-port=5452 --user-data-dir=<tmp> about:blank
//   SITE=http://localhost:5451 CDP=5452 [SHOTS=<dir>] node dev/e2e/emote-editor.mjs
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import gifenc from 'gifenc';
const { GIFEncoder, quantize, applyPalette } = gifenc;

const SITE = process.env.SITE ?? 'http://localhost:5451';
const SHOTS = process.env.SHOTS ?? '';
const here = dirname(fileURLToPath(import.meta.url));

async function launch(port) {
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
  const ws = new WebSocket(tabs.find((t) => t.type === 'page').webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener('open', r));
  let id = 0;
  const pend = new Map();
  ws.addEventListener('message', (e) => {
    const m = JSON.parse(e.data);
    // The editor's leave guard asks before a reload drops edits: say yes (each run starts fresh).
    if (m.method === 'Page.javascriptDialogOpening') ws.send(JSON.stringify({ id: ++id, method: 'Page.handleJavaScriptDialog', params: { accept: true } }));
    if (m.id && pend.has(m.id)) {
      const { res, rej } = pend.get(m.id);
      pend.delete(m.id);
      m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result);
    }
  });
  return { send: (method, params = {}) => new Promise((res, rej) => { const i = ++id; pend.set(i, { res, rej }); ws.send(JSON.stringify({ id: i, method, params })); }), close: () => ws.close() };
}

const b = await launch(Number(process.env.CDP ?? 5452));
await b.send('Page.enable');
// A leave prompt left open by an earlier run blocks the page until answered.
await b.send('Page.handleJavaScriptDialog', { accept: true }).catch(() => {});
await b.send('Runtime.enable');
await b.send('Network.enable');
await b.send('Network.setCookie', { name: 'mock_session', value: '1', url: process.env.API ?? 'http://localhost:5450' });
const ev = async (expr) => {
  const r = await b.send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(`${expr.slice(0, 120)}: ${r.exceptionDetails.exception?.description}`);
  return r.result.value;
};
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
async function until(expr, ms = 20000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (await ev(expr)) return true;
    await wait(150);
  }
  return false;
}
const results = [];
const check = (name, ok, detail = '') => {
  results.push({ name, ok: !!ok });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? ` (${detail})` : ''}`);
};
async function point(sel) {
  const r = await ev(`(() => { const el = ${sel}; if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const b = el.getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })()`);
  if (!r) throw new Error('no element: ' + sel);
  return r;
}
async function click(sel) {
  const [x, y] = await point(sel);
  await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
  for (const type of ['mousePressed', 'mouseReleased']) await b.send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
}
async function drag(from, to, steps = 8) {
  await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: from[0], y: from[1] });
  await b.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: from[0], y: from[1], button: 'left', buttons: 1, clickCount: 1 });
  for (let i = 1; i <= steps; i++) {
    const x = from[0] + ((to[0] - from[0]) * i) / steps;
    const y = from[1] + ((to[1] - from[1]) * i) / steps;
    await b.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'left', buttons: 1 });
  }
  await b.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: to[0], y: to[1], button: 'left', buttons: 0, clickCount: 1 });
}
async function press(k, mods = 0) {
  const code = k.length === 1 ? `Key${k.toUpperCase()}` : k;
  const vk = k.length === 1 ? k.toUpperCase().charCodeAt(0) : { Escape: 27, Delete: 46 }[k];
  await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: k, code, windowsVirtualKeyCode: vk, modifiers: mods });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code, windowsVirtualKeyCode: vk, modifiers: mods });
}

/** Types `text` into a field (replacing it) and presses Enter, as a person would. */
async function typeInto(sel, text) {
  await ev(`(() => { const i = ${sel}; i.focus(); i.select?.(); return 0; })()`);
  await b.send('Input.insertText', { text });
  await b.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
  await b.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
  await wait(100);
}

/** Drops a file (bytes from here) on the open upload dialog. */
async function dropFile(bytes, name, type) {
  const b64 = Buffer.from(bytes).toString('base64');
  await ev(`(() => {
    const bin = atob('${b64}'); const u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
    const dt = new DataTransfer(); dt.items.add(new File([u], ${JSON.stringify(name)}, { type: ${JSON.stringify(type)} }));
    document.querySelector('dialog.upload-dialog').dispatchEvent(new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true }));
    return true;
  })()`);
}

/** A 24-frame animated GIF, 320 × 240, a dot going round, 40 ms a frame. */
function gif(w = 320, h = 240, frames = 24, delay = 40) {
  const g = GIFEncoder();
  for (let f = 0; f < frames; f++) {
    const rgba = new Uint8Array(w * h * 4);
    const cx = w / 2 + Math.cos((f / frames) * Math.PI * 2) * w * 0.3;
    const cy = h / 2 + Math.sin((f / frames) * Math.PI * 2) * h * 0.3;
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const i = (y * w + x) * 4;
        const d = Math.hypot(x - cx, y - cy);
        const on = d < h * 0.15;
        rgba.set(on ? [255, 120, 40, 255] : [(x * 255) / w, (y * 255) / h, 160, (x + y) % 64 < 48 ? 255 : 0], i);
      }
    const palette = quantize(rgba, 256, { format: 'rgba4444', oneBitAlpha: true });
    const index = applyPalette(rgba, palette, 'rgba4444');
    const t = palette.findIndex((p) => p[3] === 0);
    g.writeFrame(index, w, h, { palette, delay, transparent: t >= 0, transparentIndex: Math.max(0, t) });
  }
  g.finish();
  return g.bytes();
}

const state = () =>
  ev(`(() => {
    const rules = [...document.querySelectorAll('.rule')].map((r) => [r.dataset.rule, r.classList.contains('pass') ? 'pass' : r.classList.contains('fail') ? 'fail' : 'pending', r.querySelector('.val')?.textContent.trim()]);
    const cont = document.querySelector('[data-testid="editor-continue"]');
    return { rules, cont: cont ? !cont.disabled : null, pos: document.querySelector('.tl .pos')?.textContent.trim(), facts: document.querySelector('.facts')?.textContent.trim() };
  })()`);

async function setup(width, height, theme, mobile = false) {
  await b.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile });
  await b.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: theme }, { name: 'prefers-reduced-motion', value: 'reduce' }] });
  await b.send('Page.navigate', { url: SITE + '/robots.txt' });
  await wait(300);
  await ev(`try { localStorage.setItem('cyexTheme', '${theme}'); localStorage.setItem('cyexMotion', 'reduce'); } catch {} 0`);
  await b.send('Page.navigate', { url: SITE + '/?upload' });
  await until(`!!document.querySelector('dialog.upload-dialog[open]')`);
  await wait(800);
}

async function shot(name) {
  if (!SHOTS) return;
  mkdirSync(SHOTS, { recursive: true });
  const { data } = await b.send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(SHOTS, `${name}.png`), Buffer.from(data, 'base64'));
}

const GIF = gif();
const MP4 = readFileSync(join(here, 'fixtures', 'loop.mp4'));

async function toEditor() {
  await click(`[...document.querySelectorAll('.modes [role=radio]')].find((x) => x.textContent.includes('Editor'))`);
  return until(`!!document.querySelector('.ed')`);
}

async function run(width, height, theme, tag, full) {
  const mobile = width < 640;
  await setup(width, height, theme, mobile);
  check(`${tag} the dialog opens in Quick upload`, await ev(`document.querySelector('.modes [aria-checked=true]')?.textContent.includes('Quick')`));
  await shot(`${tag}-1-quick-upload`);
  const rect = () => ev(`(() => { const b = document.querySelector('dialog.upload-dialog').getBoundingClientRect(); return [b.x, b.y, b.width, b.height].map(Math.round).join(','); })()`);
  const quickRect = await rect();
  check(`${tag} the editor loads when switched to`, await toEditor());
  await wait(300);
  check(`${tag} both modes are the same size and place`, (await rect()) === quickRect, `${quickRect} / ${await rect()}`);
  await shot(`${tag}-1b-editor-empty`);
  await dropFile(GIF, 'orbit.gif', 'image/gif');
  check(`${tag} a GIF opens: 24 frames`, await until(`document.querySelector('.tl .pos')?.textContent.trim() === '1 / 24'`, 60000), (await state()).pos);
  await until(`document.querySelector('.rule[data-rule=bytes]')?.classList.contains('pass')`);
  let s = await state();
  check(`${tag} every rule passes and Continue is enabled`, s.cont === true && s.rules.every((r) => r[1] === 'pass'), JSON.stringify(s.rules));
  await shot(`${tag}-2-editor-gif`);

  // Drawing: the rectangle tool, then a drag on the stage.
  await press('r');
  const box = await ev(`(() => { const r = document.querySelector('.stage').getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; })()`);
  await drag([box[0] + box[2] * 0.35, box[1] + box[3] * 0.35], [box[0] + box[2] * 0.6, box[1] + box[3] * 0.62]);
  await press('a');
  await drag([box[0] + box[2] * 0.3, box[1] + box[3] * 0.7], [box[0] + box[2] * 0.55, box[1] + box[3] * 0.45]);
  check(`${tag} drawing adds undo steps`, await until(`document.querySelector('.hist button')?.getAttribute('aria-label')?.startsWith('Undo Draw')`));
  await shot(`${tag}-3-drawing`);

  {
    await press('z', 2); // Ctrl
    check(`${tag} Ctrl+Z undoes the arrow`, await until(`document.querySelector('.hist button')?.getAttribute('aria-label') === 'Undo Draw a rectangle'`));
    await press('z', 2 | 8); // Ctrl+Shift
    check(`${tag} Ctrl+Shift+Z redoes it`, await until(`document.querySelector('.hist button')?.getAttribute('aria-label') === 'Undo Draw an arrow'`));
    // The size rule: stretch past 1000 px, then the fix.
    await ev(`(() => { const i = [...document.querySelectorAll('.wh input')][0]; i.focus(); i.value = '2400'; i.dispatchEvent(new Event('input', { bubbles: true })); i.blur(); return 0; })()`);
    check(`${tag} 2400 px fails the size rule and disables Continue`, await until(`document.querySelector('.rule[data-rule=side]')?.classList.contains('fail') && document.querySelector('[data-testid=editor-continue]').disabled`));
    await click(`document.querySelector('.rule[data-rule=side] .fix')`);
    check(`${tag} "Fit to 1.000 px" passes again`, await until(`document.querySelector('.rule[data-rule=side]')?.classList.contains('pass')`), (await state()).rules.find((r) => r[0] === 'side')?.[2]);
    // Speed: 400% would be 10 ms frames; frames merge so the upload stays within 50 fps.
    await typeInto(`document.querySelector('.pctbox input')`, '400');
    check(`${tag} 400% speed merges frames and keeps the frame rate rule`, await until(`document.querySelector('.rule[data-rule=fps]')?.classList.contains('pass') && /of 24 frames upload/.test(document.querySelector('.spdnote')?.textContent ?? '')`), await ev(`document.querySelector('.spdnote')?.textContent.trim()`));
    check(`${tag} the loop is a quarter as long`, (await ev(`document.querySelector('.tl .len')?.textContent.trim()`)) === '0,2 s', await ev(`document.querySelector('.tl .len')?.textContent`));
    await shot(`${tag}-7-speed-400`);
    await typeInto(`document.querySelector('.pctbox input')`, '0');
    check(`${tag} 0% freezes it: a still of the frame on show`, await until(`/still image of frame/.test(document.querySelector('.spdnote')?.textContent ?? '') && document.querySelector('.rule[data-rule=frames] .val')?.textContent.trim() === '1'`));
    check(`${tag} play is off while frozen`, await ev(`[...document.querySelectorAll('.tl .tb')].find((x) => /^(Play|Pause)$/.test(x.getAttribute('aria-label')))?.disabled === true`));
    await shot(`${tag}-8-speed-0`);
    await click(`document.querySelector('[aria-label="Speed back to 100%"]')`);
    check(`${tag} back to 100%`, await until(`document.querySelector('.pctbox input')?.value === '100'`));
    // Frames set to 10 ms at 100%: the frame rate rule fails until its fix.
    await typeInto(`document.querySelector('.tl .delay input')`, '10');
    await click(`[...document.querySelectorAll('.tl .ui-btn')].find((x) => x.textContent.trim() === 'All frames')`);
    check(`${tag} 10 ms frames fail the frame rate rule`, await until(`document.querySelector('.rule[data-rule=fps]')?.classList.contains('fail')`));
    await ev(`document.querySelector('.rules')?.scrollIntoView({ block: 'start' }), 0`);
    await shot(`${tag}-4-rules-failing`);
    await click(`document.querySelector('.rule[data-rule=fps] .fix')`);
    check(`${tag} "Limit to 50 fps" passes again`, await until(`document.querySelector('.rule[data-rule=fps]')?.classList.contains('pass')`));
    await until(`!document.querySelector('[data-testid=editor-continue]').disabled`, 30000);
    await ev(`document.querySelector('.rules')?.scrollIntoView({ block: 'start' }), 0`);
    await shot(`${tag}-5-rules-passing`);

    // Stretching: the resize tool's east handle pulls it wider, to any shape; past 4:1 it can't upload.
    // Back to the GIF's own 320 × 240 first (proportions are free, so each side is typed).
    await typeInto(`[...document.querySelectorAll('.wh input')][0]`, '320');
    await typeInto(`[...document.querySelectorAll('.wh input')][1]`, '240');
    // Keys are the editor's once the field lets go of focus.
    await ev(`document.activeElement?.blur(), 0`);
    await press('s');
    check(`${tag} the resize tool shows the output with its size`, await until(`/px ·/.test(document.querySelector('.stage .readout')?.textContent ?? '')`));
    await ev(`document.querySelector('.stage')?.scrollIntoView({ block: 'nearest' }), 0`);
    const before = await ev(`[...document.querySelectorAll('.wh input')].map((i) => +i.value)`);
    const handle = async () => {
      // The east handle: the right edge of the output, half way down (the readout says the size).
      const [w, h] = await ev(`[...document.querySelectorAll('.wh input')].map((i) => +i.value)`);
      const z = (await ev(`parseFloat(document.querySelector('.stage .zl').textContent)`)) / 100;
      const r = await ev(`(() => { const b = document.querySelector('.stage').getBoundingClientRect(); return [b.x, b.y, b.width, b.height]; })()`);
      return { w, h, z, r };
    };
    {
      const { w, h, z, r } = await handle();
      // The fitted output sits in the middle of the stage.
      const cx = r[0] + r[2] / 2;
      const cy = r[1] + r[3] / 2;
      const east = [cx + (w * z) / 2, cy];
      await drag(east, [Math.min(r[0] + r[2] - 4, east[0] + Math.max(60, r[2] * 0.2)), cy], 10);
      const after = await ev(`[...document.querySelectorAll('.wh input')].map((i) => +i.value)`);
      check(`${tag} the side handle stretches the width only`, after[0] > before[0] && after[1] === h, `${before.join('×')} → ${after.join('×')}`);
    }
    // Typed: far past 4:1, and the rules say why Continue is off.
    await typeInto(`[...document.querySelectorAll('.wh input')][0]`, String(before[1] * 5));
    check(`${tag} past 4:1 fails the shape rule, said plainly`, await until(`document.querySelector('.rule[data-rule=aspect]')?.classList.contains('fail') && /too wide/.test(document.querySelector('.rule[data-rule=aspect]')?.textContent ?? '') && document.querySelector('[data-testid=editor-continue]').disabled`));
    check(`${tag} the stage says it can't upload`, await until(`/can’t upload/.test(document.querySelector('.stage .readout')?.textContent ?? '')`));
    await shot(`${tag}-6-stretch-too-wide`);
    await typeInto(`[...document.querySelectorAll('.wh input')][0]`, String(before[0]));
    await typeInto(`[...document.querySelectorAll('.wh input')][1]`, String(before[0] * 3 + 5));
    check(`${tag} taller than 1:3 fails too`, await until(`/too tall/.test(document.querySelector('.rule[data-rule=aspect]')?.textContent ?? '')`));
    await typeInto(`[...document.querySelectorAll('.wh input')][1]`, String(before[0] * 3));
    check(`${tag} exactly 1:3 passes`, await until(`document.querySelector('.rule[data-rule=aspect]')?.classList.contains('pass')`));
    await typeInto(`[...document.querySelectorAll('.wh input')][1]`, String(before[1]));
    await ev(`document.querySelector('.stage')?.scrollIntoView({ block: 'nearest' }), 0`);
    await wait(150);

    // The wheel over the stage zooms at the pointer, and nothing scrolls.
    const zl = () => ev(`parseFloat(document.querySelector('.stage .zl').textContent)`);
    const scrolls = () => ev(`[document.scrollingElement.scrollTop, ...[...document.querySelectorAll('.work, .scroll, .pane')].map((e) => e.scrollTop)].join(',')`);
    const z0 = await zl();
    const s0 = await scrolls();
    const sr = await ev(`(() => { const b = document.querySelector('.stage').getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })()`);
    for (let i = 0; i < 3; i++) await b.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: sr[0], y: sr[1], deltaX: 0, deltaY: -100 });
    await wait(200);
    const z1 = await zl();
    for (let i = 0; i < 6; i++) await b.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: sr[0], y: sr[1], deltaX: 0, deltaY: 100 });
    await wait(200);
    const z2 = await zl();
    check(`${tag} the wheel zooms in and out`, z1 > z0 && z2 < z1, `${z0}% → ${z1}% → ${z2}%`);
    check(`${tag} the page and dialog don't scroll while zooming`, (await scrolls()) === s0, `${s0} / ${await scrolls()}`);
    await b.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: sr[0], y: sr[1], deltaX: 0, deltaY: -40, modifiers: 2 });
    await wait(150);
    check(`${tag} a pinch (Ctrl + wheel) zooms too`, (await zl()) > z2, `${z2}% → ${await zl()}%`);
    await press('v');
  }
  if (full) {
    // Continue: the file goes to the quick form.
    await click(`document.querySelector('[data-testid=editor-continue]')`);
    check(`${tag} Continue hands a WebP to Quick upload`, await until(`/orbit\\.(webp|gif)/.test(document.querySelector('.meta')?.textContent ?? '') && document.querySelector('.modes [aria-checked=true]')?.textContent.includes('Quick')`), await ev(`document.querySelector('.meta')?.textContent`));
    check(`${tag} the quick form reads it as an image`, await until(`!!document.querySelector('.pv img, .pv .sizes')`), await ev(`document.querySelector('.drop .ui-error')?.textContent ?? ''`));
    // Upload it through the normal path.
    await click(`document.querySelector('[data-testid=accept-rules]')`);
    await click(`document.querySelector('.up button[type=submit]')`);
    check(`${tag} the mock accepts the upload`, await until(`!document.querySelector('dialog.upload-dialog[open]')`, 20000));

    // An MP4 from Quick upload: "Open in editor".
    await b.send('Page.navigate', { url: SITE + '/?upload' });
    await until(`!!document.querySelector('dialog.upload-dialog[open]')`);
    await wait(500);
    await dropFile(MP4, 'loop.mp4', 'video/mp4');
    check(`${tag} Quick upload offers Open in editor for an MP4`, await until(`!!document.querySelector('[data-testid=open-in-editor].primary')`));
    await click(`document.querySelector('[data-testid=open-in-editor]')`);
    check(`${tag} the MP4 opens as 30 frames`, await until(`document.querySelector('.tl .pos')?.textContent.trim() === '1 / 30'`, 30000), (await state()).pos);
    const delays = await ev(`[...document.querySelectorAll('.tl .item .d')].map((d) => +d.textContent)`);
    check(`${tag} 30 fps frames are 33 or 34 ms`, delays.length > 5 && delays.every((d) => d === 33 || d === 34), delays.join(','));
    check(`${tag} the loop lasts exactly 1 s`, (await ev(`document.querySelector('.tl .len')?.textContent.trim()`)) === '1 s', await ev(`document.querySelector('.tl .len')?.textContent`));
  }
}

const SET = [
  [1920, 1080, 'dark', '1920-dark', true],
  [1440, 900, 'light', '1440-light', false],
  [375, 812, 'dark', '375-dark', false],
];
for (const [w, h, theme, tag, full] of SET) {
  try {
    await run(w, h, theme, tag, full);
  } catch (e) {
    check(`${tag} ran to the end`, false, e.message);
  }
}
// Leave nothing to guard, so the next run's first navigation isn't held by a leave prompt.
await b.send('Page.navigate', { url: 'about:blank' }).catch(() => {});
b.close();
const failed = results.filter((r) => !r.ok).length;
console.log(`${results.length - failed}/${results.length} passed`);
process.exit(failed ? 1 : 0);
