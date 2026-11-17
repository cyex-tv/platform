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
import { createServer } from 'node:http';

const PORT = Number(process.env.PORT ?? 8787);
const ORIGIN = process.env.SITE_ORIGIN ?? 'http://localhost:5173';
const BASE = `http://localhost:${PORT}`;
const OPEN_ENV = process.env.MOCK_OPEN === '1';
const ADMIN_ENV = process.env.MOCK_ADMIN === '1' || OPEN_ENV;
const FACES = process.env.MOCK_FACES === '1';

const ME_ID = '01JD8X0000000000000000MEME';
const OWNER_DEFS = [
  ['lumen_fox', ME_ID], ['quietoak', '01JD8X0000000000000000OAK1'], ['nora_byte', '01JD8X0000000000000000NORA'],
  ['pixelmoth'], ['byteNod'], ['sunnyfern'], ['mossgate'], ['kiwi_rae'], ['tidalwren'], ['emberlynx'], ['sleepyhollis'], ['orbit_jules'],
];
const OWNERS = OWNER_DEFS.map(([displayName, id], n) => ({
  id: id ?? `01JD8X0000000000000000U${String(n).padStart(3, '0')}`, displayName, avatar: `${BASE}/avatars/${n}.svg`,
}));
const ME_SUMMARY = OWNERS[0];
const OTHERS = OWNERS.slice(1);

// ---- Emote pictures -------------------------------------------------------------------
// Original faces drawn from the emote's index: a head, eyes, a mouth and sometimes a hat, ears,
// headphones or a small extra. Everything is deterministic, so an emote always looks the same.
// Pictures are SVG, 32x32 (wide ones 64x32), transparent, with a darker outline of the same hue.

