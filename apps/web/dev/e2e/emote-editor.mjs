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
    if (m.id && pend.has(m.id)) {
      const { res, rej } = pend.get(m.id);
      pend.delete(m.id);
      m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result);
    }
  });
  return { send: (method, params = {}) => new Promise((res, rej) => { const i = ++id; pend.set(i, { res, rej }); ws.send(JSON.stringify({ id: i, method, params })); }), close: () => ws.close() };
}

const b = await launch(Number(process.env.CDP ?? 5452));
await b.send('Runtime.enable');
await b.send('Page.enable');
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
  check(`${tag} the editor loads when switched to`, await toEditor());
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

  if (full) {
    await press('z', 2); // Ctrl
    check(`${tag} Ctrl+Z undoes the arrow`, await until(`document.querySelector('.hist button')?.getAttribute('aria-label') === 'Undo Draw a rectangle'`));
    await press('z', 2 | 8); // Ctrl+Shift
    check(`${tag} Ctrl+Shift+Z redoes it`, await until(`document.querySelector('.hist button')?.getAttribute('aria-label') === 'Undo Draw an arrow'`));
    // The size rule: stretch past 1000 px, then the fix.
    await ev(`(() => { const i = [...document.querySelectorAll('.wh input')][0]; i.focus(); i.value = '2400'; i.dispatchEvent(new Event('input', { bubbles: true })); i.blur(); return 0; })()`);
    check(`${tag} 2400 px fails the size rule and disables Continue`, await until(`document.querySelector('.rule[data-rule=side]')?.classList.contains('fail') && document.querySelector('[data-testid=editor-continue]').disabled`));
    await click(`document.querySelector('.rule[data-rule=side] .fix')`);
    check(`${tag} "Fit to 1.000 px" passes again`, await until(`document.querySelector('.rule[data-rule=side]')?.classList.contains('pass')`), (await state()).rules.find((r) => r[0] === 'side')?.[2]);
    await click(`[...document.querySelectorAll('.tl .speed')][0]`);
    await ev(`(() => { const s = document.querySelector('.tl .speed'); s.value = '4'; s.dispatchEvent(new Event('change', { bubbles: true })); return 0; })()`);
    check(`${tag} 4× speed (10 ms frames) fails the frame rate rule`, await until(`document.querySelector('.rule[data-rule=fps]')?.classList.contains('fail')`));
    await shot(`${tag}-4-rules-failing`);
    await click(`document.querySelector('.rule[data-rule=fps] .fix')`);
    check(`${tag} "Limit to 50 fps" passes again`, await until(`document.querySelector('.rule[data-rule=fps]')?.classList.contains('pass')`));
    await until(`!document.querySelector('[data-testid=editor-continue]').disabled`, 30000);
    await shot(`${tag}-5-rules-passing`);
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
b.close();
const failed = results.filter((r) => !r.ok).length;
console.log(`${results.length - failed}/${results.length} passed`);
process.exit(failed ? 1 : 0);
