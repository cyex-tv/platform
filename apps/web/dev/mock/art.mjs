// Mock API: art. See ../mock-api.mjs.


// ---- Emote pictures -------------------------------------------------------------------
// Original faces drawn from the emote's index: a head, eyes, a mouth and sometimes a hat, ears,
// headphones or a small extra. Everything is deterministic, so an emote always looks the same.
// Pictures are SVG, 32x32 (wide ones 64x32), transparent, with a darker outline of the same hue.

export function rngFor(i, salt = 0) {
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
export function spec(i) {
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
export const NAME_OF = new Map();
export function nameFor(i) {
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

export function picture(i, scale, still = false) {
  // The `_static` files are the first frame: the same drawing without its SMIL animation.
  const s = still ? { ...spec(i), animated: false } : spec(i);
  const W = s.wide ? 64 : 32;
  const body = s.wide ? scene(s) : s.zero ? overlay(s) : face(s);
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${W * scale}" height="${32 * scale}" viewBox="0 0 ${W} 32">${body}</svg>`;
}

// ---- Avatars: small original faces, patterns and scenes on a square (the site crops them round).
export function avatar(n) {
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