function rngFor(i, salt = 0) {
  let a = (Math.imul(i + 1, 2654435761) ^ Math.imul(salt + 7, 40503)) >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const TAGS = ['cat', 'happy', 'sad', 'hype', 'love', 'meme', 'wave', 'sleepy', 'dance', 'pog'];
const SUBJECTS = ['tabby', 'fox', 'owl', 'moth', 'bun', 'oak', 'fern', 'wren', 'lynx', 'moon', 'sun', 'star', 'bean', 'toad', 'pup',
  'duck', 'mochi', 'pea', 'kiwi', 'bee'];
const WORDS = {
  cat: ['Paws', 'Loaf', 'Purr', 'Mew'], happy: ['Smile', 'Beam', 'Grin', 'Yay'], sad: ['Cry', 'Sob', 'Droop', 'Rain'],
  hype: ['Hype', 'Rush', 'Boom', 'Zoom'], love: ['Love', 'Heart', 'Hug', 'Mush'], meme: ['Smug', 'Stare', 'Hmm', 'Sus'],
  wave: ['Hi', 'Wave', 'Hello', 'Bye'], sleepy: ['Zzz', 'Nap', 'Yawn', 'Doze'], dance: ['Jam', 'Hop', 'Groove', 'Bounce'],
  pog: ['Wow', 'Gasp', 'Pop', 'Whoa'],
};
const WIDE_SUBJECTS = ['tabby', 'worm', 'loaf'];

const SPECS = new Map();
function spec(i) {
  let s = SPECS.get(i);
  if (s) return s;
  const r = rngFor(i);
  const pick = (a) => a[Math.floor(r() * a.length)];
  const wide = i % 7 === 3;
  const zero = !wide && i % 23 === 5;
  s = { i, wide, zero, animated: i % 4 === 1, h: Math.floor(r() * 360), scene: wide ? Math.floor(i / 7) % 3 : 0 };
  if (zero) {
    s.kind = pick(['hat', 'sparkles', 'crown', 'halo', 'hearts', 'shades']);
    s.primary = { hat: 'hype', sparkles: 'happy', crown: 'meme', halo: 'love', hearts: 'love', shades: 'meme' }[s.kind];
    s.anim = { hat: 'bounce', crown: 'bounce', halo: 'bounce', shades: 'wobble' }[s.kind] ?? 'none';
  } else if (wide) {
    s.primary = ['cat', 'happy', 'sleepy'][s.scene];
  } else {
    s.head = pick(['circle', 'circle', 'squircle', 'egg', 'blob']);
    s.eyes = pick(['dots', 'dots', 'happy', 'wide', 'closed', 'hearts', 'sparkles', 'shades']);
    s.mouth = pick(['smile', 'smile', 'grin', 'open', 'flat', 'tongue', 'o', 'wavy']);
    s.gear = r() < 0.4 ? pick(['ears', 'hat', 'crown', 'phones']) : null;
    s.side = r() < 0.5 ? pick(['sweat', 'tear', 'hand', 'question', 'excl', 'zzz']) : null;
    if (s.side === 'zzz' && s.gear) s.side = null;
    s.blush = r() < 0.25;
    s.anim = s.side === 'hand' ? 'wave' : s.gear === 'phones' ? 'jam' : s.gear === 'ears' ? 'wobble'
      : pick(['bounce', 'wobble', 'blink', 'spin', 'jam', 'bounce']);
    s.primary = s.gear === 'ears' ? 'cat' : s.eyes === 'closed' || s.side === 'zzz' ? 'sleepy' : s.side === 'hand' ? 'wave'
      : s.eyes === 'hearts' ? 'love' : s.eyes === 'shades' || s.side === 'question' ? 'meme' : s.side === 'tear' ? 'sad'
      : s.side === 'excl' || s.eyes === 'sparkles' ? 'hype' : s.gear === 'phones' ? 'dance'
      : s.mouth === 'open' || s.mouth === 'o' ? 'pog' : s.blush ? 'love' : pick(['happy', 'happy', 'meme', 'hype']);
  }
  const second = pick(TAGS.filter((t) => t !== s.primary));
  s.tags = r() < 0.7 ? [s.primary, second] : [s.primary];
  s.subject = wide ? WIDE_SUBJECTS[s.scene] : pick(SUBJECTS);
  s.word = pick(WORDS[s.primary]);
  SPECS.set(i, s);
  return s;
}

const USED_NAMES = new Set();
const NAME_OF = new Map();
function nameFor(i) {
  let n = NAME_OF.get(i);
  if (n) return n;
  const s = spec(i);
  n = s.subject + s.word;
  if (USED_NAMES.has(n.toLowerCase()) || i >= 300) n += i;
  USED_NAMES.add(n.toLowerCase());
  NAME_OF.set(i, n);
  return n;
}

// SMIL helper: one looping animateTransform with eased keyframes.
function loop(type, vals, dur, o = {}) {
  const n = vals.length;
  const kt = vals.map((_, k) => +(k / (n - 1)).toFixed(3)).join(';');
  const spl = o.linear ? '' : ` calcMode="spline" keySplines="${Array(n - 1).fill('.45 0 .55 1').join(';')}"`;
  return `<animateTransform attributeName="transform" type="${type}" values="${vals.join(';')}" keyTimes="${kt}"${spl} dur="${dur}s" repeatCount="indefinite"${o.begin ? ` begin="${o.begin}s"` : ''}/>`;
}
const pen = (c, w = 1.3) => `fill="none" stroke="${c}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"`;
const heart = (x, y, k, fill, line) =>
  `<path transform="translate(${x} ${y}) scale(${k})" d="M0 3C-5-1-3-5.5 0-3 3-5.5 5-1 0 3z" fill="${fill}" stroke="${line}" stroke-width="${(1.1 / k).toFixed(2)}" stroke-linejoin="round"/>`;
const star = (x, y, k, fill, line, extra = '') =>
  `<path transform="translate(${x} ${y}) scale(${k})" d="M0-4 1.1-1.1 4 0 1.1 1.1 0 4-1.1 1.1-4 0-1.1-1.1z" fill="${fill}" stroke="${line}" stroke-width="${(1 / k).toFixed(2)}" stroke-linejoin="round">${extra}</path>`;
const zed = (x, y, k, c, extra = '') =>
  `<g transform="translate(${x} ${y}) scale(${k})"><path d="M0 0h4l-4 5h4" ${pen(c, 1.2 / k)}>${extra}</path></g>`;
const GOLD = 'hsl(46 95% 60%)';
const GOLD_LINE = 'hsl(33 75% 32%)';

function wrap(inner, s) {
  if (!s.animated) return inner;
  switch (s.anim) {
    case 'bounce': return `<g>${loop('translate', ['0 0', '0 -2.6', '0 0'], 0.8)}${inner}</g>`;
    case 'wobble': return `<g>${loop('rotate', ['-7 16 28', '7 16 28', '-7 16 28'], 0.7)}${inner}</g>`;
    case 'spin': return `<g>${loop('rotate', ['0 16 17', '360 16 17'], 1.2, { linear: true })}${inner}</g>`;
    case 'jam': return `<g>${loop('translate', ['0 0', '0 2.2', '0 0'], 0.6)}<g>${loop('rotate', ['-6 16 28', '6 16 28', '-6 16 28'], 1.2)}${inner}</g></g>`;
    default: return inner;
  }
}

function face(s) {
  const { h } = s;
  const fill = `hsl(${h} 78% 64%)`;
  const line = `hsl(${h} 62% 28%)`;
  const ink = `hsl(${h} 55% 13%)`;
  let out = '';
  if (s.gear === 'ears') {
    const inner = `hsl(${h} 85% 82%)`;
    out += `<g fill="${fill}" stroke="${line}" stroke-width="1.3" stroke-linejoin="round"><path d="M5.2 13.4L6.4 2.6 14.4 7.4z"/><path d="M26.8 13.4L25.6 2.6 17.6 7.4z"/></g>`
      + `<g fill="${inner}"><path d="M7.6 9.6L8 5.6l3.4 2.2z"/><path d="M24.4 9.6L24 5.6 20.6 7.8z"/></g>`;
  }
  const heads = {
    circle: '<circle cx="16" cy="17" r="12.6"/>',
    squircle: '<rect x="3.6" y="4.8" width="24.8" height="24.4" rx="9"/>',
    egg: '<ellipse cx="16" cy="17" rx="11.8" ry="13"/>',
    blob: '<path d="M16 4.6c7.2 0 12.6 4.4 12.6 12 0 7.2-5.1 11.8-12.6 11.8S3.4 23.8 3.4 16.6C3.4 9 8.8 4.6 16 4.6z"/>',
  };
  out += `<g fill="${fill}" stroke="${line}" stroke-width="1.4" stroke-linejoin="round">${heads[s.head]}</g>`;
  out += '<ellipse cx="11" cy="9.6" rx="4" ry="2" fill="#fff" opacity=".28" transform="rotate(-25 11 9.6)"/>';
  if (s.blush) out += '<ellipse cx="8.4" cy="19.6" rx="2.4" ry="1.5" fill="#ff6f91" opacity=".6"/><ellipse cx="23.6" cy="19.6" rx="2.4" ry="1.5" fill="#ff6f91" opacity=".6"/>';

  const eyeSet = {
    dots: `<g fill="${ink}"><circle cx="11.3" cy="15" r="1.8"/><circle cx="20.7" cy="15" r="1.8"/></g>`,
    happy: `<g ${pen(ink, 1.5)}><path d="M8.9 15.8q2.4-3.6 4.8 0"/><path d="M18.3 15.8q2.4-3.6 4.8 0"/></g>`,
    wide: `<g fill="#fff" stroke="${ink}" stroke-width=".9"><ellipse cx="11.3" cy="14.6" rx="3.1" ry="3.6"/><ellipse cx="20.7" cy="14.6" rx="3.1" ry="3.6"/></g><g fill="${ink}"><circle cx="11.8" cy="15" r="1.5"/><circle cx="20.2" cy="15" r="1.5"/></g>`,
    closed: `<g ${pen(ink, 1.5)}><path d="M8.8 15q2.5 2.4 5 0"/><path d="M18.2 15q2.5 2.4 5 0"/></g>`,
    hearts: heart(11.3, 14.6, 0.75, '#ff4d7a', ink) + heart(20.7, 14.6, 0.75, '#ff4d7a', ink),
    sparkles: star(11.3, 14.8, 0.95, GOLD, ink) + star(20.7, 14.8, 0.95, GOLD, ink),
    shades: `<g fill="${ink}"><rect x="6.4" y="12" width="8.8" height="5.8" rx="2.6"/><rect x="16.8" y="12" width="8.8" height="5.8" rx="2.6"/></g><path d="M15 13.6h2" ${pen(ink, 1.3)}/><path d="M8.2 14.2l2-.6M18.6 14.2l2-.6" ${pen('#fff', 1)} opacity=".55"/>`,
  };
  const blink = s.animated && s.anim === 'blink';
  const blinkAnim = loop('scale', ['1 1', '1 1', '1 .1', '1 1'], 1.2, { linear: true }).replace('keyTimes="0;.333;.667;1"', 'keyTimes="0;.8;.9;1"');
  out += blink ? `<g transform="translate(0 15)"><g>${blinkAnim}<g transform="translate(0 -15)">${eyeSet[s.eyes]}</g></g></g>` : eyeSet[s.eyes];

  const mouths = {
    smile: `<path d="M12.4 20.4q3.6 4.2 7.2 0" ${pen(ink, 1.4)}/>`,
    grin: `<path d="M11.4 19.8h9.2q0 5.2-4.6 5.2t-4.6-5.2z" fill="#fff" stroke="${ink}" stroke-width="1.3" stroke-linejoin="round"/><path d="M11.8 22.2h8.4" ${pen(ink, .8)}/>`,
    open: `<ellipse cx="16" cy="22.2" rx="2.9" ry="3.2" fill="${ink}"/><ellipse cx="16" cy="23.8" rx="1.7" ry="1.1" fill="#ff7a96"/>`,
    flat: `<path d="M13 22.2h6" ${pen(ink, 1.5)}/>`,
    tongue: `<path d="M15.2 22.4v2.2a1.5 1.5 0 0 0 3 0v-2.6" fill="#ff7a96" stroke="${ink}" stroke-width="1" stroke-linejoin="round"/><path d="M12.4 20.4q3.6 4.2 7.2 0" ${pen(ink, 1.4)}/>`,
    o: `<circle cx="16" cy="22.2" r="1.9" fill="${ink}"/>`,
    wavy: `<path d="M11 22q1.25-2 2.5 0t2.5 0 2.5 0 2.5 0" ${pen(ink, 1.4)}/>`,
  };
  out += mouths[s.mouth];

  const hue2 = (h + 150) % 360;
  if (s.gear === 'hat') {
    out += `<g transform="rotate(-14 16 8)"><path d="M16 2.2L22 11H10z" fill="hsl(${hue2} 80% 62%)" stroke="hsl(${hue2} 60% 26%)" stroke-width="1.2" stroke-linejoin="round"/>`
      + `<g fill="#fff" opacity=".85"><circle cx="15" cy="8" r=".9"/><circle cx="18" cy="9.2" r=".9"/><circle cx="16.4" cy="5.6" r=".8"/></g><circle cx="16" cy="2.2" r="1.7" fill="${GOLD}" stroke="${GOLD_LINE}" stroke-width="1"/></g>`;
  } else if (s.gear === 'crown') {
    out += `<path d="M9.4 10.4L8.4 3.8l4.4 3.4L16 2.6l3.2 4.6 4.4-3.4-1 6.6z" fill="${GOLD}" stroke="${GOLD_LINE}" stroke-width="1.2" stroke-linejoin="round"/><circle cx="16" cy="8.2" r="1.1" fill="#ff4d7a"/>`;
  } else if (s.gear === 'phones') {
    const acc = `hsl(${hue2} 75% 58%)`;
    out += `<path d="M4.8 17.4a11.2 11.2 0 0 1 22.4 0" ${pen(ink, 2)}/><g fill="${acc}" stroke="${ink}" stroke-width="1.2"><rect x="2.8" y="14.8" width="4.4" height="8" rx="1.9"/><rect x="24.8" y="14.8" width="4.4" height="8" rx="1.9"/></g>`;
  }

  const A = s.animated;
  const skin = 'hsl(32 85% 76%)';
  const skinLine = 'hsl(24 55% 30%)';
  switch (s.side) {
    case 'sweat':
      out += `<path d="M25.2 5.4q2.6 3.4 0 5.6q-2.6-2.2 0-5.6z" fill="#8fd8ff" stroke="#2b6f9c" stroke-width="1" stroke-linejoin="round">${A ? '<animate attributeName="opacity" values="1;.5;1" dur="1.2s" repeatCount="indefinite"/>' : ''}</path>`;
      break;
    case 'tear':
      out += `<g><path d="M10.4 18.4q1.7 2.5 0 4q-1.7-1.5 0-4z" fill="#8fd8ff" stroke="#2b6f9c" stroke-width=".9" stroke-linejoin="round"/>${A ? `${loop('translate', ['0 -1', '0 4'], 1.2, { linear: true })}<animate attributeName="opacity" values="1;1;0" keyTimes="0;.6;1" dur="1.2s" repeatCount="indefinite"/>` : ''}</g>`;
      break;
    case 'hand':
      out += `<g transform="translate(26.6 22.6)"><g>${A ? loop('rotate', ['-20 0 4', '20 0 4', '-20 0 4'], 0.6) : ''}`
        + `<g fill="${skin}" stroke="${skinLine}" stroke-width="1" stroke-linejoin="round"><rect x="-3.1" y="-6.2" width="1.9" height="5" rx=".95"/><rect x="-.95" y="-7" width="1.9" height="5.8" rx=".95"/><rect x="1.2" y="-6.2" width="1.9" height="5" rx=".95"/><ellipse cx="0" cy="1.4" rx="3.5" ry="3.7"/><ellipse cx="-3.7" cy="1.4" rx="1.1" ry="1.8" transform="rotate(30 -3.7 1.4)"/></g></g></g>`;
      break;
    case 'question':
      out += `<g ${pen('hsl(48 95% 58%)', 1.7)} ><path d="M23.6 4q0-2.8 3-2.8t3 2.6q0 1.8-2 2.8-1 .7-1 2"/></g><circle cx="27.6" cy="10.6" r=".9" fill="hsl(48 95% 58%)"/>`;
      break;
    case 'excl':
      out += `<path d="M27 1.6v6.2" ${pen('#ff4d4d', 2.2)}>${A ? '<animate attributeName="opacity" values="1;.55;1" dur="0.6s" repeatCount="indefinite"/>' : ''}</path><circle cx="27" cy="10.8" r="1.2" fill="#ff4d4d"/>`;
      break;
    case 'zzz': {
      const f = (b) => (A ? `<animate attributeName="opacity" values=".2;1;.2" dur="1.2s" begin="${b}s" repeatCount="indefinite"/>` : '');
      out += zed(19.5, 7.6, 0.8, ink, f(0)) + zed(23.4, 3.6, 1, ink, f(0.4)) + zed(26.8, 0.8, 1.05, ink, f(0.8));
      break;
    }
  }
  return wrap(out, s);
}

// Zero-width emotes: a transparent overlay that sits on top of another emote.
function overlay(s) {
  const A = s.animated;
  const twinkle = (k) => (A ? `<animate attributeName="opacity" values=".3;1;.3" dur="1.2s" begin="${(k * 0.3).toFixed(1)}s" repeatCount="indefinite"/>` : '');
  const { h } = s;
  let out = '';
  switch (s.kind) {
    case 'hat': {
      const c = `hsl(${h} 80% 62%)`;
      const l = `hsl(${h} 60% 26%)`;
      out = `<g transform="rotate(-12 16 12)"><path d="M16 1.6L24 20H8z" fill="${c}" stroke="${l}" stroke-width="1.4" stroke-linejoin="round"/>`
        + `<path d="M11.6 11.6l8.8 0M9.6 16.2h12.8" ${pen('#fff', 1.4)} opacity=".6"/><circle cx="16" cy="1.8" r="2.4" fill="${GOLD}" stroke="${GOLD_LINE}" stroke-width="1.2"/></g>`;
      break;
    }
    case 'sparkles':
      out = star(8, 9, 2.6, GOLD, GOLD_LINE, twinkle(0)) + star(23, 6, 1.9, GOLD, GOLD_LINE, twinkle(1)) + star(24, 22, 2.8, GOLD, GOLD_LINE, twinkle(2))
        + star(10, 25, 1.7, GOLD, GOLD_LINE, twinkle(3)) + star(16, 15, 1.2, '#fff', GOLD_LINE, twinkle(4));
      break;
    case 'crown':
      out = `<path d="M5 22L3.4 7.6l6.8 5.4L16 4l5.8 9 6.8-5.4L27 22z" fill="${GOLD}" stroke="${GOLD_LINE}" stroke-width="1.5" stroke-linejoin="round"/><path d="M5.6 25.4h20.8" ${pen(GOLD_LINE, 2.2)}/><g fill="#ff4d7a" stroke="${GOLD_LINE}" stroke-width=".8"><circle cx="16" cy="16.4" r="1.9"/><circle cx="9.6" cy="17.6" r="1.3"/><circle cx="22.4" cy="17.6" r="1.3"/></g>`;
      break;
    case 'halo':
      out = `<g><ellipse cx="16" cy="8" rx="10" ry="3.4" ${pen(GOLD_LINE, 4)}/><ellipse cx="16" cy="8" rx="10" ry="3.4" ${pen(GOLD, 2)}/></g>`;
      break;
    case 'hearts': {
      const mv = (k) => (A ? loop('translate', ['0 0', '0 -3.4', '0 0'], 1.2, { begin: (k * 0.2).toFixed(1) }) : '');
      out = `<g>${mv(0)}${heart(9, 22, 1.5, '#ff6f91', '#a02048')}</g><g>${mv(1)}${heart(22, 15, 1.1, '#ff8fb0', '#a02048')}</g><g>${mv(2)}${heart(14, 8, 0.8, '#ffb3c8', '#a02048')}</g>`;
      break;
    }
    case 'shades':
      out = `<g fill="hsl(${h} 45% 14%)" stroke="hsl(${h} 45% 5%)" stroke-width="1" stroke-linejoin="round"><rect x="2.6" y="10.4" width="12" height="9" rx="4"/><rect x="17.4" y="10.4" width="12" height="9" rx="4"/></g><path d="M14.6 13h2.8" ${pen('hsl(' + h + ' 45% 14%)', 1.6)}/><path d="M5.4 13.2l3.4-1M20.2 13.2l3.4-1" ${pen('#fff', 1.2)} opacity=".6"/>`;
      break;
  }
  return wrap(out, s);
}

// Wide (64x32) scenes with no text: a stretching cat, a caterpillar, a sleeping loaf.
function scene(s) {
  const { h, animated: A } = s;
  const fill = `hsl(${h} 78% 64%)`;
  const line = `hsl(${h} 62% 28%)`;
  const ink = `hsl(${h} 55% 13%)`;
  // An outlined limb: the outline colour first, then the fill colour on top.
  const limb = (d, w) => `<path d="${d}" ${pen(line, w + 2)}/><path d="${d}" ${pen(fill, w)}/>`;
  if (s.scene === 0) {
    return `<g>${A ? loop('rotate', ['-14 6.5 13', '12 6.5 13', '-14 6.5 13'], 1.0) : ''}${limb('M6.5 13C2 9 3.6 3.4 8.6 4.2', 2.2)}</g>`
      + limb('M8 17L7 27.6', 3.4) + `<ellipse cx="7" cy="29" rx="3.2" ry="1.7" fill="${fill}" stroke="${line}" stroke-width="1.2"/>`
      + `<path d="M5 12C5 8.5 10 8.5 13 11C24 18 38 21 46 22L47 30C36 29 22 26 12 21C7 19 5 16 5 12z" fill="${fill}" stroke="${line}" stroke-width="1.4" stroke-linejoin="round"/>`
      + `<path d="M22 15.4l-1.2 4.4M28 17.6l-1.2 4.2M34 19.4l-1 4" ${pen(line, 1.4)} opacity=".5"/>`
      + limb('M44 26.4L57.4 28.6', 3.4) + `<ellipse cx="59.4" cy="28.8" rx="2.8" ry="1.9" fill="${fill}" stroke="${line}" stroke-width="1.2"/>`
      + `<g>${A ? loop('translate', ['0 0', '0 1.6', '0 0'], 1.0) : ''}<g fill="${fill}" stroke="${line}" stroke-width="1.3" stroke-linejoin="round"><path d="M45.6 16L46.4 8.4l5.4 4z"/><path d="M58.4 16L57.6 8.4l-5.4 4z"/></g>`
      + `<circle cx="52" cy="19.6" r="7.6" fill="${fill}" stroke="${line}" stroke-width="1.4"/><path d="M47.2 19q1.9-2.4 3.8 0M53 19q1.9-2.4 3.8 0" ${pen(ink, 1.4)}/><ellipse cx="52" cy="21.4" rx="1.1" ry=".8" fill="#ff7a96"/><path d="M50.6 23q1.4 1.5 2.8 0" ${pen(ink, 1)}/><ellipse cx="46.8" cy="22" rx="1.6" ry="1" fill="#ff6f91" opacity=".55"/><ellipse cx="57.2" cy="22" rx="1.6" ry="1" fill="#ff6f91" opacity=".55"/></g>`;
  }
  if (s.scene === 1) {
    let out = '';
    for (let k = 0; k < 5; k++) {
      const x = 8 + k * 9;
      const lt = k % 2 ? 66 : 60;
      out += `<g>${A ? loop('translate', ['0 0', '0 -3', '0 0'], 1.2, { begin: (k * 0.15).toFixed(2) }) : ''}<path d="M${x - 1.6} 26.6v2.2M${x + 1.6} 26.6v2.2" ${pen(line, 1.2)}/><circle cx="${x}" cy="22" r="5.4" fill="hsl(${h} 78% ${lt}%)" stroke="${line}" stroke-width="1.4"/></g>`;
    }
    return out + `<g>${A ? loop('translate', ['0 0', '0 -3', '0 0'], 1.2, { begin: 0.75 }) : ''}<path d="M49.4 13.4q-1.4-4.6-4.4-5.6M56 13.4q1.4-4.6 4.4-5.6" ${pen(line, 1.2)}/><circle cx="45" cy="7.8" r="1.4" fill="${GOLD}" stroke="${line}" stroke-width=".9"/><circle cx="60.4" cy="7.8" r="1.4" fill="${GOLD}" stroke="${line}" stroke-width=".9"/>`
      + `<circle cx="52.6" cy="20" r="8.2" fill="${fill}" stroke="${line}" stroke-width="1.4"/><g fill="#fff" stroke="${ink}" stroke-width=".9"><circle cx="49.4" cy="18.6" r="2.4"/><circle cx="55.8" cy="18.6" r="2.4"/></g><g fill="${ink}"><circle cx="49.9" cy="19" r="1.2"/><circle cx="56.3" cy="19" r="1.2"/></g><path d="M49.2 23.6q3.4 3.4 6.8 0" ${pen(ink, 1.3)}/></g>`;
  }
  const f = (b) => (A ? `<animate attributeName="opacity" values=".15;1;.15" dur="1.2s" begin="${b}s" repeatCount="indefinite"/>` : '');
  return `<g transform="translate(0 29.5)"><g>${A ? loop('scale', ['1 1', '1 1.07', '1 1'], 1.2) : ''}<g transform="translate(0 -29.5)">`
    + `<g fill="${fill}" stroke="${line}" stroke-width="1.3" stroke-linejoin="round"><path d="M10.6 12.6L10 4.6l8 4z"/><path d="M53.4 12.6L54 4.6l-8 4z"/></g>`
    + `<path d="M4 27C4 13 14 8 32 8s28 5 28 19q0 2.6-4 2.6H8Q4 29.6 4 27z" fill="${fill}" stroke="${line}" stroke-width="1.4" stroke-linejoin="round"/>`
    + `<path d="M12 12.6q8-3.2 16-3.4" ${pen('#fff', 1.6)} opacity=".3"/>`
    + `<path d="M18.6 19q3.6 3 7.2 0M38.2 19q3.6 3 7.2 0" ${pen(ink, 1.5)}/><path d="M30 22.4q2 1.6 4 0" ${pen(ink, 1.2)}/><ellipse cx="15.4" cy="22.4" rx="3" ry="1.7" fill="#ff6f91" opacity=".55"/><ellipse cx="48.6" cy="22.4" rx="3" ry="1.7" fill="#ff6f91" opacity=".55"/>`
    + `<g fill="${fill}" stroke="${line}" stroke-width="1.2"><ellipse cx="22" cy="29.4" rx="4.4" ry="2"/><ellipse cx="42" cy="29.4" rx="4.4" ry="2"/></g></g></g></g>`
    // The three z's sit in the free corner above the loaf's right side, inside the 64x32 box (stroke
    // included): the biggest one ends at x 63.6, y 0.4.
    + `${zed(54.8, 8, 0.65, ink, f(0))}${zed(57.2, 4.4, 0.8, ink, f(0.4))}${zed(59.2, 1, 0.95, ink, f(0.8))}`;
}

function picture(i, scale, still = false) {
  // The `_static` files are the first frame: the same drawing without its SMIL animation.
  const s = still ? { ...spec(i), animated: false } : spec(i);
  const W = s.wide ? 64 : 32;
  const body = s.wide ? scene(s) : s.zero ? overlay(s) : face(s);
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W * scale}" height="${32 * scale}" viewBox="0 0 ${W} 32">${body}</svg>`;
}

