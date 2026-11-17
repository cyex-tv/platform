// @ts-nocheck
/* Pack: Surfaces and textiles: stone, metal, fabric, paper. Same contracts as js/library.js; see js/packs/README.md. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, smoothstep, tri, wrap, hexToRgb, lutAt, hash2, vnoise, fbm, ridged, luma, hsvToRgb } = PL;
  const P = PL.P;
  const { rgb01, colorPrep, eachPx, bilinear, remap, boxBlur } = PL.fx;
  const add = (store, id, def) => { if (store[id]) throw new Error('Duplicate layer or effect id: ' + id); def.id = id; def.pack = 'surfaces'; store[id] = def; };
  const layer = (id, def) => add(PL.LAYERS, id, def);
  const effect = (id, def) => add(PL.EFFECTS, id, def);
  for (const c of ['Surfaces and textiles']) if (!PL.LAYER_CATS.includes(c)) PL.LAYER_CATS.push(c);
  for (const c of []) if (!PL.EFFECT_CATS.includes(c)) PL.EFFECT_CATS.push(c);

  // ---- definitions go here ----
  const CAT = 'Surfaces and textiles';
  const put = (o, r, g, b) => { o[0] = r < 0 ? 0 : r > 1 ? 1 : r; o[1] = g < 0 ? 0 : g > 1 ? 1 : g; o[2] = b < 0 ? 0 : b > 1 ? 1 : b; o[3] = 1; return o; };
  const mc = (o, a, b, k) => put(o, a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k);
  const ramp3 = (o, a, b, c, k) => (k < 0.5 ? mc(o, a, b, k * 2) : mc(o, b, c, (k - 0.5) * 2));
  // moving highlight band: x in cycles, w = width in cycles; returns 0..1
  const sheen = (x, w) => { const d = fract(x + 0.5) - 0.5; return Math.exp(-d * d / (w * w)); };
  const hsv = (o, h, s, v) => {
    h = fract(h) * 6; const i = Math.floor(h), f = h - i, p = v * (1 - s), q = v * (1 - s * f), r = v * (1 - s * (1 - f));
    if (i === 0) put(o, v, r, p); else if (i === 1) put(o, q, v, p); else if (i === 2) put(o, p, v, r);
    else if (i === 3) put(o, p, q, v); else if (i === 4) put(o, r, p, v); else put(o, v, p, q);
    return o;
  };
  const twinkle = (t, speed, h, pw) => Math.pow(0.5 + 0.5 * Math.cos(TAU * (t * speed + h)), pw);
  // cellular noise tiling every px cells across x. V = [f1, f2, dx, dy, id] (dx, dy: offset from the nearest point)
  const V = [0, 0, 0, 0, 0];
  const vor = (x, y, px, seed) => {
    const ix = Math.floor(x), iy = Math.floor(y);
    let f1 = 9, f2 = 9, dx = 0, dy = 0, id = 0;
    for (let j = -1; j <= 1; j++) for (let i = -1; i <= 1; i++) {
      const cx = ix + i, cy = iy + j, wx = wrap(cx, px);
      const ox = x - (cx + hash2(wx, cy, seed)), oy = y - (cy + hash2(wx, cy, seed + 7));
      const d = Math.sqrt(ox * ox + oy * oy);
      if (d < f1) { f2 = f1; f1 = d; dx = ox; dy = oy; id = hash2(wx, cy, seed + 13); } else if (d < f2) f2 = d;
    }
    V[0] = f1; V[1] = f2; V[2] = dx; V[3] = dy; V[4] = id;
    return V;
  };

  // ---- metals ----
  const crinkH = (x, y, p) => ridged(x, y, p.size, 64, 2, p.seed);
  layer('goldLeaf', {
    name: 'Gold leaf', cat: CAT, desc: 'Crinkled gold foil with torn patches and flecks that glint as the light turns.',
    tags: ['gold', 'foil', 'leaf', 'metal', 'shiny', 'luxury', 'crinkle', 'metallic'],
    params: [P.color('dark', 'Shadow', '#7a4a0c'), P.color('mid', 'Gold', '#d9a024'), P.color('light', 'Highlight', '#fff0a0'), P.range('crinkle', 'Crinkle', 0, 2, 0.05, 1), P.int('size', 'Crinkle size', 3, 24, 9), P.range('flecks', 'Flecks', 0, 1, 0.01, 0.5), P.speed(1), P.seed(5)],
    prepare: colorPrep(['dark', 'mid', 'light']),
    sample(u, v, t, p, o, c) {
      const x = u * p.size, y = v * p.size / c.A, e = 0.07, a = TAU * t * p.speed;
      const h0 = crinkH(x, y, p), gx = (crinkH(x + e, y, p) - h0) * 2, gy = (crinkH(x, y + e, p) - h0) * 2;
      const s = -(gx * Math.cos(a) + gy * Math.sin(a) * 0.7) * p.crinkle * 7;
      ramp3(o, p._dark, p._mid, p._light, clamp(0.5 + s * 0.5));
      const patch = smoothstep(0.3, 0.38, fbm(u * 5, v * 1.5, 5, 64, 2, p.seed + 9));
      const sp = Math.pow(clamp(s), 3) * 0.45;
      for (let i = 0; i < 3; i++) o[i] = clamp(mix(p._dark[i] * 0.7, o[i], patch) + sp * patch);
      const fh = hash2(Math.floor(x * 3), Math.floor(y * 3), p.seed + 3);
      if (fh > 1 - p.flecks * 0.12) { const f = twinkle(t, p.speed, fh * 37, 6) * 0.7; o[0] = clamp(o[0] + f); o[1] = clamp(o[1] + f); o[2] = clamp(o[2] + f * 0.8); }
      return o;
    },
  });
  layer('roseGold', {
    name: 'Rose gold', cat: CAT, desc: 'Polished pink metal with soft wavy reflections and a sheen that glides across.',
    tags: ['rose gold', 'pink', 'metal', 'shiny', 'luxury', 'polished', 'copper', 'metallic'],
    params: [P.color('dark', 'Shadow', '#8c4a43'), P.color('mid', 'Rose gold', '#e0a090'), P.color('light', 'Highlight', '#ffe6da'), P.int('bands', 'Reflections', 1, 8, 3), P.range('warp', 'Waviness', 0, 2, 0.05, 0.8), P.range('gloss', 'Gloss', 0, 1, 0.01, 0.6), P.speed(1), P.seed(12)],
    prepare: colorPrep(['dark', 'mid', 'light']),
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed;
      const w = fbm(u * 3 + Math.cos(a) * 0.3, v * 1.2 + Math.sin(a) * 0.3, 3, 64, 3, p.seed);
      const q = u * p.bands + v * 0.5 + (w - 0.5) * p.warp * 1.6;
      const k = Math.pow(0.5 + 0.5 * Math.sin(TAU * q), 1.4);
      ramp3(o, p._dark, p._mid, p._light, 0.15 + 0.75 * k);
      const s = sheen(u - t * p.speed + w * 0.2, 0.09) * p.gloss;
      o[0] = clamp(o[0] + s * 0.5); o[1] = clamp(o[1] + s * 0.45); o[2] = clamp(o[2] + s * 0.45);
      return o;
    },
  });
  layer('copperPatina', {
    name: 'Copper with patina', cat: CAT, desc: 'Warm copper with blotches of green verdigris and a drifting sheen on the metal.',
    tags: ['copper', 'verdigris', 'patina', 'green', 'aged', 'metal', 'bronze', 'oxidised'],
    params: [P.color('copper', 'Copper', '#b4602f'), P.color('copperLight', 'Copper highlight', '#f2a46c'), P.color('green', 'Patina', '#2f8f78'), P.color('greenLight', 'Patina light', '#9bdcc4'), P.range('amount', 'Patina', 0, 1, 0.01, 0.5), P.int('scale', 'Blotch size', 3, 12, 6), P.speed(1), P.seed(21)],
    prepare: colorPrep(['copper', 'copperLight', 'green', 'greenLight']),
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed, sc = p.scale;
      const n = fbm(u * sc + Math.cos(a) * 0.15, v * sc / c.A + Math.sin(a) * 0.15, sc, 64, 4, p.seed);
      const m = smoothstep(0.7 - p.amount * 0.4, 0.76 - p.amount * 0.4, n);
      const g = fbm(u * 60, v * 15, 60, 64, 2, p.seed + 4);
      const sh = sheen(u - t * p.speed + n * 0.2, 0.12);
      mc(o, p._copper, p._copperLight, clamp(0.25 + 0.35 * g + 0.55 * sh * (1 - m)));
      const gk = clamp(g * 1.2 - 0.1 + 0.2 * sh);
      for (let i = 0; i < 3; i++) o[i] = mix(o[i], mix(p._green[i], p._greenLight[i], gk), m);
      return o;
    },
  });
  layer('rustedMetal', {
    name: 'Rusted metal', cat: CAT, desc: 'Weathered steel eaten by orange rust, with pitting and a faint gleam on the clean parts.',
    tags: ['rust', 'iron', 'steel', 'metal', 'grunge', 'old', 'weathered', 'orange'],
    params: [P.color('steel', 'Steel', '#7b8087'), P.color('rust', 'Rust', '#8f3c16'), P.color('rustLight', 'Rust light', '#d27a2e'), P.range('amount', 'Rust', 0, 1, 0.01, 0.55), P.int('scale', 'Patch size', 3, 12, 6), P.speed(1), P.seed(33)],
    prepare: colorPrep(['steel', 'rust', 'rustLight']),
    sample(u, v, t, p, o, c) {
      const sc = p.scale;
      const n = fbm(u * sc, v * sc / c.A, sc, 64, 4, p.seed);
      const m = smoothstep(0.72 - p.amount * 0.4, 0.78 - p.amount * 0.4, n);
      const n2 = fbm(u * sc * 4, v * sc * 4 / c.A, sc * 4, 64, 3, p.seed + 2);
      const gr = vnoise(u * 16, v * 48, 16, 64, p.seed + 5);
      const sk = clamp(0.5 + (gr - 0.5) * 0.5 + sheen(u - t * p.speed, 0.12) * 0.35);
      const rk = clamp(n2 * 1.3 - 0.1);
      const pit = hash2(Math.floor(u * c.W / 2), Math.floor(v * c.H / 2), p.seed) < 0.06 ? 0.55 : 1;
      for (let i = 0; i < 3; i++) o[i] = mix(mix(p._steel[i] * 0.55, p._steel[i] * 1.25, sk), mix(p._rust[i], p._rustLight[i], rk) * pit, m);
      return put(o, o[0], o[1], o[2]);
    },
  });
  layer('hammeredMetal', {
    name: 'Hammered metal', cat: CAT, desc: 'Beaten metal full of small dimples that catch a light circling overhead.',
    tags: ['hammered', 'metal', 'dimples', 'beaten', 'silver', 'shiny', 'copper', 'dents'],
    params: [P.color('color', 'Metal', '#b9c0c8'), P.int('n', 'Dimples across', 6, 40, 18), P.range('depth', 'Depth', 0, 1, 0.01, 0.65), P.speed(1), P.seed(44)],
    prepare: colorPrep(['color']),
    sample(u, v, t, p, o, c) {
      vor(u * p.n, v * p.n / c.A, p.n, p.seed);
      const a = TAU * t * p.speed, lx = Math.cos(a), ly = Math.sin(a);
      const lit = clamp(0.55 - (V[2] * lx + V[3] * ly) * p.depth * 1.8);
      const crease = smoothstep(0, 0.14, V[1] - V[0]);
      const k = (0.3 + 0.7 * lit) * (0.7 + 0.3 * crease) * (0.92 + 0.08 * V[4]) * 1.1;
      const sp = Math.pow(clamp(-(V[2] * lx + V[3] * ly) * 2.4), 4) * 0.55 * p.depth;
      return put(o, p._color[0] * k + sp, p._color[1] * k + sp, p._color[2] * k + sp);
    },
  });
  layer('damascus', {
    name: 'Damascus steel', cat: CAT, desc: 'Etched pattern-welded steel with flowing ripples and a sliding highlight.',
    tags: ['damascus', 'steel', 'blade', 'ripple', 'metal', 'knife', 'forged', 'grey'],
    params: [P.color('dark', 'Dark steel', '#262a31'), P.color('light', 'Bright steel', '#cfd4dc'), P.int('lines', 'Layers', 4, 40, 16), P.range('warp', 'Ripple', 0, 3, 0.05, 1.2), P.speed(1), P.seed(55)],
    prepare: colorPrep(['dark', 'light']),
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed;
      const w = fbm(u * 4 + Math.cos(a) * 0.25, v * 3 + Math.sin(a) * 0.25, 4, 64, 4, p.seed);
      const q = v * p.lines * 0.5 + (w - 0.5) * p.warp * 5 + Math.sin(TAU * u * 2) * 0.3;
      const k = smoothstep(0.25, 0.75, tri(q)) * 0.8 + 0.2 * tri(q * 3 + w * 4);
      const sh = sheen(u - t * p.speed + w * 0.25, 0.14) * 0.4 * (0.4 + 0.6 * k);
      mc(o, p._dark, p._light, clamp(k * 0.85));
      return put(o, o[0] + sh, o[1] + sh, o[2] + sh);
    },
  });
  layer('titanium', {
    name: 'Anodised titanium', cat: CAT, desc: 'Brushed metal tinted with oxide rainbow colour that slides slowly along the surface.',
    tags: ['titanium', 'anodised', 'rainbow', 'iridescent', 'metal', 'oil slick', 'brushed', 'colourful'],
    params: [P.range('hue', 'Base hue', 0, 1, 0.01, 0.62), P.range('range', 'Colour range', 0, 1, 0.01, 0.4), P.range('sat', 'Saturation', 0, 1, 0.01, 0.7), P.int('scale', 'Patch size', 2, 12, 4), P.speed(1), P.seed(66)],
    sample(u, v, t, p, o, c) {
      const n = fbm(u * p.scale, v * 1.5, p.scale, 64, 3, p.seed);
      const grain = vnoise(u * 8, v * 70, 8, 64, p.seed + 1);
      const sh = sheen(u - t * p.speed, 0.2);
      const h = p.hue + (n - 0.5) * p.range * 2 + 0.5 * p.range * Math.sin(TAU * (u - t * p.speed));
      return hsv(o, h, p.sat, clamp(0.5 + 0.3 * grain + 0.25 * sh));
    },
  });

  // ---- pearl and stone ----
  layer('nacre', {
    name: 'Mother of pearl', cat: CAT, desc: 'Layered shell with soft pastel iridescence that shifts as it flows.',
    tags: ['mother of pearl', 'nacre', 'shell', 'pearl', 'iridescent', 'pastel', 'lustre', 'abalone'],
    params: [P.range('flow', 'Layering', 0.5, 4, 0.05, 2), P.range('irid', 'Iridescence', 0, 2, 0.05, 1), P.range('shift', 'Hue shift', 0, 1, 0.01, 0), P.speed(1), P.seed(77)],
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed;
      const q = fbm(u * 5 + Math.cos(a) * 0.4, v * 2.5 + Math.sin(a) * 0.4, 5, 64, 4, p.seed);
      const ph = q * p.flow + p.shift;
      const m = 0.94 + 0.03 * Math.sin(TAU * q * 9) + sheen(u - t * p.speed + q * 0.3, 0.16) * 0.18;
      return put(o, (0.86 + 0.14 * p.irid * Math.cos(TAU * ph)) * m, (0.84 + 0.16 * p.irid * Math.cos(TAU * (ph + 0.33))) * m, (0.9 + 0.1 * p.irid * Math.cos(TAU * (ph + 0.66))) * m);
    },
  });
  layer('granite', {
    name: 'Granite', cat: CAT, desc: 'Speckled stone in black, pink and grey, with mica that glints now and then.',
    tags: ['granite', 'stone', 'speckle', 'rock', 'counter', 'grey', 'mica', 'polished'],
    params: [P.color('base', 'Stone', '#d3cdc6'), P.color('dark', 'Black flecks', '#25252a'), P.color('pink', 'Pink flecks', '#a98782'), P.int('scale', 'Fleck density', 16, 100, 48), P.range('speckle', 'Speckle', 0, 1, 0.01, 0.7), P.speed(1), P.seed(88)],
    prepare: colorPrep(['base', 'dark', 'pink']),
    sample(u, v, t, p, o, c) {
      const cs = Math.max(2, Math.round(p.scale / 6));
      const n = fbm(u * cs, v * cs / c.A, cs, 64, 3, p.seed);
      const x = u * p.scale, y = v * p.scale / c.A, i = Math.floor(x), j = Math.floor(y);
      const h = hash2(i, j, p.seed), k = p.speckle * (0.5 + n);
      const sh = sheen(u - t * p.speed, 0.25) * 0.07;
      let col = p._base, f = 0.92 + 0.12 * n;
      if (h < 0.2 * k) { col = p._dark; f = 1; } else if (h < 0.4 * k) { col = p._pink; f = 1; } else if (h < 0.55 * k) f = 0.7;
      put(o, col[0] * f + sh, col[1] * f + sh, col[2] * f + sh);
      const g = hash2(i, j, p.seed + 5);
      if (g > 0.985) { const w = twinkle(t, p.speed, g * 100, 8) * 0.8; o[0] = clamp(o[0] + w); o[1] = clamp(o[1] + w); o[2] = clamp(o[2] + w); }
      return o;
    },
  });
  layer('terrazzo', {
    name: 'Terrazzo', cat: CAT, desc: 'Coloured stone chips set in pale cement with a polished shine that sweeps across.',
    tags: ['terrazzo', 'chips', 'stone', 'floor', 'speckle', 'confetti', 'cement', 'pastel'],
    params: [P.pal('pastel', true), P.color('cement', 'Cement', '#dcd7ce'), P.int('size', 'Chips across', 3, 14, 8), P.range('density', 'Density', 0.1, 1, 0.01, 0.7), P.range('shine', 'Polish', 0, 1, 0.01, 0.4), P.speed(1), P.seed(99)],
    prepare: colorPrep(['cement']),
    sample(u, v, t, p, o, c) {
      const gr = 0.96 + 0.08 * hash2(Math.floor(u * c.W), Math.floor(v * c.H), p.seed);
      let r = p._cement[0] * gr, g = p._cement[1] * gr, b = p._cement[2] * gr;
      for (let pass = 0; pass < 2; pass++) {
        const n = p.size * (1 + pass);
        vor(u * n, v * n / c.A, n, p.seed + pass * 11);
        if (V[4] > p.density) continue;
        const wob = (vnoise(u * n * 3, v * n * 3 / c.A, n * 3, 256, p.seed + 3) - 0.5) * 0.35;
        const rad = (0.2 + 0.25 * fract(V[4] * 13.7)) * (pass ? 0.7 : 1);
        if (V[0] + wob < rad) {
          lutAt(p._stops, fract(V[4] * 7.31 + pass * 0.2), o);
          const f = 0.9 + 0.2 * fract(V[4] * 91.7);
          r = o[0] * f; g = o[1] * f; b = o[2] * f;
        }
      }
      const s = sheen(u + v * 0.2 - t * p.speed, 0.12) * p.shine * 0.35;
      return put(o, r + s, g + s, b + s);
    },
  });
  layer('sandstone', {
    name: 'Sandstone strata', cat: CAT, desc: 'Wavy layers of warm sand and ochre with gritty grain and a soft passing glow.',
    tags: ['sandstone', 'strata', 'layers', 'rock', 'desert', 'canyon', 'sand', 'earth'],
    params: [P.color('light', 'Pale sand', '#ecd0a4'), P.color('mid', 'Ochre', '#c9945c'), P.color('dark', 'Rust band', '#8f5430'), P.int('bands', 'Layers', 2, 20, 7), P.range('warp', 'Waviness', 0, 1, 0.01, 0.35), P.range('grain', 'Grain', 0, 1, 0.01, 0.5), P.speed(1), P.seed(111)],
    prepare: colorPrep(['light', 'mid', 'dark']),
    sample(u, v, t, p, o, c) {
      const w = fbm(u * 3, v * 2, 3, 64, 3, p.seed);
      const q = v * p.bands + (w - 0.5) * p.warp * p.bands * 0.8;
      const L = Math.floor(q), fq = q - L;
      let tone = mix(hash2(L, 0, p.seed), hash2(L + 1, 0, p.seed), smoothstep(0.8, 1, fq));
      tone = tone * 0.85 + 0.15 * smoothstep(0, 0.1, fq);
      const g = (hash2(Math.floor(u * c.W), Math.floor(v * c.H), p.seed + 1) - 0.5) * p.grain * 0.3;
      ramp3(o, p._dark, p._mid, p._light, clamp(tone + g));
      const s = sheen(u - t * p.speed, 0.3) * 0.09;
      return put(o, o[0] + s, o[1] + s * 0.9, o[2] + s * 0.7);
    },
  });
  layer('obsidian', {
    name: 'Obsidian', cat: CAT, desc: 'Black volcanic glass with faint flow lines and a glassy reflection gliding over it.',
    tags: ['obsidian', 'glass', 'black', 'volcanic', 'dark', 'glossy', 'stone', 'mirror'],
    params: [P.color('base', 'Glass', '#0b0c12'), P.color('tint', 'Sheen tint', '#7d8bff'), P.range('gloss', 'Gloss', 0, 1, 0.01, 0.8), P.speed(1), P.seed(122)],
    prepare: colorPrep(['base', 'tint']),
    sample(u, v, t, p, o, c) {
      const f = ridged(u * 3, v * 2, 3, 64, 3, p.seed);
      const b = 0.6 + 0.9 * f;
      const rf = sheen(u + v * 0.25 + f * 0.15 - t * p.speed, 0.07) * p.gloss;
      const rf2 = sheen(u - v * 0.2 + f * 0.3 - t * p.speed + 0.5, 0.2) * 0.2 * p.gloss;
      const k = rf * 0.8 + rf2, w = Math.pow(rf, 3) * 0.45;
      let r = p._base[0] * b + p._tint[0] * k + w, g = p._base[1] * b + p._tint[1] * k + w, bl = p._base[2] * b + p._tint[2] * k + w;
      const h = hash2(Math.floor(u * c.W / 2), Math.floor(v * c.H / 2), p.seed);
      if (h > 0.992) { const s = twinkle(t, p.speed, h * 400, 6) * 0.5; r += s; g += s; bl += s; }
      return put(o, r, g, bl);
    },
  });
  layer('malachite', {
    name: 'Malachite', cat: CAT, desc: 'Banded green stone with swirling concentric rings that creep slowly outward.',
    tags: ['malachite', 'green', 'stone', 'gem', 'bands', 'rings', 'swirl', 'mineral'],
    params: [P.color('dark', 'Deep green', '#04301c'), P.color('mid', 'Green', '#17895a'), P.color('light', 'Light green', '#7fe2ae'), P.range('count', 'Bands', 2, 20, 0.5, 8), P.int('scale', 'Swirl size', 2, 10, 4), P.speed(1), P.seed(133)],
    prepare: colorPrep(['dark', 'mid', 'light']),
    sample(u, v, t, p, o, c) {
      const q = fbm(u * p.scale, v * p.scale / c.A, p.scale, 64, 4, p.seed);
      const s = tri(q * p.count - t * p.speed);
      const line = smoothstep(0, 0.1, s);
      const k = smoothstep(0.05, 0.95, s) * 0.85 + 0.1;
      ramp3(o, p._dark, p._mid, p._light, k * (0.4 + 0.6 * line));
      const sh = sheen(u - t * p.speed + q * 0.5, 0.18) * 0.12;
      return put(o, o[0] + sh, o[1] + sh, o[2] + sh);
    },
  });
  layer('geode', {
    name: 'Amethyst geode', cat: CAT, desc: 'A cluster of purple crystal facets that flash one after another as they turn.',
    tags: ['amethyst', 'geode', 'crystal', 'purple', 'gem', 'quartz', 'sparkle', 'facets'],
    params: [P.color('dark', 'Deep', '#25093f'), P.color('mid', 'Amethyst', '#7a3cc2'), P.color('light', 'Light', '#e4c8ff'), P.int('n', 'Crystals across', 6, 30, 16), P.range('flash', 'Flash', 0, 1, 0.01, 0.7), P.speed(1), P.seed(144)],
    prepare: colorPrep(['dark', 'mid', 'light']),
    sample(u, v, t, p, o, c) {
      vor(u * p.n, v * p.n / c.A, p.n, p.seed);
      const id = V[4], crease = smoothstep(0, 0.12, V[1] - V[0]);
      const br = 0.5 + 0.5 * Math.cos(TAU * (id + t * p.speed));
      const k = 0.2 + 0.4 * (0.5 + 0.5 * Math.cos(TAU * id * 3)) + 0.3 * br * p.flash;
      ramp3(o, p._dark, p._mid, p._light, clamp(k * (0.55 + 0.45 * crease) + (1 - crease) * 0.1));
      const w = Math.pow(br, 14) * p.flash * 0.6 * crease;
      const g = hash2(Math.floor(u * c.W), Math.floor(v * c.H), p.seed + 2);
      const d = g > 0.97 ? twinkle(t, p.speed, g * 90, 4) * 0.35 : 0;
      return put(o, o[0] + w + d, o[1] + w + d, o[2] + w + d);
    },
  });

  // ---- cloth, leather ----
  layer('velvetPile', {
    name: 'Velvet', cat: CAT, desc: 'Deep plush pile with soft folds and a sheen that brightens where the fabric bends.',
    tags: ['velvet', 'plush', 'fabric', 'cloth', 'soft', 'luxury', 'pile', 'curtain'],
    params: [P.color('base', 'Velvet', '#5f0d2b'), P.color('sheenCol', 'Sheen', '#ff6f96'), P.range('folds', 'Fold depth', 0, 1, 0.01, 0.6), P.range('sheen', 'Sheen', 0, 1, 0.01, 0.7), P.speed(1), P.seed(155)],
    prepare: colorPrep(['base', 'sheenCol']),
    sample(u, v, t, p, o, c) {
      const fold = fbm(u * 2, v, 2, 64, 3, p.seed);
      const pile = vnoise(u * 96, v * 30, 96, 64, p.seed + 1);
      const grazing = Math.abs(fold - 0.5) * 2;
      const lk = sheen(u - t * p.speed + fold * 0.6, 0.2) * (0.35 + 0.65 * grazing) * p.sheen;
      const f = (1 - p.folds * 0.6 * (1 - fold) + 0.2 * (fold - 0.5)) * (0.88 + 0.24 * pile);
      return put(o, mix(p._base[0] * f, p._sheenCol[0], lk), mix(p._base[1] * f, p._sheenCol[1], lk), mix(p._base[2] * f, p._sheenCol[2], lk));
    },
  });
  layer('satinSheen', {
    name: 'Satin', cat: CAT, desc: 'Smooth draped satin with broad glossy folds and a bright highlight that rolls along.',
    tags: ['satin', 'silk', 'fabric', 'smooth', 'glossy', 'drape', 'luxury', 'cloth'],
    params: [P.color('dark', 'Shadow', '#4a0f4d'), P.color('mid', 'Satin', '#b23a9a'), P.color('light', 'Highlight', '#ffd3f1'), P.int('folds', 'Folds', 1, 6, 2), P.range('warp', 'Drape', 0, 2, 0.05, 0.8), P.speed(1), P.seed(166)],
    prepare: colorPrep(['dark', 'mid', 'light']),
    sample(u, v, t, p, o, c) {
      const w = fbm(u * 2, v, 2, 64, 3, p.seed);
      const q = u * p.folds + v * 0.6 + (w - 0.5) * p.warp * 1.5 - t * p.speed;
      const s = 0.5 + 0.5 * Math.sin(TAU * q);
      ramp3(o, p._dark, p._mid, p._light, 0.12 + 0.7 * Math.pow(s, 1.6));
      const sp = Math.pow(s, 22) * 0.55;
      return put(o, o[0] + sp, o[1] + sp, o[2] + sp);
    },
  });
  layer('denimTwill', {
    name: 'Denim', cat: CAT, desc: 'Blue twill weave with diagonal ribs, faded patches and a soft passing sheen.',
    tags: ['denim', 'jeans', 'twill', 'fabric', 'blue', 'cloth', 'weave', 'faded'],
    params: [P.color('warp', 'Indigo thread', '#1f3b73'), P.color('weft', 'Weft thread', '#9bb0cf'), P.int('n', 'Threads across', 24, 120, 64), P.range('wear', 'Fading', 0, 1, 0.01, 0.4), P.speed(1), P.seed(177)],
    prepare: colorPrep(['warp', 'weft']),
    sample(u, v, t, p, o, c) {
      const x = u * p.n, y = v * p.n / c.A, i = Math.floor(x), j = Math.floor(y);
      const fx = x - i, fy = y - j;
      const hh = 0.92 + 0.16 * hash2(wrap(i, p.n), j, p.seed);
      let r, g, b;
      if (wrap(i - j, 4) !== 0) { const s = (0.55 + 0.55 * Math.sin(Math.PI * fx)) * hh; r = p._warp[0] * s; g = p._warp[1] * s; b = p._warp[2] * s; }
      else { const s = (0.55 + 0.55 * Math.sin(Math.PI * fy)) * hh; r = p._weft[0] * s; g = p._weft[1] * s; b = p._weft[2] * s; }
      const fade = smoothstep(0.45, 0.75, fbm(u * 4, v * 2, 4, 64, 3, p.seed + 3)) * p.wear * 0.55;
      const s2 = sheen(u + v * 0.3 - t * p.speed, 0.25) * 0.08;
      return put(o, mix(r, 0.75, fade) + s2, mix(g, 0.82, fade) + s2, mix(b, 0.92, fade) + s2);
    },
  });
  layer('knitStitch', {
    name: 'Knitted stitches', cat: CAT, desc: 'Rows of chunky V-shaped knit stitches with a gentle wave of light across the yarn.',
    tags: ['knit', 'wool', 'stitch', 'yarn', 'sweater', 'fabric', 'cosy', 'jumper'],
    params: [P.color('yarn', 'Yarn', '#c8394b'), P.color('shadow', 'Shadow', '#3a0c14'), P.int('n', 'Stitches across', 12, 64, 32), P.range('fuzz', 'Fuzz', 0, 1, 0.01, 0.4), P.speed(1), P.seed(188)],
    prepare: colorPrep(['yarn', 'shadow']),
    sample(u, v, t, p, o, c) {
      const x = u * p.n, y = v * p.n / c.A * 1.3, i = Math.floor(x), j = Math.floor(y);
      const fx = x - i, fy = y - j;
      const dl = Math.abs(fx - 0.5 * fy - 0.05), dr = Math.abs(fx - (1 - 0.5 * fy) + 0.05);
      const d = Math.min(dl, dr), th = 0.3;
      const strand = 1 - smoothstep(th * 0.6, th, d);
      const round = Math.sqrt(clamp(1 - (d / th) * (d / th)));
      const twist = 0.9 + 0.1 * Math.sin(TAU * (fy * 3 + (dl < dr ? fx : -fx) * 2));
      const wv = 0.82 + 0.18 * Math.cos(TAU * (u - t * p.speed));
      const fz = 1 + (hash2(Math.floor(u * c.W), Math.floor(v * c.H), p.seed) - 0.5) * p.fuzz * 0.3;
      const sh = sheen(u - t * p.speed, 0.15) * 0.18;
      const m = strand * clamp((0.45 + 0.55 * round) * twist * wv * fz * (dl < dr ? 1 : 0.92) + sh);
      return put(o, mix(p._shadow[0], p._yarn[0] * 1.1, m), mix(p._shadow[1], p._yarn[1] * 1.1, m), mix(p._shadow[2], p._yarn[2] * 1.1, m));
    },
  });
  layer('leatherGrain', {
    name: 'Leather', cat: CAT, desc: 'Pebbled leather grain with fine creases and a light that rolls slowly over the bumps.',
    tags: ['leather', 'grain', 'pebbled', 'brown', 'hide', 'texture', 'skin', 'bag'],
    params: [P.color('color', 'Leather', '#6e3b1e'), P.color('light', 'Highlight', '#d79563'), P.int('n', 'Grain size', 8, 60, 28), P.range('crease', 'Creases', 0, 1, 0.01, 0.7), P.speed(1), P.seed(199)],
    prepare: colorPrep(['color', 'light']),
    sample(u, v, t, p, o, c) {
      vor(u * p.n, v * p.n / c.A, p.n, p.seed);
      const a = TAU * t * p.speed, lx = Math.cos(a), ly = Math.sin(a) * 0.6;
      const cr = smoothstep(0, 0.2, V[1] - V[0]);
      const lit = clamp(0.45 - (V[2] * lx + V[3] * ly) * 1.1);
      const k = (1 - p.crease * 0.7 * (1 - cr)) * (0.35 + 0.65 * lit) * (0.9 + 0.1 * V[4]);
      mc(o, p._color, p._light, clamp(k * 0.9 - 0.1 + sheen(u - t * p.speed, 0.2) * 0.2 * cr));
      const f = 0.6 + 0.4 * cr;
      return put(o, o[0] * f, o[1] * f, o[2] * f);
    },
  });
  layer('linenWeave', {
    name: 'Linen', cat: CAT, desc: 'Plain-weave flax cloth with uneven slubby threads and a faint moving shimmer.',
    tags: ['linen', 'weave', 'canvas', 'fabric', 'flax', 'natural', 'cloth', 'beige'],
    params: [P.color('light', 'Flax', '#e6d9b8'), P.color('dark', 'Shadow', '#8f7c58'), P.int('n', 'Threads across', 24, 120, 64), P.range('slub', 'Slubs', 0, 1, 0.01, 0.6), P.speed(1), P.seed(210)],
    prepare: colorPrep(['light', 'dark']),
    sample(u, v, t, p, o, c) {
      const x = u * p.n, y = v * p.n / c.A, i = Math.floor(x), j = Math.floor(y);
      const q = Math.max(1, Math.round(p.n / 4));
      let s, sl;
      if (((i + j) & 1) === 0) { s = Math.sin(Math.PI * (y - j)); sl = vnoise(u * q, j * 0.9 + 3, q, 256, p.seed); }
      else { s = Math.sin(Math.PI * (x - i)); sl = vnoise(wrap(i, p.n) * 0.9 + 9, y * 0.25, 256, 256, p.seed + 1); }
      mc(o, p._dark, p._light, clamp((0.35 + 0.65 * s) * (1 - p.slub * 0.5 + p.slub * sl)));
      const sh = sheen(u + v * 0.3 - t * p.speed, 0.22) * 0.08;
      return put(o, o[0] + sh, o[1] + sh, o[2] + sh);
    },
  });
  layer('sequins', {
    name: 'Sequins', cat: CAT, desc: 'A grid of round sequins, each catching the light at its own moment.',
    tags: ['sequins', 'sparkle', 'disco', 'glitter', 'party', 'sequin', 'shiny', 'fabric'],
    params: [P.color('a', 'Sequin', '#6a2bd0'), P.color('b', 'Sequin 2', '#e0449a'), P.range('mixAmt', 'Colour mix', 0, 1, 0.01, 0.5), P.int('n', 'Sequins across', 12, 80, 40), P.speed(1), P.seed(221)],
    prepare: colorPrep(['a', 'b']),
    sample(u, v, t, p, o, c) {
      const x = u * p.n, y = v * p.n / c.A / 0.866, j = Math.floor(y);
      const xo = x + 0.5 * (j & 1), i = Math.floor(xo);
      const fx = xo - i - 0.5, fy = y - j - 0.5;
      const id = hash2(wrap(i, p.n), j, p.seed);
      const r = Math.sqrt(fx * fx + fy * fy);
      if (r > 0.48) return put(o, 0.04, 0.02, 0.07);
      const fl = twinkle(t, p.speed, id, 5), mm = id * p.mixAmt;
      const hx = 0.2 * Math.cos(TAU * id * 7), hy = 0.2 * Math.sin(TAU * id * 7);
      const hl = Math.exp(-((fx - hx) * (fx - hx) + (fy - hy) * (fy - hy)) / 0.04) * fl * 0.9;
      const f = (0.55 + 0.45 * (1 - r / 0.48) - 0.25 * smoothstep(0.38, 0.48, r)) * (0.75 + 0.5 * fl);
      return put(o, mix(p._a[0], p._b[0], mm) * f + hl, mix(p._a[1], p._b[1], mm) * f + hl, mix(p._a[2], p._b[2], mm) * f + hl);
    },
  });
  layer('glitterVinyl', {
    name: 'Glitter vinyl', cat: CAT, desc: 'Glossy coloured base packed with tiny flakes that twinkle out of step.',
    tags: ['glitter', 'vinyl', 'sparkle', 'flakes', 'shiny', 'party', 'twinkle', 'pink'],
    params: [P.color('base', 'Base', '#d81b84'), P.int('n', 'Flake grid', 48, 220, 110), P.range('density', 'Density', 0, 1, 0.01, 0.55), P.range('rainbow', 'Rainbow flakes', 0, 1, 0.01, 0.3), P.speed(2), P.seed(232)],
    prepare: colorPrep(['base']),
    sample(u, v, t, p, o, c) {
      const x = u * p.n, y = v * p.n / c.A, i = Math.floor(x), j = Math.floor(y);
      const h = hash2(i, j, p.seed), h2 = hash2(i, j, p.seed + 7);
      const gr = fbm(u * 12, v * 4, 12, 64, 2, p.seed + 3);
      const sh = sheen(u - t, 0.18) * 0.18;
      const b = 0.55 + 0.4 * gr + sh;
      let r = p._base[0] * b, g = p._base[1] * b, bl = p._base[2] * b;
      if (h2 < p.density) {
        const f = twinkle(t, p.speed, h * 19, 9);
        hsv(o, h * 3, 0.55, 1);
        r += mix(1, o[0], p.rainbow) * f; g += mix(1, o[1], p.rainbow) * f; bl += mix(1, o[2], p.rainbow) * f;
      }
      return put(o, r, g, bl);
    },
  });

  // ---- animal prints and camo ----
  layer('leopardPrint', {
    name: 'Leopard print', cat: CAT, desc: 'Tawny fur with broken dark rosettes and a soft sheen rolling over the coat.',
    tags: ['leopard', 'animal print', 'spots', 'rosettes', 'fur', 'cheetah', 'jaguar', 'wild'],
    params: [P.color('ground', 'Fur', '#dba646'), P.color('centre', 'Rosette centre', '#b9772a'), P.color('spot', 'Spots', '#241509'), P.int('n', 'Rosettes across', 4, 16, 8), P.speed(1), P.seed(243)],
    prepare: colorPrep(['ground', 'centre', 'spot']),
    sample(u, v, t, p, o, c) {
      const n = p.n;
      vor(u * n, v * n / c.A, n, p.seed);
      const id = V[4], dx = V[2], dy = V[3];
      const wob = (vnoise(u * n * 4, v * n * 4 / c.A, n * 4, 256, p.seed + 3) - 0.5) * 0.22;
      const rr = V[0] + wob;
      const r0 = 0.24 + 0.1 * fract(id * 7.7), ri = r0 * 0.55;
      let spot = 0, cen = 0;
      if (id < 0.2) spot = 1 - smoothstep(0.08, 0.12, rr);
      else {
        const gap = smoothstep(0.32, 0.5, vnoise(dx * 2.5 + id * 31, dy * 2.5 + id * 17, 256, 256, p.seed + 9));
        spot = smoothstep(ri - 0.02, ri + 0.02, rr) * (1 - smoothstep(r0 - 0.02, r0 + 0.02, rr)) * gap;
        cen = 1 - smoothstep(ri - 0.02, ri + 0.02, rr);
      }
      const f = 1 + (hash2(Math.floor(u * c.W), Math.floor(v * c.H), p.seed + 1) - 0.5) * 0.12 + sheen(u - t * p.speed + wob, 0.25) * 0.12;
      const k = clamp(spot), kc = cen * (1 - k);
      return put(o, mix(mix(p._ground[0], p._centre[0], kc), p._spot[0], k) * f, mix(mix(p._ground[1], p._centre[1], kc), p._spot[1], k) * f, mix(mix(p._ground[2], p._centre[2], kc), p._spot[2], k) * f);
    },
  });
  layer('zebraPrint', {
    name: 'Zebra stripes', cat: CAT, desc: 'Bold black and white stripes that wander, fork and ripple slowly.',
    tags: ['zebra', 'stripes', 'animal print', 'black and white', 'monochrome', 'wild', 'fur', 'safari'],
    params: [P.color('light', 'Light', '#f3efe6'), P.color('dark', 'Dark', '#0d0d10'), P.int('stripes', 'Stripes', 3, 20, 9), P.range('warp', 'Wander', 0, 2, 0.05, 0.8), P.range('thick', 'Stripe weight', 0.1, 0.9, 0.01, 0.5), P.speed(1), P.seed(254)],
    prepare: colorPrep(['light', 'dark']),
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed;
      const w = fbm(u * 4 + Math.cos(a) * 0.4, v * 3 + Math.sin(a) * 0.4, 4, 64, 3, p.seed);
      const w2 = fbm(u * 6, v * 4, 6, 64, 3, p.seed + 5);
      const q = u * p.stripes + v * 0.5 + (w - 0.5) * p.warp * 2;
      const th = p.thick + (w2 - 0.5) * 0.5;
      const s = smoothstep(th - 0.05, th + 0.05, tri(q));
      const d = (hash2(Math.floor(u * c.W), Math.floor(v * c.H), p.seed) - 0.5) * 0.08 + sheen(u - t * p.speed, 0.2) * 0.1 * (0.4 + s);
      mc(o, p._dark, p._light, s);
      return put(o, o[0] + d, o[1] + d, o[2] + d);
    },
  });
  layer('camoPattern', {
    name: 'Camouflage', cat: CAT, desc: 'Woodland blotches or digital pixel camo in four colours that slowly morph.',
    tags: ['camo', 'camouflage', 'army', 'military', 'woodland', 'pixel', 'digital', 'green'],
    params: [P.color('ground', 'Ground', '#5d6b3b'), P.color('c2', 'Dark green', '#3a4a29'), P.color('c3', 'Brown', '#6e5530'), P.color('c4', 'Black', '#1c2014'), P.select('style', 'Style', [['woodland', 'Woodland'], ['pixel', 'Digital pixels']], 'woodland'), P.int('scale', 'Blotch size', 3, 12, 5), P.speed(1), P.seed(265)],
    prepare: colorPrep(['ground', 'c2', 'c3', 'c4']),
    sample(u, v, t, p, o, c) {
      const a = TAU * t * p.speed, sc = p.scale;
      let uu = u, vv = v, e = 0.04;
      if (p.style === 'pixel') { const rows = Math.max(1, Math.round(48 / c.A)); uu = (Math.floor(u * 48) + 0.5) / 48; vv = (Math.floor(v * rows) + 0.5) / rows; e = 0.004; }
      const x = uu * sc + Math.cos(a) * 0.5, y = vv * sc / c.A + Math.sin(a) * 0.5;
      const m1 = smoothstep(0.5 - e, 0.5 + e, fbm(x, y, sc, 64, 3, p.seed));
      const m2 = smoothstep(0.58 - e, 0.58 + e, fbm(x + 3, y * 1.3, sc, 64, 3, p.seed + 5));
      const m3 = smoothstep(0.64 - e, 0.64 + e, fbm(x * 2, y * 2, sc * 2, 64, 3, p.seed + 9));
      for (let i = 0; i < 3; i++) o[i] = mix(mix(mix(p._ground[i], p._c2[i], m1), p._c3[i], m2), p._c4[i], m3);
      return put(o, o[0], o[1], o[2]);
    },
  });
  layer('snakeskin', {
    name: 'Snakeskin', cat: CAT, desc: 'Keeled diamond scales with banded colour and an iridescent glint that slides along.',
    tags: ['snake', 'snakeskin', 'reptile', 'scales', 'python', 'animal print', 'diamond', 'skin'],
    params: [P.color('base', 'Base', '#3e4a24'), P.color('pattern', 'Pattern', '#d1ad55'), P.color('edge', 'Gaps', '#120f08'), P.int('n', 'Scales across', 8, 40, 20), P.int('bands', 'Pattern bands', 1, 10, 4), P.speed(1), P.seed(276)],
    prepare: colorPrep(['base', 'pattern', 'edge']),
    sample(u, v, t, p, o, c) {
      const x = u * p.n, y = v * p.n / c.A;
      const a = x + y, b = x - y, i = Math.floor(a), j = Math.floor(b);
      const fa = a - i - 0.5, fb = b - j - 0.5;
      const m = Math.max(Math.abs(fa), Math.abs(fb)) * 2;
      const id = hash2(wrap(i, p.n), wrap(j, p.n), p.seed);
      const dome = 1 - m * m;
      const ridge = 1 - smoothstep(0, 0.35, Math.abs(fa - fb));
      const tone = 0.5 + 0.45 * Math.sin(TAU * (u * p.bands + 0.1 * Math.sin(TAU * v))) + (id - 0.5) * 0.4;
      mc(o, p._base, p._pattern, smoothstep(0.45, 0.6, tone));
      const f = 0.55 + 0.55 * dome + 0.2 * ridge;
      const s = sheen(u - t * p.speed + id * 0.05, 0.1) * (0.3 + 0.7 * dome) * 0.45;
      const gap = smoothstep(0.8, 0.96, m);
      return put(o, mix(o[0] * f + s, p._edge[0], gap), mix(o[1] * f + s, p._edge[1], gap), mix(o[2] * f + s * 1.1, p._edge[2], gap));
    },
  });

}
