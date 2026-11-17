// A stand-in for api.cyex.tv, for looking at the site without the real stack:
//   node dev/mock-api.mjs            (listens on localhost:8787)
//   PUBLIC_API_URL=http://localhost:8787 npm run dev
// It answers the site's GraphQL operations with made-up emotes and draws their pictures as
// little procedural faces (SVG). "Signing in" sets a mock cookie; everything signed in lives in memory and
// resets when it restarts. MOCK_ADMIN=1, or opening /mock/admin once, gives the mock account
// every permission (/mock/admin?off clears it).
// Development only; nothing here ships.
import { createServer } from 'node:http';

const PORT = Number(process.env.PORT ?? 8787);
const ORIGIN = process.env.SITE_ORIGIN ?? 'http://localhost:5173';
const BASE = `http://localhost:${PORT}`;
const ADMIN_ENV = process.env.MOCK_ADMIN === '1';

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
const ALL = Array.from({ length: 300 }, (_, i) => emote(i));
// The mock account's own uploads: a few of each state.
const MINE = Array.from({ length: 9 }, (_, i) =>
  emote(600 + i, ME_SUMMARY, i < 2 ? 'PENDING' : i === 2 ? 'REJECTED' : 'APPROVED'));
let uploads = 0;

const member = (e, name = e.name) => ({ name, zeroWidth: e.flags.zeroWidth, addedAt: e.createdAt, emote: e });
const SETS = [
  { id: '01JD8XSET00000000000000MAIN', name: 'Main', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: [...MINE.slice(3, 7), ...ALL.slice(10, 30)].map((e) => member(e)), editors: [OTHERS[0]] },
  { id: '01JD8XSET0000000000000ALTSS', name: 'Subathon', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: ALL.slice(40, 46).map((e) => member(e)), editors: [] },
  { id: '01JD8XSET000000000000000OAK', name: 'quietoak main', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[0],
    emotes: ALL.slice(60, 70).map((e) => member(e)), editors: [ME_SUMMARY] },
];
const GLOBAL = { id: '01JD8XSET0000000000000GLOBL', name: 'Global', kind: 'GLOBAL', capacity: 200, owner: null,
  emotes: ALL.slice(0, 24).map((e) => member(e)), editors: [] };
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
const PERMISSIONS = ['emotes.review', 'emotes.takedown', 'emotes.edit_any', 'sets.edit_any', 'global_set.edit', 'users.view', 'users.restrict', 'roles.manage', 'audit.view'];

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
const sorted = (hits, sort) => {
  if (sort === 'TOP') return [...hits].sort((a, b) => b.channelCount - a.channelCount || a.name.localeCompare(b.name));
  if (sort === 'NEW') return [...hits].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  return hits;
};

const findEmote = (id) => MINE.find((e) => e.id === id) ?? ALL.find((e) => e.id === id);
const findSet = (id) => SETS.find((s) => s.id === id);
const err = (message, code) => ({ data: null, errors: [{ message, extensions: { code } }] });
const setOut = (s) => ({ ...s, editors: undefined });

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
      return { data: { mySets: SETS.map(setOut) } };
    case 'MySetNames':
      return { data: { mySets: SETS.map((s) => ({ id: s.id, name: s.name, kind: s.kind, emotes: s.emotes.map((m) => ({ emote: { id: m.emote.id } })) })) } };
    case 'ReportEmote':
      if (!v.reason?.trim()) return err('Give a reason of 1 to 500 characters; the uploader sees it.', 'bad_reason');
      return { data: { reportEmote: { id: 'R' + Date.now() } } };
    case 'EditSet': {
      const s = findSet(v.id);
      if (!s) return { data: { set: null, setEditors: [] } };
      return { data: { set: setOut(s), setEditors: s.editors } };
    }
    case 'CreateSet': {
      if (v.input.kind === 'PERSONAL' && SETS.some((s) => s.kind === 'PERSONAL' && s.owner.id === ME_ID))
        return err('You have a personal set already.', 'personal_set_exists');
      const s = { id: `01JD8XSETNEW${String(SETS.length).padStart(14, '0')}`, name: v.input.name, kind: v.input.kind,
        capacity: v.input.kind === 'PERSONAL' ? 50 : 1000, owner: ME_SUMMARY, emotes: [], editors: [] };
      SETS.push(s);
      return { data: { createSet: { id: s.id } } };
    }
    case 'UpdateSet': {
      const s = findSet(v.id);
      if (v.input.name) s.name = v.input.name;
      return { data: { updateSet: { id: s.id, name: s.name } } };
    }
    case 'DeleteSet': {
      if (CHANNELS.some((c) => c.activeSetId === v.id)) return err('A channel’s active set can’t be deleted.', 'set_in_use');
      SETS.splice(SETS.findIndex((s) => s.id === v.id), 1);
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
    case 'MyChannels':
      return { data: { myChannels: CHANNELS, mySets: SETS.map((s) => ({ id: s.id, name: s.name, kind: s.kind, owner: s.owner ? { id: s.owner.id } : null })) } };
    case 'UpdateChannel': {
      const c = CHANNELS.find((x) => x.platform === v.platform && x.platformChannelId === v.id);
      if (v.input.activeSetId !== undefined) c.activeSetId = v.input.activeSetId || null;
      if (v.input.allowPersonalEmotes !== undefined) c.allowPersonalEmotes = v.input.allowPersonalEmotes;
      return { data: { updateChannelSettings: { activeSetId: c.activeSetId, allowPersonalEmotes: c.allowPersonalEmotes } } };
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

function answer(body, session, admin) {
  const q = body.query ?? '';
  const v = body.variables ?? {};
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
    if (v.login !== 'lumen_fox') return { data: { channelByLogin: null } };
    return { data: { channelByLogin: { ...CHANNELS[0], platform: v.platform } } };
  }
  if (q.includes('query Set(')) {
    const s = findSet(v.id) ?? SETS[0];
    return { data: { set: setOut(s) } };
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
  if (origin && LOCAL.test(origin)) return origin;
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
  const session = /(?:^|;\s*)mock_session=1/.test(req.headers.cookie ?? '');
  const url = new URL(req.url ?? '/', BASE);
  const admin = ADMIN_ENV || /(?:^|;\s*)mock_admin=1/.test(req.headers.cookie ?? '');
  if (url.pathname === '/mock/admin') {
    const off = url.searchParams.has('off');
    const cookie = off ? 'mock_admin=; Path=/; Max-Age=0' : 'mock_admin=1; Path=/; SameSite=Lax';
    res.writeHead(302, { location: SITE + '/admin', 'set-cookie': cookie });
    return res.end();
  }

  const img = url.pathname.match(/^\/emotes\/(\d+)\/(\d)x(_static)?\.webp$/);
  if (img) {
    res.writeHead(200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    return res.end(picture(Number(img[1]), Number(img[2]), Boolean(img[3])));
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
    return json(res, 200, answer(JSON.parse(raw || '{}'), session, admin));
  }
  res.writeHead(404).end();
}).listen(PORT, 'localhost', () => console.log(`mock api on ${BASE}${ADMIN_ENV ? ' (admin)' : ''}`));

function json(res, status, body) {
  res.writeHead(status, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
}