// ---- Avatars: small original faces, patterns and scenes on a square (the site crops them round).
function avatar(n) {
  const h = (n * 67 + 20) % 360;
  const bg = `hsl(${h} 55% 80%)`;
  const fg = `hsl(${(h + 20) % 360} 70% 60%)`;
  const dk = `hsl(${h} 55% 24%)`;
  const kind = [3, 2, 1, 0, 0, 2, 1, 3, 2, 3, 0, 1][n % 12];
  let art = '';
  if (kind === 0) {
    art = `<circle cx="32" cy="35" r="19" fill="${fg}" stroke="${dk}" stroke-width="2.4"/><circle cx="25" cy="31" r="2.6" fill="${dk}"/><circle cx="39" cy="31" r="2.6" fill="${dk}"/><path d="M24.5 40q7.5 7 15 0" fill="none" stroke="${dk}" stroke-width="2.6" stroke-linecap="round"/><ellipse cx="21" cy="38" rx="3.2" ry="2" fill="#ff6f91" opacity=".55"/><ellipse cx="43" cy="38" rx="3.2" ry="2" fill="#ff6f91" opacity=".55"/>`;
  } else if (kind === 1) {
    art = `<circle cx="32" cy="32" r="26" fill="hsl(${(h + 40) % 360} 65% 66%)"/><circle cx="32" cy="32" r="18" fill="${fg}"/><circle cx="32" cy="32" r="9" fill="${bg}"/><rect x="14" y="14" width="36" height="36" rx="4" transform="rotate(${20 + n * 7} 32 32)" fill="none" stroke="${dk}" stroke-width="2.4"/><circle cx="32" cy="32" r="3.4" fill="${dk}"/>`;
  } else if (kind === 2) {
    art = `<rect width="64" height="64" fill="hsl(${(h + 200) % 360} 60% 78%)"/><circle cx="44" cy="22" r="9" fill="hsl(46 95% 62%)" stroke="${GOLD_LINE}" stroke-width="2"/><path d="M0 50Q16 30 32 46T64 40V64H0z" fill="${fg}" stroke="${dk}" stroke-width="2.4" stroke-linejoin="round"/><path d="M0 58Q20 46 40 56T64 54V64H0z" fill="hsl(${(h + 60) % 360} 55% 44%)" stroke="${dk}" stroke-width="2.4" stroke-linejoin="round"/>`;
  } else {
    art = `<g fill="${fg}" stroke="${dk}" stroke-width="2.4" stroke-linejoin="round"><path d="M12 30L14 8l16 10z"/><path d="M52 30L50 8 34 18z"/><circle cx="32" cy="36" r="20"/></g><g fill="${dk}"><circle cx="24.5" cy="33" r="2.8"/><circle cx="39.5" cy="33" r="2.8"/></g><path d="M29.4 40h5.2l-2.6 3z" fill="#ff7a96" stroke="${dk}" stroke-width="1.6" stroke-linejoin="round"/><path d="M32 43q-3 3.4-6 1.4M32 43q3 3.4 6 1.4" fill="none" stroke="${dk}" stroke-width="2" stroke-linecap="round"/>`;
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><rect width="64" height="64" fill="${bg}"/>${art}</svg>`;
}

// ---- Data -----------------------------------------------------------------------------
const NOW = Date.now();
const DAY = 86400_000;

const emote = (i, owner = OTHERS[i % OTHERS.length], state = 'APPROVED') => {
  const s = spec(i);
  const w = s.wide ? 64 : 32;
  const idx = i < 300 ? i : i % 50;
  return {
    id: `01JD8X${String(i).padStart(20, '0')}`,
    name: nameFor(i),
    state,
    animated: s.animated,
    tags: [...s.tags],
    host: `${BASE}/emotes/${i}`,
    createdAt: new Date(NOW - idx * 0.2 * DAY - Math.floor(rngFor(i, 3)() * 0.18 * DAY)).toISOString(),
    flags: { zeroWidth: s.zero, nsfw: false, private: false },
    // Made-up but stable: a few popular ones, a long tail, some in no channel.
    channelCount: i % 11 === 0 ? 0 : (i * 37) % 91 + (i % 5 === 0 ? 400 : 0),
    owner,
    files: [1, 2, 3, 4].flatMap((k) => [
      { name: `${k}x.webp`, format: 'WEBP', scale: k, width: w * k, height: 32 * k, size: 900 * k, isStatic: false },
      ...(s.animated ? [{ name: `${k}x_static.webp`, format: 'WEBP', scale: k, width: w * k, height: 32 * k, size: 400 * k, isStatic: true }] : []),
    ]),
  };
};

// ---- Twitch global emotes -------------------------------------------------------------
// Names and ids were read from the global-emote listing at https://twitchemotes.com/ and each id was
// checked against https://static-cdn.jtvnw.net/emoticons/v2/<id>/default/dark/{1.0,2.0,3.0} (200 and an
// image). The images belong to Twitch and are only ever referenced by URL: /twitch/<n>/<k>x[_static].webp
// on this mock redirects to the CDN, so the site's `host + /1x.webp` URLs work unchanged. On that CDN
// 1.0 / 2.0 / 3.0 are 1x / 2x / 4x of the same emote (a 28 px emote is 28, 56 and 112 px); there is no 3x.
// Animated ones use the `animated` format (GIF) and `static` for the first frame.
// [name, CDN id, width at 1x, height at 1x, tags, 'a' if animated]
const TWITCH = [
  ['4Head', '354', 20, 30, 'laugh,meme'],
  ['ANELE', '3792', 28, 28, 'happy,meme'],
  ['ArsonNoSexy', '50', 18, 27, 'meme,fire'],
  ['BatChest', '115234', 28, 28, 'hype,meme'],
  ['BCWarrior', '30', 29, 27, 'hype,meme'],
  ['BigBrother', '1904', 24, 30, 'meme,face'],
  ['BloodTrail', '69', 41, 28, 'hype,meme'],
  ['BrainSlug', '115233', 28, 28, 'meme,weird'],
  ['BrokeBack', '4057', 28, 28, 'sad,meme'],
  ['CoolStoryBob', '123171', 28, 28, 'meme,sarcasm'],
  ['CorgiDerp', '49106', 27, 28, 'dog,cute'],
  ['CurseLit', '116625', 28, 28, 'hype,fire'],
  ['DendiFace', '58135', 28, 28, 'face,meme'],
  ['DogFace', '114835', 28, 28, 'dog,meme'],
  ['DoritosChip', '102242', 28, 28, 'meme,food'],
  ['FailFish', '360', 22, 30, 'sad,meme'],
  ['FrankerZ', '65', 40, 30, 'dog,happy'],
  ['FreakinStinkin', '117701', 28, 28, 'meme,face'],
  ['GrammarKing', '3632', 28, 28, 'meme,face'],
  ['HassaanChop', '20225', 28, 28, 'meme,hype'],
  ['HeyGuys', '30259', 28, 28, 'wave,happy'],
  ['Jebaited', '114836', 28, 28, 'meme,troll'],
  ['JonCarnage', '26', 20, 27, 'meme,face'],
  ['KAPOW', '133537', 28, 28, 'hype,meme'],
  ['KappaPride', '55338', 28, 28, 'pride,meme'],
  ['KappaRoss', '70433', 28, 28, 'meme,art'],
  ['KappaWealth', '81997', 28, 28, 'meme,money'],
  ['Kippa', '1901', 24, 28, 'meme,face'],
  ['KomodoHype', '81273', 28, 28, 'hype,happy'],
  ['Kreygasm', '41', 19, 27, 'hype,pog'],
  ['MrDestructoid', '28', 39, 27, 'robot,meme'],
  ['NinjaGrumpy', '138325', 28, 28, 'meme,mad'],
  ['NotLikeThis', '58765', 28, 28, 'sad,meme'],
  ['OneHand', '66', 20, 27, 'meme,hand'],
  ['OptimizePrime', '16', 22, 27, 'robot,meme'],
  ['PJSalt', '36', 36, 30, 'mad,meme'],
  ['PartyTime', '135393', 28, 28, 'happy,hype'],
  ['PeoplesChamp', '3412', 28, 28, 'hype,pog'],
  ['PermaSmug', '27509', 28, 28, 'smug,meme'],
  ['PogChamp', '305954156', 28, 28, 'pog,hype'],
  ['Poooound', '117484', 28, 28, 'hype,meme'],
  ['PopCorn', '724216', 28, 28, 'food,meme'],
  ['RaccAttack', '114870', 28, 28, 'cute,hype'],
  ['RalpherZ', '1900', 33, 30, 'dog,happy'],
  ['RedCoat', '22', 19, 27, 'meme,face'],
  ['RlyTho', '134256', 28, 28, 'meme,sarcasm'],
  ['RuleFive', '107030', 28, 28, 'meme,face'],
  ['SMOrc', '52', 32, 32, 'hype,meme'],
  ['SeriousSloth', '81249', 28, 28, 'cute,meme'],
  ['ShadyLulu', '52492', 28, 28, 'meme,face'],
  ['ShazBotstix', '87', 24, 30, 'meme,face'],
  ['StrawBeary', '114876', 28, 28, 'cute,happy'],
  ['SwiftRage', '34', 21, 28, 'mad,meme'],
  ['TBAngel', '143490', 28, 28, 'happy,cute'],
  ['TearGlove', '160403', 28, 28, 'sad,cute'],
  ['TheRinger', '18', 20, 27, 'meme,face'],
  ['TheTarFu', '111351', 28, 28, 'meme,face'],
  ['TooSpicy', '114846', 28, 28, 'meme,fire'],
  ['TriHard', '120232', 28, 28, 'hype,meme'],
  ['TwitchLit', '166263', 28, 28, 'hype,fire'],
  ['UWot', '134255', 28, 28, 'meme,confused'],
  ['UnSane', '111792', 28, 28, 'meme,mad'],
  ['UncleNox', '114856', 28, 28, 'meme,face'],
  ['VoteYea', '106293', 28, 28, 'vote,happy'],
  ['WTRuck', '114847', 28, 28, 'meme,truck'],
  ['WholeWheat', '1896', 20, 30, 'meme,food'],
  ['YouWHY', '4337', 28, 28, 'sad,meme'],
  ['bleedPurple', '62835', 28, 28, 'twitch,hype'],
  ['cmonBruh', '84608', 28, 28, 'meme,sarcasm'],
  ['imGlitch', '112290', 28, 28, 'twitch,meme'],
  ['mcaT', '35063', 28, 28, 'cat,meme'],
  ['panicBasket', '22998', 28, 28, 'meme,panic'],
  ['twitchRaid', '62836', 28, 28, 'twitch,hype'],
  ['HmmFace', 'emotesv2_e455cf70a96e44db9da533b89d40f062', 28, 28, 'meme,thinking'],
  ['Shush', 'emotesv2_819621bcb8f44566a1bd8ea63d06c58f', 28, 28, 'meme,quiet'],
  ['DinoDance', 'emotesv2_dcd06b30a5c24f6eb871e8f5edbd44f7', 28, 28, 'dance,happy', 'a'],
  ['YappingDuck', 'emotesv2_bea88a2caa044e409c1b32ee0850b825', 28, 28, 'meme,duck', 'a'],
  ['ClapHands', 'emotesv2_cb4c93c8bef44ae6888cf239f98754b7', 28, 28, 'clap,happy', 'a'],
];
const CDN = 'https://static-cdn.jtvnw.net/emoticons/v2';
const CDN_SCALE = { 1: '1.0', 2: '2.0', 3: '3.0', 4: '3.0' }; // 3x has no CDN size: the 4x file, shown smaller
const twitchUrl = (n, k, still) => {
  const t = TWITCH[n];
  if (!t || !CDN_SCALE[k]) return null;
  const format = t[5] ? (still ? 'static' : 'animated') : 'default';
  return `${CDN}/${t[1]}/${format}/dark/${CDN_SCALE[k]}`;
};
const twitchEmote = (n, owner = OTHERS[n % OTHERS.length]) => {
  const [name, , w, h, tags, a] = TWITCH[n];
  const animated = a === 'a';
  const idx = n;
  return {
    id: `01JD8X${String(n).padStart(20, '0')}`,
    name,
    state: 'APPROVED',
    animated,
    tags: tags.split(','),
    host: `${BASE}/twitch/${n}`,
    createdAt: new Date(NOW - idx * 0.2 * DAY - Math.floor(rngFor(n, 3)() * 0.18 * DAY)).toISOString(),
    flags: { zeroWidth: false, nsfw: false, private: false },
    channelCount: n % 11 === 0 ? 0 : (n * 37) % 91 + (n % 5 === 0 ? 400 : 0),
    owner,
    files: [1, 2, 3, 4].flatMap((k) => [
      { name: `${k}x.webp`, format: 'WEBP', scale: k, width: w * k, height: h * k, size: 900 * k, isStatic: false },
      ...(animated ? [{ name: `${k}x_static.webp`, format: 'WEBP', scale: k, width: w * k, height: h * k, size: 400 * k, isStatic: true }] : []),
    ]),
  };
};
const ALL = FACES ? Array.from({ length: 300 }, (_, i) => emote(i)) : TWITCH.map((_, n) => twitchEmote(n));
// The mock account's own uploads: a few of each state.
const MINE = Array.from({ length: 9 }, (_, i) =>
  emote(600 + i, ME_SUMMARY, i < 2 ? 'PENDING' : i === 2 ? 'REJECTED' : 'APPROVED'));
let uploads = 0;

const member = (e, name = e.name) => ({ name, zeroWidth: e.flags.zeroWidth, addedAt: e.createdAt, emote: e });
const SETS = [
  { id: '01JD8XSET00000000000000MAIN', name: 'Main', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: [...MINE.slice(3, 7), ...ALL.slice(10, 30)].map((e) => member(e)), editors: [OTHERS[0]], visibility: 'LISTED', createdAt: '2026-11-10T12:05:00.000Z' },
  { id: '01JD8XSET0000000000000ALTSS', name: 'Subathon', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: ALL.slice(40, 46).map((e) => member(e)), editors: [], visibility: 'UNLISTED', createdAt: '2026-11-12T19:40:00.000Z' },
  { id: '01JD8XSET000000000000000OAK', name: 'quietoak main', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[0],
    emotes: ALL.slice(60, 70).map((e) => member(e)), editors: [ME_SUMMARY], visibility: 'LISTED', createdAt: '2026-11-08T09:00:00.000Z' },
];
const GLOBAL = { id: '01JD8XSET0000000000000GLOBL', name: 'Global', kind: 'GLOBAL', capacity: 200, owner: null,
  emotes: ALL.slice(0, 24).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-01T00:00:00.000Z' };
SETS.push(GLOBAL);
const PENDING = Array.from({ length: 7 }, (_, i) => emote(800 + i, OTHERS[i % 2], 'PENDING'));
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
  { platform: 'TWITCH', platformUserId: '141981764', login: 'lumen_fox', displayName: 'lumen_fox', avatar: ME_SUMMARY.avatar },
];
const NOTES = [
  { id: 'N1', kind: 'emote.rejected', payload: JSON.stringify({ name: MINE[2].name, reason: 'Looks like a copy of a well-known emote.' }), createdAt: '2026-11-16T21:00:00.000Z', read: false },
];
// Account editors (ADR 0014): two people work on the mock account, and it works on two others'.
const EDITOR_FLAGS = ['EMOTES', 'SETS', 'EDITORS', 'PROFILE'];
const ACCOUNT_EDITORS = [
  { account: OTHERS[1], permissions: ['EMOTES'], grantedBy: ME_SUMMARY, grantedAt: '2026-11-12T10:00:00.000Z', updatedAt: '2026-11-12T10:00:00.000Z' },
  { account: OTHERS[2], permissions: ['EMOTES', 'SETS', 'EDITORS'], grantedBy: ME_SUMMARY, grantedAt: '2026-11-14T18:30:00.000Z', updatedAt: '2026-11-14T18:30:00.000Z' },
];
const EDITOR_OF = [
  { account: OTHERS[0], permissions: ['EMOTES', 'SETS'], grantedAt: '2026-11-13T09:00:00.000Z' },
  { account: OTHERS[4], permissions: ['EMOTES', 'PROFILE'], grantedAt: '2026-11-15T20:00:00.000Z' },
];
const SHARED_SETS = [
  { id: '01JD8XSET0000000000000OAKAL', name: 'quietoak cozy', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[0],
    emotes: ALL.slice(70, 78).map((e) => member(e)), editors: [], visibility: 'PRIVATE', createdAt: '2026-11-13T09:10:00.000Z' },
  { id: '01JD8XSET0000000000000FERN1', name: 'fern garden', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[4],
    emotes: ALL.slice(46, 54).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-15T20:30:00.000Z' },
];
// Public sets of other accounts (accountSets, cloneSet): listed, unlisted and private, so a
// profile shows only the listed ones and a private one 404s to everyone else.
const PUBLIC_SETS = [
  { id: '01JD8XSET0000000000000NORA1', name: 'nora picks', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[1],
    emotes: ALL.slice(5, 23).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-09T15:00:00.000Z' },
  { id: '01JD8XSET0000000000000NORA2', name: 'cozy nights', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[1],
    emotes: ALL.slice(30, 38).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-14T22:00:00.000Z' },
  { id: '01JD8XSET0000000000000NORA3', name: 'drafts', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[1],
    emotes: ALL.slice(80, 84).map((e) => member(e)), editors: [], visibility: 'PRIVATE', createdAt: '2026-11-16T08:00:00.000Z' },
  { id: '01JD8XSET0000000000000NORA4', name: 'nora personal', kind: 'PERSONAL', capacity: 5, owner: OTHERS[1],
    emotes: ALL.slice(90, 93).map((e) => member(e)), editors: [], visibility: 'UNLISTED', createdAt: '2026-11-11T11:00:00.000Z' },
];
const EDITOR_CHANNELS = [
  { platform: 'TWITCH', platformChannelId: '55500001', login: 'quietoak', activeSetId: SETS[2].id, allowPersonalEmotes: true, account: OTHERS[0] },
  { platform: 'KICK', platformChannelId: '55500002', login: 'quietoak', activeSetId: null, allowPersonalEmotes: false, account: OTHERS[0] },
  { platform: 'TWITCH', platformChannelId: '55500003', login: 'sunnyfern', activeSetId: SHARED_SETS[1].id, allowPersonalEmotes: true, account: OTHERS[4] },
];
// The sets of the accounts the mock account edits: theirs among SETS (per-set grants) and SHARED_SETS.
const editorSets = () => [...SETS, ...SHARED_SETS].filter((s) => s.owner && EDITOR_OF.some((g) => g.account.id === s.owner.id));
const editorChannels = () => EDITOR_CHANNELS.filter((c) => EDITOR_OF.some((g) => g.account.id === c.account.id));
const flagsOn = (ownerId) => (ownerId === ME_ID ? EDITOR_FLAGS : EDITOR_OF.find((g) => g.account.id === ownerId)?.permissions ?? []);
const editorOfOut = () => EDITOR_OF.map((g) => ({ ...g }));

const PERMISSIONS = ['emotes.review', 'emotes.takedown', 'emotes.edit_any', 'sets.edit_any', 'global_set.edit', 'users.view', 'users.restrict', 'roles.manage', 'audit.view', 'cosmetics.manage', 'cosmetics.grant'];

// ---- Chat cosmetics (ADR 0015) ----------------------------------------------------------
// SAMPLE DATA ONLY: two made-up badges (simple SVG drawn here) and three sample paints, so the
// site can be built before any real cosmetics exist. Pictures are served by this mock as SVG under
// /cosmetics/badges/<key>/<n>x.webp. Operations are recognised by their root field; arguments are
// read from variables named like the arguments ($id, $input, $badgeId, $paintId, $ownerId,
// $accountId, $kind, $cosmeticId, $platform, $userIds, $reason, $page, $perPage).
const BADGE_ART = {
  'sample-star': (k) => `<svg xmlns="http://www.w3.org/2000/svg" width="${18 * k}" height="${18 * k}" viewBox="0 0 18 18"><circle cx="9" cy="9" r="8.5" fill="hsl(268 70% 55%)"/><path d="M9 3.2l1.7 3.6 3.9.5-2.9 2.7.8 3.9L9 12l-3.5 1.9.8-3.9-2.9-2.7 3.9-.5z" fill="hsl(46 95% 62%)"/></svg>`,
  'sample-shield': (k) => `<svg xmlns="http://www.w3.org/2000/svg" width="${18 * k}" height="${18 * k}" viewBox="0 0 18 18"><path d="M9 1l7 2.6v5c0 4.2-3 7.3-7 8.4-4-1.1-7-4.2-7-8.4v-5z" fill="hsl(160 60% 38%)"/><path d="M5.6 9.2l2.3 2.3 4.6-4.7" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
};
const C_TIME = '2026-11-17T12:00:00.000Z';
const cImage = (key, kind = 'BADGE') => ({
  id: `01JD8XIMG${key.replace(/[^a-z]/g, '').toUpperCase().padEnd(17, '0').slice(0, 17)}`,
  kind, host: `${BASE}/cosmetics/${kind === 'BADGE' ? 'badges' : 'paints'}/${key}`, animated: false,
  width: 72, height: 72, ready: true, createdAt: C_TIME,
  files: [1, 2, 4].map((k) => ({ name: `${k}x.webp`, format: 'WEBP', scale: k, width: 18 * k, height: 18 * k, size: 300 * k, isStatic: false })),
});
const C_IMAGES = [cImage('sample-star'), cImage('sample-shield')];
const BADGES = [
  { id: '01JD8XBDG000000000000000001', name: 'Sample: Early', tooltip: 'Sample badge (mock data)', image: C_IMAGES[0], createdAt: C_TIME, updatedAt: C_TIME },
  { id: '01JD8XBDG000000000000000002', name: 'Sample: Helper', tooltip: 'Sample badge (mock data)', image: C_IMAGES[1], createdAt: C_TIME, updatedAt: C_TIME },
];
const paintOf = (id, name, kind, rest) => ({ id, name, kind, angle: 90, repeat: false, shape: 'CIRCLE', color: null, stops: [], shadows: [], image: null, createdAt: C_TIME, updatedAt: C_TIME, ...rest });
const PAINTS = [
  paintOf('01JD8XPNT000000000000000001', 'Sample Sunset', 'LINEAR_GRADIENT', { angle: 90, color: '#FF7A45',
    stops: [{ at: 0, color: '#FF7A45' }, { at: 0.5, color: '#FF3D7F' }, { at: 1, color: '#8F5BFF' }] }),
  paintOf('01JD8XPNT000000000000000002', 'Sample Aurora', 'RADIAL_GRADIENT', { shape: 'ELLIPSE', color: '#3DDC97',
    stops: [{ at: 0, color: '#3DDC97' }, { at: 0.6, color: '#3BA7FF' }, { at: 1, color: '#7B61FF' }] }),
  paintOf('01JD8XPNT000000000000000003', 'Sample Ember', 'LINEAR_GRADIENT', { angle: 45, repeat: true, color: '#FFB020',
    stops: [{ at: 0, color: '#FFB020' }, { at: 0.25, color: '#FF5A1F' }, { at: 0.5, color: '#FFB020' }],
    shadows: [{ x: 0, y: 1, radius: 2, color: '#00000099' }] }),
];
// Who owns what (entitlements) and what each account shows.
const C_OWNED = {
  [ME_ID]: { BADGE: [BADGES[0].id, BADGES[1].id], PAINT: [PAINTS[0].id, PAINTS[1].id] },
  [OTHERS[0].id]: { BADGE: [BADGES[1].id], PAINT: [PAINTS[2].id] },
  [OTHERS[4].id]: { BADGE: [], PAINT: [PAINTS[1].id] },
};
const C_SELECTED = {
  [ME_ID]: { badgeId: BADGES[0].id, paintId: PAINTS[0].id },
  [OTHERS[0].id]: { badgeId: BADGES[1].id, paintId: PAINTS[2].id },
  [OTHERS[4].id]: { badgeId: null, paintId: PAINTS[1].id },
};
const C_GRANTS = Object.entries(C_OWNED).flatMap(([account, kinds]) => Object.entries(kinds).flatMap(([kind, list]) =>
  list.map((cosmeticId) => ({ accountId: account, kind, cosmeticId, source: 'GRANT', reason: 'Sample grant (mock data)', grantedBy: ME_SUMMARY, grantedAt: C_TIME }))));
// Chatters the batch lookup knows (platform user id -> account).
const C_CHATTERS = { '141981764': ME_ID, '55500001': OTHERS[0].id, '55500003': OTHERS[4].id };
let cosmeticSeq = 0;
const cosmeticCatalog = () => ({ badges: BADGES, paints: PAINTS });
const ownedOut = (accountId) => {
  const owned = C_OWNED[accountId] ?? { BADGE: [], PAINT: [] };
  const sel = C_SELECTED[accountId] ?? { badgeId: null, paintId: null };
  return { account: publicAccount(accountId) ?? { id: accountId, displayName: '', avatar: null },
    badges: BADGES.filter((b) => owned.BADGE.includes(b.id)), paints: PAINTS.filter((p) => owned.PAINT.includes(p.id)),
    badgeId: sel.badgeId, paintId: sel.paintId };
};
const userCosmeticsOut = (platform, ids = []) => {
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
function cosmeticsAnswer(q, v, session, admin) {
  const has = (field) => new RegExp(`\\b${field}\\s*[({]`).test(q) || new RegExp(`\\b${field}\\s*$`, 'm').test(q);
  const mutation = /^\s*mutation/.test(q);
  const staff = (perm) => (!session ? err('Sign in to use this.', 'signed_out') : admin ? null : err(`This needs ${perm}.`, 'forbidden'));
  if (!mutation) {
    const fields = ['cosmetics', 'userCosmetics', 'myCosmetics', 'cosmeticImage', 'cosmeticEntitlements'].filter(has);
    if (!fields.length) return null;
    const data = {};
    for (const f of fields) {
      if (f === 'cosmetics') data.cosmetics = cosmeticCatalog();
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
        if (BADGES.some((b) => b !== before && b.name.toLowerCase() === name.trim().toLowerCase())) return err('A badge or paint with that name exists already.', 'name_taken');
        const tooltip = input.tooltip ?? before?.tooltip;
        if (typeof tooltip !== 'string' || !tooltip.trim() || tooltip.trim().length > 64) return err('Tooltips are 1 to 64 characters.', 'bad_tooltip');
        const image = input.imageId ? C_IMAGES.find((i) => i.id === input.imageId && i.kind === 'BADGE') : before?.image;
        if (!image) return err('No badge picture with that ID.', 'bad_image');
        if (!image.ready) return err('That picture\u2019s files are still being made. Try again in a moment.', 'image_not_ready');
        const now = new Date().toISOString();
        const badge = { ...(before ?? { id: `01JD8XBDG${String(100 + cosmeticSeq++).padStart(17, '0')}`, createdAt: now }), name: name.trim(), tooltip: tooltip.trim(), image, updatedAt: now };
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
        if (PAINTS.some((x) => x !== before && x.name.toLowerCase() === input.name.trim().toLowerCase())) return err('A badge or paint with that name exists already.', 'name_taken');
        if (input.imageId && !C_IMAGES.some((i) => i.id === input.imageId && i.kind === 'PAINT')) return err('No paint picture with that ID.', 'bad_image');
        const paint = paintFrom(before?.id ?? `01JD8XPNT${String(100 + cosmeticSeq++).padStart(17, '0')}`, input, before);
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
// Public channel lookup: the mock account's own channel, plus every channel emoteChannels() can
// return, each with an active set, so a link from "used in" always lands on a channel page.
const channelIndex = new Map();
const channelByLogin = (platform, login) => {
  if (typeof login !== 'string') return null;
  if (!channelIndex.size) {
    for (const e of ALL)
      for (const c of emoteChannels(e, e.channelCount)) {
        const key = `${c.platform}/${c.login.toLowerCase()}`;
        if (!channelIndex.has(key)) channelIndex.set(key, c);
      }
  }
  if (login.toLowerCase() === 'lumen_fox') return { ...CHANNELS[0], platform };
  return channelIndex.get(`${platform}/${login.toLowerCase()}`) ?? null;
};
const sorted = (hits, sort) => {
  if (sort === 'TOP') return [...hits].sort((a, b) => b.channelCount - a.channelCount || a.name.localeCompare(b.name));
  if (sort === 'NEW') return [...hits].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  return hits;
};

const findEmote = (id) => MINE.find((e) => e.id === id) ?? ALL.find((e) => e.id === id);
const findSet = (id) => SETS.find((s) => s.id === id) ?? SHARED_SETS.find((s) => s.id === id) ?? PUBLIC_SETS.find((s) => s.id === id);
const ALL_SETS = () => [...SETS, ...SHARED_SETS, ...PUBLIC_SETS];
const ACTIVE_SET_IDS = () => new Set([...CHANNELS, ...EDITOR_CHANNELS].map((c) => c.activeSetId).filter(Boolean));
// The API's rule: anyone opens listed and unlisted sets; a private one only its owner, editors and
// account editors, or anyone while a channel shows it.
const mayView = (s, session) => s.visibility !== 'PRIVATE' || ACTIVE_SET_IDS().has(s.id)
  || (session && (s.owner?.id === ME_ID || s.editors.some((e) => e.id === ME_ID) || EDITOR_OF.some((g) => g.account.id === s.owner?.id)));
const err = (message, code) => ({ data: null, errors: [{ message, extensions: { code } }] });
const setOut = (s) => ({ ...s, editors: undefined, visibility: s.visibility ?? 'LISTED', listed: (s.visibility ?? 'LISTED') === 'LISTED',
  slotsUsed: s.emotes.length, createdAt: s.createdAt ?? '2026-11-10T12:00:00.000Z' });
const setCard = (s, preview = 12) => {
  const o = setOut(s);
  return { id: o.id, name: o.name, kind: o.kind, owner: o.owner, visibility: o.visibility, listed: o.listed, capacity: o.capacity,
    slotsUsed: o.slotsUsed, active: ACTIVE_SET_IDS().has(o.id), createdAt: o.createdAt,
    preview: [...o.emotes].reverse().slice(0, Math.max(0, Math.min(50, preview))) };
};
// accountSets(accountId, preview): an account's listed sets as cards, signed in or not.
const accountSets = (accountId, preview) =>
  ALL_SETS().filter((s) => s.owner?.id === accountId && s.kind !== 'GLOBAL' && (s.visibility ?? 'LISTED') === 'LISTED'
    && !(STATUS[accountId]?.blocked)).slice(0, 50).map((s) => setCard(s, preview ?? 12));
const VISIBILITIES = ['LISTED', 'UNLISTED', 'PRIVATE'];
let clones = 0;
// cloneSet(id, name, ownerId, visibility): the API's rules in small: approved emotes only,
// others' private emotes skipped, capacity 1000.
function cloneSet(v, session) {
  const src = findSet(v.id);
  if (!src || !mayView(src, session)) return err('Not found.', 'not_found');
  const ownerId = v.ownerId || ME_ID;
  if (!flagsOn(ownerId).includes('SETS'))
    return err('Only the account’s owner and its editors with the sets permission can make sets for it.', 'forbidden');
  if (++clones > 30) return err('That’s a lot of copied sets. Try again in a while.', 'rate_limited');
  const name = (v.name ?? src.name).trim();
  if (!name || name.length > 100) return err('Set names are 1 to 100 characters.', 'bad_name');
  if (v.visibility && !VISIBILITIES.includes(v.visibility)) return err('Unknown visibility.', 'bad_visibility');
  const owner = ownerId === ME_ID ? ME_SUMMARY : EDITOR_OF.find((g) => g.account.id === ownerId).account;
  const keep = src.emotes.filter((m) => m.emote.state === 'APPROVED' && (!m.emote.flags.private || m.emote.owner?.id === ownerId));
  const copied = keep.slice(0, 1000);
  const s = { id: `01JD8XSETCPY${String(clones).padStart(14, '0')}`, name, kind: 'CHANNEL', capacity: 1000, owner,
    emotes: copied.map((m) => ({ ...m, addedAt: new Date().toISOString() })), editors: [], visibility: v.visibility ?? 'LISTED',
    createdAt: new Date().toISOString() };
  (ownerId === ME_ID ? SETS : SHARED_SETS).push(s);
  return { data: { cloneSet: { set: setOut(s), copied: copied.length, skipped: src.emotes.length - copied.length } } };
}

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
      return { data: { mySets: SETS.map(setOut), editorSets: editorSets().map(setOut), editorOf: editorOfOut() } };
    case 'MySetNames':
      return { data: { mySets: SETS.map((s) => ({ id: s.id, name: s.name, kind: s.kind, emotes: s.emotes.map((m) => ({ emote: { id: m.emote.id } })) })) } };
    case 'ReportEmote':
      if (!v.reason?.trim()) return err('Give a reason of 1 to 500 characters; the uploader sees it.', 'bad_reason');
      return { data: { reportEmote: { id: 'R' + Date.now() } } };
    case 'EditSet': {
      const s = findSet(v.id);
      if (!s) return { data: { set: null, setEditors: [], editorOf: editorOfOut() } };
      return { data: { set: setOut(s), setEditors: s.editors, editorOf: editorOfOut() } };
    }
    case 'CreateSet': {
      const ownerId = v.input.ownerId || ME_ID;
      if (!flagsOn(ownerId).includes('SETS'))
        return err('Only the account’s owner and its editors with the sets permission can make sets for it.', 'forbidden');
      const owner = ownerId === ME_ID ? ME_SUMMARY : EDITOR_OF.find((g) => g.account.id === ownerId).account;
      const list = ownerId === ME_ID ? SETS : SHARED_SETS;
      if (v.input.kind === 'PERSONAL' && [...SETS, ...SHARED_SETS].some((s) => s.kind === 'PERSONAL' && s.owner?.id === ownerId))
        return err('You already have a personal set.', 'personal_set_exists');
      const visibility = v.input.visibility ?? (v.input.kind === 'PERSONAL' ? 'UNLISTED' : 'LISTED');
      if (v.input.kind === 'PERSONAL' && visibility === 'PRIVATE')
        return err('Personal sets can’t be private, and the global set is always listed.', 'bad_visibility');
      const s = { id: `01JD8XSETNEW${String(SETS.length + SHARED_SETS.length).padStart(14, '0')}`, name: v.input.name, kind: v.input.kind,
        capacity: v.input.kind === 'PERSONAL' ? 5 : 1000, owner, emotes: [], editors: [], visibility, createdAt: new Date().toISOString() };
      list.push(s);
      return { data: { createSet: setOut(s) } };
    }
    case 'UpdateSet': {
      const s = findSet(v.id);
      if (!s) return err('Not found.', 'not_found');
      if (v.input.visibility) {
        if (s.kind === 'GLOBAL' || (s.kind === 'PERSONAL' && v.input.visibility === 'PRIVATE'))
          return err('Personal sets can’t be private, and the global set is always listed.', 'bad_visibility');
        if (!flagsOn(s.owner?.id).includes('SETS'))
          return err('Only the set’s owner and their editors with the sets permission can change who sees it.', 'forbidden');
        s.visibility = v.input.visibility;
      }
      if (v.input.name) s.name = v.input.name;
      return { data: { updateSet: setOut(s) } };
    }
    case 'DeleteSet': {
      if ([...CHANNELS, ...EDITOR_CHANNELS].some((c) => c.activeSetId === v.id)) return err('A channel shows this set. Pick another active set first.', 'set_in_use');
      const list = SETS.some((s) => s.id === v.id) ? SETS : SHARED_SETS;
      list.splice(list.findIndex((s) => s.id === v.id), 1);
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
    case 'MyChannels': {
      const short = (s) => ({ id: s.id, name: s.name, kind: s.kind, owner: s.owner ? { id: s.owner.id } : null });
      return { data: { myChannels: CHANNELS, mySets: SETS.map(short), editorChannels: editorChannels(), editorSets: editorSets().map(short), editorOf: editorOfOut() } };
    }
    case 'UpdateChannel': {
      const c = [...CHANNELS, ...editorChannels()].find((x) => x.platform === v.platform && x.platformChannelId === v.id);
      if (!c || !flagsOn(c.account.id).includes('SETS'))
        return err('Only the channel’s owner and its account editors with the sets permission can change it.', 'forbidden');
      if (v.input.activeSetId !== undefined) c.activeSetId = v.input.activeSetId || null;
      if (v.input.allowPersonalEmotes !== undefined) c.allowPersonalEmotes = v.input.allowPersonalEmotes;
      return { data: { updateChannelSettings: { activeSetId: c.activeSetId, allowPersonalEmotes: c.allowPersonalEmotes } } };
    }
    case 'Editors':
      return { data: { accountEditors: ACCOUNT_EDITORS, editorOf: editorOfOut() } };
    case 'AddAccountEditor': {
      const { login, permissions } = v.input;
      if (!permissions?.length) return err('Pick at least one permission.', 'no_permissions');
      if (ACCOUNT_EDITORS.length >= 20) return err('An account can have at most 20 editors.', 'too_many_editors');
      const name = String(login ?? '').trim().replace(/^@/, '').toLowerCase();
      if (name === ME_SUMMARY.displayName.toLowerCase()) return err('The account’s owner can do all of this already.', 'already_owner');
      const a = OTHERS.find((o) => o.displayName.toLowerCase() === name);
      if (!a) return err('Not found.', 'not_found');
      if (ACCOUNT_EDITORS.some((e) => e.account.id === a.id)) return err('They are an editor already.', 'already_editor');
      const now = new Date().toISOString();
      const ed = { account: a, permissions: EDITOR_FLAGS.filter((f) => permissions.includes(f)), grantedBy: ME_SUMMARY, grantedAt: now, updatedAt: now };
      ACCOUNT_EDITORS.push(ed);
      return { data: { addAccountEditor: ed } };
    }
    case 'UpdateAccountEditor': {
      const ed = ACCOUNT_EDITORS.find((e) => e.account.id === v.accountId);
      if (!ed) return err('Not found.', 'not_found');
      if (!v.permissions?.length) return err('Pick at least one permission.', 'no_permissions');
      ed.permissions = EDITOR_FLAGS.filter((f) => v.permissions.includes(f));
      ed.updatedAt = new Date().toISOString();
      return { data: { updateAccountEditor: ed } };
    }
    case 'RemoveAccountEditor': {
      if (v.ownerId && v.ownerId !== ME_ID) {
        const i = EDITOR_OF.findIndex((g) => g.account.id === v.ownerId);
        if (i >= 0 && v.accountId === ME_ID) EDITOR_OF.splice(i, 1);
        return { data: { removeAccountEditor: i >= 0 } };
      }
      const i = ACCOUNT_EDITORS.findIndex((e) => e.account.id === v.accountId);
      if (i >= 0) ACCOUNT_EDITORS.splice(i, 1);
      return { data: { removeAccountEditor: i >= 0 } };
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

// Fields added after the site's first queries, answered by field name so any operation name works.
function newFields(q, v, session) {
  if (/\bcloneSet\s*\(/.test(q)) return session ? cloneSet(v, session) : err('Sign in to use this.', 'signed_out');
  if (/\baccountSets\s*\(/.test(q)) {
    const data = { accountSets: accountSets(v.accountId ?? v.id ?? '', v.preview) };
    if (/\baccount\s*\(/.test(q)) data.account = publicAccount(v.accountId ?? v.id ?? '');
    return { data };
  }
  return null;
}

// New fields merge with whatever else the same operation asks for (the older handlers below).
function answer(body, session, admin) {
  const added = newFields(body.query ?? '', body.variables ?? {}, session);
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
    const terms = (v.query ?? '').toLowerCase().split(/\s+/).filter(Boolean);
    let hits = ALL.filter((e) => terms.every((t) => e.name.toLowerCase().includes(t) || e.tags.some((g) => g.includes(t))));
    if (v.animated === true) hits = hits.filter((e) => e.animated);
    if (v.animated === false) hits = hits.filter((e) => !e.animated);
    if (v.owner) hits = hits.filter((e) => e.owner?.id === v.owner);
    hits = sorted(hits, v.sort ?? 'RELEVANCE');
    const per = v.perPage ?? 50;
    const page = v.page ?? 1;
    return { data: { searchEmotes: { total: hits.length, page, perPage: per, items: hits.slice((page - 1) * per, page * per) } } };
  }
  if (q.includes('query Emote(')) return { data: { emote: ALL.find((e) => e.id === v.id) ?? null } };
  if (q.includes('query Channel') || q.includes('query FindAccount')) {
    const c = channelByLogin(v.platform, v.login);
    return { data: { channelByLogin: c } };
  }
  if (q.includes('query Set(') || /\bset\s*\(\s*id:/.test(q)) {
    // Unknown ids fall back to the first set, as before; private ones you can't see are null.
    const s = findSet(v.id) ?? SETS[0];
    return { data: { set: mayView(s, session) ? setOut(s) : null } };
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

createServer(async (req, res) => {
  const SITE = siteOf(req);
  res.setHeader('access-control-allow-origin', SITE);
  res.setHeader('access-control-allow-credentials', 'true');
  res.setHeader('access-control-allow-headers', 'content-type');
  if (req.method === 'OPTIONS') return res.writeHead(204).end();
  const session = OPEN_ENV || /(?:^|;\s*)mock_session=1/.test(req.headers.cookie ?? '');
  const url = new URL(req.url ?? '/', BASE);
  const admin = ADMIN_ENV || /(?:^|;\s*)mock_admin=1/.test(req.headers.cookie ?? '');
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
  const img = url.pathname.match(/^\/emotes\/(\d+)\/(\d)x(_static)?\.webp$/);
  if (img) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(picture(Number(img[1]), Number(img[2]), Boolean(img[3])));
  }
  const badgeArt = url.pathname.match(/^\/cosmetics\/badges\/([a-z-]+)\/([124])x\.webp$/);
  if (badgeArt && BADGE_ART[badgeArt[1]]) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(BADGE_ART[badgeArt[1]](Number(badgeArt[2])));
  }
  if (req.method === 'GET' && url.pathname === '/v1/cosmetics') {
    const c = cosmeticCatalog();
    const toRest = (x) => JSON.parse(JSON.stringify(x, (k, val) => (typeof val === 'string' && /^[A-Z_]+$/.test(val) && ['kind', 'shape', 'format'].includes(k) ? val.toLowerCase() : val))
      .replace(/"([a-z]+)([A-Z][a-zA-Z]*)":/g, (_, a, b) => `"${a}${b.replace(/[A-Z]/g, (m) => '_' + m.toLowerCase())}":`));
    return json(res, 200, toRest(c));
  }
  if (req.method === 'GET' && url.pathname === '/v1/cosmetics/users') {
    const ids = (url.searchParams.get('ids') ?? '').split(',').filter(Boolean);
    const out = userCosmeticsOut((url.searchParams.get('platform') ?? '').toUpperCase(), ids);
    if (!out) return json(res, 400, { error: 'too_many', message: 'Look up at most 100 users at once.' });
    return json(res, 200, { users: out.map((u) => ({ platform_user_id: u.platformUserId, badge_id: u.badgeId, paint_id: u.paintId })) });
  }
  if (req.method === 'POST' && url.pathname === '/v1/cosmetics/images') {
    const raw = (await read(req)).toString('latin1');
    if (!session) return json(res, 401, { error: 'signed_out', message: 'Sign in to upload.' });
    if (!admin) return json(res, 403, { error: 'forbidden', message: 'This needs cosmetics.manage.' });
    const kind = raw.match(/name="kind"\r\n\r\n([^\r]*)/)?.[1];
    if (kind !== 'badge' && kind !== 'paint') return json(res, 400, { error: 'bad_kind', message: 'kind is badge or paint.' });
    // Uploads reuse the sample star picture; the files are "made" two seconds later.
    const img = { ...cImage('sample-star', kind.toUpperCase()), id: `01JD8XIMG${String(100 + cosmeticSeq++).padStart(17, '0')}`, ready: false, files: [], createdAt: new Date().toISOString() };
    if (kind === 'paint') img.host = `${BASE}/cosmetics/badges/sample-star`;
    C_IMAGES.push(img);
    setTimeout(() => Object.assign(img, { ready: true, files: cImage('sample-star').files }), 2000);
    return json(res, 202, img);
  }
  const av = url.pathname.match(/^\/avatars\/(\d+)\.svg$/);
  if (av) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(avatar(Number(av[1])));
  }
  if (req.method === 'GET' && /^\/v1\/auth\/(twitch|kick)\/login$/.test(url.pathname)) {
    if (url.searchParams.get('link') === 'true' && !IDENTITIES.some((i) => i.platform === 'KICK') && url.pathname.includes('kick'))
      IDENTITIES.push({ platform: 'KICK', platformUserId: '4815162', login: 'lumen_fox', displayName: 'lumen_fox', avatar: null });
    const back = url.searchParams.get('return_to') ?? '/';
    res.writeHead(302, { location: SITE + (back.startsWith('/') ? back : '/'), 'set-cookie': 'mock_session=1; Path=/; HttpOnly; SameSite=Lax' });
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
    const e = emote(700 + uploads++, ME_SUMMARY, 'PENDING');
    e.name = name;
    e.host = '';
    e.files = [];
    e.createdAt = new Date().toISOString();
    MINE.unshift(e);
    // The worker "finishes" the files a few seconds later.
    setTimeout(() => Object.assign(e, { host: emote(700).host, files: emote(700).files }), 4000);
    return json(res, 202, e);
  }
  if (req.method === 'POST' && url.pathname === '/v1/gql') {
    const raw = (await read(req)).toString('utf8');
    // A malformed request (bad JSON, no query, a batch) gets an error, never a crashed mock.
    try {
      const body = JSON.parse(raw || '{}');
      if (typeof body?.query !== 'string') return json(res, 400, { data: null, errors: [{ message: 'Send one query.', extensions: { code: 'bad_request' } }] });
      return json(res, 200, answer(body, session, admin));
    } catch (e) {
      return json(res, 400, { data: null, errors: [{ message: String(e?.message ?? e), extensions: { code: 'bad_request' } }] });
    }
  }
  res.writeHead(404).end();
}).listen(PORT, process.env.MOCK_HOST ?? 'localhost', () => console.log(`mock api on ${BASE}${OPEN_ENV ? ' (open: everyone is staff)' : ADMIN_ENV ? ' (admin)' : ''}`));

function json(res, status, body) {
  res.writeHead(status, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
}
