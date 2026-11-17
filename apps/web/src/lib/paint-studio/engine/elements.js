// @ts-nocheck
/* Scene elements: objects you place, copy and animate (shapes, text and emoji, pictures, ribbons,
   mountains, a city skyline, lightning). They draw with the 2D canvas API: draw(ctx, t, p, c).
   Shared blocks: Colours, Placement, Motion, Copies, Glow and outline. All motion loops seamlessly. */
export default function install(PL) {
  'use strict';
  const { TAU, clamp, fract, mix, tri, hash2, fbm, ridged, rng, rgbaStr, paletteToStops, pickWeighted } = PL;
  const P = PL.P;
  const G = (g, d) => Object.assign(d, { g });
  const when = (fn, d) => Object.assign(d, { show: fn });
  const multi = (p) => p.layout !== 'single';
  // Split text into what people see as characters, so 👨‍👩‍👧, ❄️ and flags stay whole.
  const segmenter = typeof Intl !== 'undefined' && Intl.Segmenter ? new Intl.Segmenter(undefined, { granularity: 'grapheme' }) : null;
  const graphemes = (s) => (segmenter ? [...segmenter.segment(s)].map((x) => x.segment) : [...s]).filter((g) => /\S/.test(g.replace(/[‍️]/g, '')));

  // ---------- shared parameter blocks ----------
  const colours = (o = {}) => [
    G('Colours', P.select('fill', 'Fill', [['gradient', 'Gradient'], ['radial', 'Radial gradient'], ['mixed', 'One colour per copy'], ['solid', 'First colour only']], o.fill ?? 'gradient')),
    G('Colours', P.pal(o.pal ?? 'candy', false, 'pal', 'Colours')),
  ];
  const placement = (o = {}) => [
    G('Placement', P.range('x', 'X', -0.25, 1.25, 0.01, o.x ?? 0.5)),
    G('Placement', P.range('y', 'Y', -0.25, 1.25, 0.01, o.y ?? 0.5)),
    G('Placement', P.range('size', 'Size', 0.05, 2.5, 0.01, o.size ?? 0.6, 'Height of the element, relative to the paint')),
    G('Placement', P.range('stretch', 'Stretch', 0.2, 5, 0.05, o.stretch ?? 1)),
    G('Placement', P.range('rot', 'Rotation', -180, 180, 1, o.rot ?? 0)),
  ];
  const MOTIONS = [['none', 'Stay still'], ['float', 'Float'], ['bounce', 'Bounce'], ['sway', 'Sway'], ['orbit', 'Orbit'], ['figure8', 'Figure eight'], ['wander', 'Wander'], ['driftR', 'Drift right'], ['driftL', 'Drift left'], ['rise', 'Rise'], ['fall', 'Fall'], ['path', 'Move from A to B']];
  const motion = (o = {}) => [
    G('Motion', P.select('motion', 'Movement', MOTIONS, o.motion ?? 'float')),
    when((p) => p.motion !== 'none', G('Motion', P.int('mspeed', 'Speed', 1, 8, o.mspeed ?? 1, 'Cycles per loop'))),
    when((p) => !['none', 'driftR', 'driftL', 'rise', 'fall'].includes(p.motion), G('Motion', P.range('amount', 'Distance', 0, 1, 0.01, o.amount ?? 0.25))),
    when((p) => p.motion === 'path', G('Motion', P.range('toX', 'Point B x', -0.25, 1.25, 0.01, o.toX ?? 0.85))),
    when((p) => p.motion === 'path', G('Motion', P.range('toY', 'Point B y', -0.25, 1.25, 0.01, o.toY ?? 0.5))),
    when((p) => p.motion === 'path', G('Motion', P.bool('pingpong', 'Go back and forth', o.pingpong ?? true))),
    G('Motion', P.speed(o.spin ?? 0, 'spin', 'Spin')),
    G('Motion', P.range('pulse', 'Pulse size', 0, 1, 0.01, o.pulse ?? 0)),
    G('Motion', P.range('blink', 'Fade in and out', 0, 1, 0.01, o.blink ?? 0)),
    when((p) => p.pulse > 0 || p.blink > 0, G('Motion', P.int('pspeed', 'Pulse and fade speed', 1, 8, o.pspeed ?? 2, 'Cycles per loop'))),
  ];
  const copies = (o = {}) => [
    G('Copies', P.select('layout', 'Arrangement', [['single', 'Just one'], ['row', 'Row'], ['grid', 'Grid'], ['scatter', 'Scatter'], ['ring', 'Ring']], o.layout ?? 'single')),
    when(multi, G('Copies', P.int('count', 'How many', 1, 80, o.count ?? 8))),
    when(multi, G('Copies', P.range('spread', 'Spread', 0, 1.5, 0.01, o.spread ?? 1))),
    when(multi, G('Copies', P.range('sizeVar', 'Size variety', 0, 1, 0.01, o.sizeVar ?? 0.35))),
    when(multi, G('Copies', P.range('stagger', 'Timing offset', 0, 1, 0.01, o.stagger ?? 1, 'Spreads the copies out in time so they don\'t move together'))),
    when(multi, G('Copies', P.seed(o.seed ?? 1))),
  ];
  const glow = (o = {}) => [
    G('Glow and outline', P.range('outline', 'Outline', 0, 0.3, 0.005, o.outline ?? 0)),
    when((p) => p.outline > 0, G('Glow and outline', P.color('outlineColor', 'Outline colour', o.outlineColor ?? '#000000'))),
    G('Glow and outline', P.range('glow', 'Glow', 0, 1, 0.01, o.glow ?? 0)),
    when((p) => p.glow > 0, G('Glow and outline', P.color('glowColor', 'Glow colour', o.glowColor ?? '#ffffff'))),
  ];
  PL.ELEMENT_BLOCKS = { colours, placement, motion, copies, glow };

  // ---------- copies and motion ----------
  function instances(p, c) {
    const single = p.layout === 'single' || !p.layout;
    const n = single ? 1 : Math.max(1, p.count | 0), r = rng((p.seed || 1) * 7919 + 13), out = [];
    const cols = p.layout === 'grid' ? Math.max(1, Math.round(Math.sqrt(n * c.A))) : 1, rows = Math.ceil(n / cols);
    for (let i = 0; i < n; i++) {
      const r1 = r(), r2 = r(), r3 = r(), r4 = r(), r5 = r();
      let x = p.x, y = p.y;
      if (p.layout === 'row') x = p.x + ((i + 0.5) / n - 0.5) * p.spread;
      else if (p.layout === 'grid') { x = p.x + (((i % cols) + 0.5) / cols - 0.5) * p.spread; y = p.y + ((Math.floor(i / cols) + 0.5) / rows - 0.5) * p.spread; }
      else if (p.layout === 'scatter') { x = p.x + (r1 - 0.5) * p.spread; y = p.y + (r2 - 0.5) * p.spread; }
      else if (p.layout === 'ring') { const a = (TAU * i) / n; x = p.x + (Math.cos(a) * p.spread * 0.5) / c.A; y = p.y + Math.sin(a) * p.spread * 0.5; }
      const phase = single ? 0 : p.layout === 'scatter' ? r4 * p.stagger : (i / n) * p.stagger;
      const size = p.size * (single ? 1 : 1 - p.sizeVar * r3);
      out.push({ i, x, y, phase, size, rnd: r5, colour: p.fill === 'mixed' ? pickWeighted(p.pal, r()) : null });
    }
    return out;
  }
  function animate(it, p, c, t) {
    let x = it.x * c.W, y = it.y * c.H, rot = ((p.rot || 0) * Math.PI) / 180, sc = 1, al = 1;
    const ph = t * (p.mspeed || 1) + it.phase, A = (p.amount ?? 0) * c.H, s = Math.sin(TAU * ph), co = Math.cos(TAU * ph);
    const m = it.size * c.H * 0.6 * (p.stretch || 1);
    switch (p.motion) {
      case 'float': y -= A * 0.5 * s; break;
      case 'bounce': y -= A * 1.2 * Math.abs(Math.sin(Math.PI * ph)); break;
      case 'sway': rot += p.amount * 0.9 * s; x += A * 0.25 * s; break;
      case 'orbit': x += A * co; y += A * s; break;
      case 'figure8': x += A * s; y += A * 0.5 * Math.sin(2 * TAU * ph); break;
      case 'wander': {
        const k = p.mspeed || 1;
        x += A * 0.6 * (Math.sin(TAU * (t * k + it.rnd)) + 0.5 * Math.sin(TAU * (t * 2 * k + it.phase * 3)));
        y += A * 0.45 * (Math.cos(TAU * (t * k + it.rnd * 2)) + 0.5 * Math.sin(TAU * (t * 3 * k + it.rnd)));
        break;
      }
      case 'driftR': case 'driftL': { const span = c.W + 2 * m; x = fract(it.x + (p.motion === 'driftR' ? 1 : -1) * t * p.mspeed) * span - m; break; }
      case 'rise': case 'fall': { const span = c.H + 2 * m; y = fract(it.y + (p.motion === 'fall' ? 1 : -1) * t * p.mspeed) * span - m; break; }
      case 'path': {
        let k = p.pingpong ? tri(ph) : fract(ph); k = k * k * (3 - 2 * k);
        x = mix(it.x, p.toX + (it.x - p.x), k) * c.W; y = mix(it.y, p.toY + (it.y - p.y), k) * c.H; break;
      }
    }
    rot += TAU * (p.spin || 0) * t;
    if (p.pulse) sc *= 1 + p.pulse * 0.5 * Math.sin(TAU * (t * p.pspeed + it.phase));
    if (p.blink) al *= 1 - p.blink * (0.5 + 0.5 * Math.sin(TAU * (t * p.pspeed + it.phase)));
    return { x, y, rot, sc, al };
  }

  // ---------- fills ----------
  function fillFor(ctx, p, R, it, h = R) {
    if (p.fill === 'mixed' && it && it.colour) return rgbaStr(it.colour.c, it.colour.a ?? 1);
    const st = paletteToStops(p.pal);
    if (p.fill === 'solid') { const c0 = (p.pal.colors.find((c) => c.w > 0) || { c: '#ffffff', a: 1 }); return rgbaStr(c0.c, c0.a ?? 1); }
    const g = p.fill === 'radial' ? ctx.createRadialGradient(-R * 0.25, -h * 0.25, 0, 0, 0, Math.max(R, h) * 1.1) : ctx.createLinearGradient(0, -h, 0, h);
    for (const s of st) g.addColorStop(clamp(s.at), rgbaStr(s.color, s.a));
    return g;
  }

  // ---------- sprites: each element is drawn once per frame-size into a small canvas, then stamped ----------
  function makeSprite(w, h, pad, paint) {
    const cv = PL.makeCanvas(1, 1);
    cv.width = Math.max(1, Math.ceil(w + pad * 2)); cv.height = Math.max(1, Math.ceil(h + pad * 2));
    const ctx = cv.getContext('2d');
    ctx.translate(cv.width / 2, cv.height / 2);
    paint(ctx);
    return cv;
  }
  function stampAll(ctx, t, p, c, spriteFor) {
    const its = instances(p, c);
    for (const it of its) {
      const a = animate(it, p, c, t);
      const spr = spriteFor(it);
      if (!spr) continue;
      const k = (it.size / p.size) * a.sc;
      ctx.save();
      ctx.globalAlpha = clamp(a.al);
      ctx.translate(a.x, a.y); ctx.rotate(a.rot); ctx.scale(k * (p.stretch || 1), k);
      if (p.glow > 0) { ctx.shadowColor = p.glowColor; ctx.shadowBlur = p.glow * it.size * c.H * 0.6; }
      ctx.drawImage(spr, -spr.width / 2, -spr.height / 2);
      if (p.glow > 0.6) ctx.drawImage(spr, -spr.width / 2, -spr.height / 2);
      ctx.restore();
    }
  }
  // Sprite cache lives on the compiled params, so it resets whenever settings change.
  function cached(p, key, make) { p._spr = p._spr || {}; return p._spr[key] || (p._spr[key] = make()); }

  // ---------- shapes ----------
  const SHAPES = [['circle', 'Circle'], ['orb', 'Soft orb'], ['ring', 'Ring'], ['square', 'Square'], ['rounded', 'Rounded square'], ['triangle', 'Triangle'], ['diamond', 'Diamond'], ['star', 'Star'], ['sparkle', 'Sparkle'], ['heart', 'Heart'], ['moon', 'Crescent moon'], ['sun', 'Striped sun'], ['planet', 'Planet with ring'], ['bolt', 'Lightning bolt'], ['hexagon', 'Hexagon'], ['polygon', 'Polygon'], ['cross', 'Plus'], ['drop', 'Drop'], ['cloud', 'Cloud'], ['flower', 'Flower'], ['note', 'Music note'], ['crown', 'Crown']];
  function poly(ctx, n, R, inner, rot = -Math.PI / 2) {
    ctx.beginPath();
    const k = inner ? n * 2 : n;
    for (let i = 0; i < k; i++) { const r = inner && i % 2 ? R * inner : R, a = rot + (i / k) * TAU; i ? ctx.lineTo(Math.cos(a) * r, Math.sin(a) * r) : ctx.moveTo(Math.cos(a) * r, Math.sin(a) * r); }
    ctx.closePath();
  }
  function shapePath(ctx, kind, R, p) {
    ctx.beginPath();
    switch (kind) {
      case 'square': ctx.rect(-R, -R, 2 * R, 2 * R); break;
      case 'rounded': ctx.roundRect ? ctx.roundRect(-R, -R, 2 * R, 2 * R, R * 0.35) : ctx.rect(-R, -R, 2 * R, 2 * R); break;
      case 'triangle': poly(ctx, 3, R * 1.1); break;
      case 'diamond': ctx.moveTo(0, -R); ctx.lineTo(R * 0.7, 0); ctx.lineTo(0, R); ctx.lineTo(-R * 0.7, 0); ctx.closePath(); break;
      case 'star': poly(ctx, p.points || 5, R, p.inner || 0.45); break;
      case 'polygon': poly(ctx, p.points || 6, R); break;
      case 'hexagon': poly(ctx, 6, R, 0, 0); break;
      case 'sparkle': ctx.moveTo(0, -R); ctx.quadraticCurveTo(R * 0.12, -R * 0.12, R, 0); ctx.quadraticCurveTo(R * 0.12, R * 0.12, 0, R); ctx.quadraticCurveTo(-R * 0.12, R * 0.12, -R, 0); ctx.quadraticCurveTo(-R * 0.12, -R * 0.12, 0, -R); ctx.closePath(); break;
      case 'heart': ctx.moveTo(0, R * 0.95); ctx.bezierCurveTo(-R * 1.25, R * 0.1, -R * 0.95, -R * 1.05, 0, -R * 0.45); ctx.bezierCurveTo(R * 0.95, -R * 1.05, R * 1.25, R * 0.1, 0, R * 0.95); ctx.closePath(); break;
      case 'bolt': ctx.moveTo(R * 0.15, -R); ctx.lineTo(-R * 0.55, R * 0.1); ctx.lineTo(-R * 0.05, R * 0.1); ctx.lineTo(-R * 0.25, R); ctx.lineTo(R * 0.55, -R * 0.15); ctx.lineTo(R * 0.05, -R * 0.15); ctx.closePath(); break;
      case 'cross': { const w = R * 0.33; ctx.rect(-w, -R, 2 * w, 2 * R); ctx.rect(-R, -w, 2 * R, 2 * w); break; }
      case 'drop': ctx.moveTo(0, -R); ctx.bezierCurveTo(R * 0.25, -R * 0.4, R * 0.75, 0, R * 0.75, R * 0.3); ctx.arc(0, R * 0.3, R * 0.75, 0, Math.PI); ctx.bezierCurveTo(-R * 0.75, 0, -R * 0.25, -R * 0.4, 0, -R); ctx.closePath(); break;
      case 'cloud': for (const [x, y, r] of [[-0.55, 0.2, 0.42], [0, -0.1, 0.55], [0.55, 0.2, 0.42], [0.25, 0.35, 0.4], [-0.25, 0.35, 0.4]]) { ctx.moveTo((x + r) * R, y * R); ctx.arc(x * R, y * R, r * R, 0, TAU); } break;
      case 'flower': for (let i = 0; i < 5; i++) { const a = (i / 5) * TAU - Math.PI / 2; ctx.moveTo(Math.cos(a) * R * 0.5 + R * 0.5, Math.sin(a) * R * 0.5); ctx.arc(Math.cos(a) * R * 0.5, Math.sin(a) * R * 0.5, R * 0.5, 0, TAU); } break;
      case 'note': ctx.ellipse(-R * 0.3, R * 0.6, R * 0.38, R * 0.28, -0.4, 0, TAU); ctx.rect(R * 0.02, -R, R * 0.14, R * 1.6); ctx.moveTo(R * 0.16, -R); ctx.quadraticCurveTo(R * 0.8, -R * 0.6, R * 0.55, -R * 0.1); ctx.quadraticCurveTo(R * 0.6, -R * 0.55, R * 0.16, -R * 0.6); ctx.closePath(); break;
      case 'crown': ctx.moveTo(-R, R * 0.6); ctx.lineTo(-R, -R * 0.5); ctx.lineTo(-R * 0.5, 0); ctx.lineTo(0, -R * 0.8); ctx.lineTo(R * 0.5, 0); ctx.lineTo(R, -R * 0.5); ctx.lineTo(R, R * 0.6); ctx.closePath(); break;
      default: ctx.arc(0, 0, R, 0, TAU);
    }
  }
  function shapeSprite(p, R, it) {
    const kind = p.kind, pad = Math.ceil(R * (p.outline || 0) * 2 + 2);
    const wide = kind === 'planet' ? 1.9 : 1;
    return makeSprite(2 * R * wide, 2 * R, pad, (ctx) => {
      if (kind === 'orb') {
        const st = paletteToStops(p.pal), col = p.fill === 'mixed' && it && it.colour ? it.colour : (p.pal.colors[0] || { c: '#ffffff', a: 1 });
        const g = ctx.createRadialGradient(0, 0, 0, 0, 0, R);
        if (p.fill === 'mixed' || p.fill === 'solid') { g.addColorStop(0, rgbaStr(col.c, col.a ?? 1)); g.addColorStop(0.5, rgbaStr(col.c, (col.a ?? 1) * 0.55)); g.addColorStop(1, rgbaStr(col.c, 0)); }
        else { for (const s of st) g.addColorStop(clamp(s.at), rgbaStr(s.color, s.a * (1 - s.at * 0.9))); g.addColorStop(1, 'rgba(0,0,0,0)'); }
        ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0, 0, R, 0, TAU); ctx.fill();
        return;
      }
      const fs = fillFor(ctx, p, R, it);
      if (kind === 'planet') {
        const last = p.pal.colors[p.pal.colors.length - 1] || { c: '#ffffff', a: 1 };
        ctx.lineWidth = R * 0.12; ctx.strokeStyle = rgbaStr(last.c, 0.9);
        ctx.beginPath(); ctx.ellipse(0, 0, R * 1.75, R * 0.42, -0.25, Math.PI, TAU); ctx.stroke();
        ctx.fillStyle = fs; ctx.beginPath(); ctx.arc(0, 0, R * 0.8, 0, TAU); ctx.fill();
        ctx.beginPath(); ctx.ellipse(0, 0, R * 1.75, R * 0.42, -0.25, 0, Math.PI); ctx.stroke();
        return;
      }
      if (kind === 'ring') { ctx.beginPath(); ctx.arc(0, 0, R, 0, TAU); ctx.arc(0, 0, R * (1 - (p.thick ?? 0.3)), 0, TAU, true); ctx.fillStyle = fs; ctx.fill(); }
      else { shapePath(ctx, kind, R, p); ctx.fillStyle = fs; ctx.fill('nonzero'); }
      if (p.outline > 0 && kind !== 'ring') { shapePath(ctx, kind, R, p); ctx.lineWidth = p.outline * R * 2; ctx.strokeStyle = p.outlineColor; ctx.lineJoin = 'round'; ctx.stroke(); }
      ctx.globalCompositeOperation = 'destination-out';
      if (kind === 'moon') { ctx.beginPath(); ctx.arc(R * 0.42, -R * 0.18, R * 0.82, 0, TAU); ctx.fill(); }
      if (kind === 'sun') { for (let i = 0; i < 6; i++) { const y = R * (0.12 + i * 0.16), hgt = R * (0.03 + i * 0.022); ctx.fillRect(-R, y, 2 * R, hgt); } }
      ctx.globalCompositeOperation = 'source-over';
    });
  }
  const elementLayer = (id, def) => { def.cat = 'Scene elements'; def.id = id; PL.LAYERS[id] = def; };

  elementLayer('shape', {
    name: 'Shape', desc: 'Circles, hearts, stars, moons, planets and more. Copy, arrange and animate them.',
    params: [
      ...colours({ pal: 'candy' }),
      G('Shape', P.select('kind', 'Shape', SHAPES, 'heart')),
      when((p) => p.kind === 'star' || p.kind === 'polygon', G('Shape', P.int('points', 'Points', 3, 12, 5))),
      when((p) => p.kind === 'star', G('Shape', P.range('inner', 'Inner radius', 0.1, 0.9, 0.01, 0.45))),
      when((p) => p.kind === 'ring', G('Shape', P.range('thick', 'Ring thickness', 0.05, 0.9, 0.01, 0.3))),
      ...placement({ size: 0.5 }), ...motion({ motion: 'float' }), ...copies({ layout: 'row', count: 6 }), ...glow(),
    ],
    draw(ctx, t, p, c) {
      const R = Math.min(400, (p.size * c.H) / 2);
      stampAll(ctx, t, p, c, (it) => cached(p, R.toFixed(1) + (it.colour ? it.colour.c : ''), () => shapeSprite(p, R, it)));
    },
  });

  const FONTS = [['sans', 'Sans'], ['serif', 'Serif'], ['mono', 'Mono'], ['rounded', 'Rounded'], ['script', 'Script'], ['impact', 'Heavy']];
  const fontStack = { sans: "'Segoe UI', system-ui, sans-serif", serif: "Georgia, 'Times New Roman', serif", mono: "Consolas, 'Cascadia Mono', monospace", rounded: "'Arial Rounded MT Bold', 'Segoe UI', sans-serif", script: "'Segoe Script', 'Brush Script MT', cursive", impact: "Impact, 'Arial Black', sans-serif" };
  elementLayer('glyph', {
    name: 'Text and emoji', desc: 'Any characters: emoji like 🌸 ❄️ ⭐ 🔥, symbols like ♥ ✦ ☾, or a word.',
    params: [
      G('Shape', P.text('text', 'Characters', '❄')),
      G('Shape', P.select('font', 'Font', FONTS, 'sans')),
      G('Shape', P.bool('bold', 'Bold', true)),
      G('Shape', P.bool('cycle', 'Copies take turns through the characters', true)),
      ...colours({ pal: 'ice', fill: 'solid' }),
      ...placement({ size: 0.45 }), ...motion({ motion: 'fall', spin: 1 }), ...copies({ layout: 'scatter', count: 14 }), ...glow(),
    ],
    draw(ctx, t, p, c) {
      const R = Math.min(400, (p.size * c.H) / 2);
      const chars = graphemes(p.text || '');
      if (!chars.length) return;
      stampAll(ctx, t, p, c, (it) => {
        const str = p.cycle ? chars[it.i % chars.length] : p.text;
        return cached(p, R.toFixed(1) + str + (it.colour ? it.colour.c : ''), () => {
          const font = `${p.bold ? 700 : 400} ${R * 2}px ${fontStack[p.font] || fontStack.sans}`;
          const mc = PL.makeCanvas(1, 1).getContext('2d'); mc.font = font;
          const w = Math.max(R, mc.measureText(str).width), pad = Math.ceil(R * (p.outline || 0) * 2 + 2);
          return makeSprite(w, R * 2.4, pad, (x) => {
            x.font = font; x.textAlign = 'center'; x.textBaseline = 'middle';
            if (p.outline > 0) { x.lineWidth = p.outline * R * 2; x.strokeStyle = p.outlineColor; x.lineJoin = 'round'; x.strokeText(str, 0, 0); }
            x.fillStyle = fillFor(x, p, w / 2, it, R); x.fillText(str, 0, 0);
          });
        });
      });
    },
  });

  elementLayer('sprite', {
    name: 'Picture', desc: 'Your own image as a movable element. PNGs with transparency work best.',
    params: [
      G('Shape', P.image()),
      G('Colours', P.color('tint', 'Tint colour', '#ff7bd5')),
      G('Colours', P.range('tintAmt', 'Tint', 0, 1, 0.01, 0)),
      ...placement({ size: 0.8 }), ...motion({ motion: 'float' }), ...copies({ layout: 'single' }), ...glow(),
    ],
    draw(ctx, t, p, c) {
      const im = PL.imageCache[p.src];
      if (!im) return;
      if (!im.canvas) { im.canvas = PL.makeCanvas(im.w, im.h); im.canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(im.data), im.w, im.h), 0, 0); }
      const H = Math.min(512, p.size * c.H), W = (H * im.w) / im.h;
      stampAll(ctx, t, p, c, () => cached(p, H.toFixed(1), () => makeSprite(W, H, 1, (x) => {
        x.drawImage(im.canvas, -W / 2, -H / 2, W, H);
        if (p.tintAmt > 0) { x.globalCompositeOperation = 'source-atop'; x.globalAlpha = p.tintAmt; x.fillStyle = p.tint; x.fillRect(-W / 2, -H / 2, W, H); }
      })));
    },
  });

  elementLayer('ribbon', {
    name: 'Ribbons', desc: 'Flowing wavy lines, like light trails or sound waves.',
    params: [
      G('Colours', P.select('fill', 'Colour', [['along', 'Gradient along the line'], ['mixed', 'One colour per line']], 'along')),
      G('Colours', P.pal('neonPop', true, 'pal', 'Colours')),
      G('Shape', P.int('lines', 'Lines', 1, 12, 3)),
      G('Shape', P.range('y', 'Height', 0, 1, 0.01, 0.5)),
      G('Shape', P.range('gap', 'Gap between lines', 0, 0.5, 0.01, 0.12)),
      G('Shape', P.range('amp', 'Wave height', 0, 0.6, 0.01, 0.22)),
      G('Shape', P.int('waves', 'Waves across', 1, 10, 2)),
      G('Shape', P.range('thick', 'Thickness', 0.005, 0.3, 0.005, 0.05)),
      G('Shape', P.range('offset', 'Line offset', 0, 1, 0.01, 0.15)),
      G('Motion', P.speed(1, 'speed', 'Flow speed')),
      ...glow({ glow: 0.5 }),
    ],
    draw(ctx, t, p, c) {
      const { W, H } = c, r = rng(99);
      const st = paletteToStops(p.pal);
      if (p.glow > 0) { ctx.shadowColor = p.glowColor; ctx.shadowBlur = p.glow * H * 0.25; }
      ctx.lineCap = 'round'; ctx.lineJoin = 'round'; ctx.lineWidth = Math.max(1, p.thick * H);
      for (let i = 0; i < p.lines; i++) {
        const y0 = (p.y + (i - (p.lines - 1) / 2) * p.gap) * H;
        ctx.beginPath();
        for (let x = -4; x <= W + 4; x += 2) {
          const y = y0 + p.amp * H * Math.sin(TAU * ((x / W) * p.waves - t * p.speed + i * p.offset));
          x < -2 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
        }
        if (p.fill === 'mixed') { const col = pickWeighted(p.pal, r()); ctx.strokeStyle = rgbaStr(col.c, col.a ?? 1); }
        else { const g = ctx.createLinearGradient(0, 0, W, 0); for (const s of st) g.addColorStop(clamp(s.at), rgbaStr(s.color, s.a)); ctx.strokeStyle = g; }
        ctx.stroke();
      }
    },
  });

  elementLayer('mountains', {
    name: 'Mountains', desc: 'Layered mountain ridges with parallax scrolling. Colours run from the back ridge to the front.',
    params: [
      G('Colours', P.pal('peaks', false, 'pal', 'Colours (back to front)')),
      G('Shape', P.int('ridges', 'Ridges', 1, 5, 3)),
      G('Shape', P.range('y', 'Base height', 0, 1, 0.01, 0.55)),
      G('Shape', P.range('height', 'Peak height', 0.05, 1, 0.01, 0.45)),
      G('Shape', P.int('rough', 'Peaks across', 1, 16, 4)),
      G('Shape', P.select('style', 'Style', [['sharp', 'Sharp peaks'], ['soft', 'Rolling hills']], 'sharp')),
      G('Motion', P.speed(1, 'speed', 'Scroll speed')),
      G('Glow and outline', P.range('rim', 'Rim light', 0, 1, 0.01, 0.4)),
      G('Glow and outline', P.color('rimColor', 'Rim colour', '#ff9ad5')),
      G('Settings', P.seed(3)),
    ],
    draw(ctx, t, p, c) {
      const { W, H } = c, n = p.ridges, lut = PL.buildLut(paletteToStops(p.pal)); // once per frame, not per ridge
      for (let l = 0; l < n; l++) {
        const depth = n === 1 ? 1 : l / (n - 1);
        const base = (p.y + depth * (1 - p.y) * 0.55) * H, amp = p.height * H * (0.7 + 0.3 * (1 - depth));
        const per = p.rough * (l + 1), shift = t * p.speed * (l + 1);
        const pts = [];
        for (let x = 0; x <= W; x += 2) {
          const u = x / W - shift;
          const nn = p.style === 'soft' ? fbm(u * per, l * 3.1, per, 64, 3, p.seed + l) : ridged(u * per, l * 3.1, per, 64, 4, p.seed + l);
          pts.push([x, base - amp * nn]);
        }
        const col = PL.lutAt(lut, depth, [0, 0, 0, 0]);
        ctx.beginPath(); ctx.moveTo(0, H + 2);
        for (const [x, y] of pts) ctx.lineTo(x, y);
        ctx.lineTo(W, H + 2); ctx.closePath();
        ctx.fillStyle = `rgba(${col[0] * 255},${col[1] * 255},${col[2] * 255},${col[3]})`; ctx.fill();
        if (p.rim > 0) {
          ctx.beginPath(); pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
          ctx.strokeStyle = p.rimColor; ctx.globalAlpha = p.rim * (0.4 + 0.6 * (1 - depth)); ctx.lineWidth = Math.max(1, H * 0.02); ctx.stroke(); ctx.globalAlpha = 1;
        }
      }
    },
  });

  elementLayer('skyline', {
    name: 'City skyline', desc: 'Buildings with lit, flickering windows. Can scroll past.',
    params: [
      G('Colours', P.color('body', 'Buildings', '#120c24')),
      G('Colours', P.color('back', 'Back row', '#2a1d4f')),
      G('Colours', P.pal('candy', false, 'pal', 'Window colours')),
      G('Shape', P.range('height', 'Height', 0.1, 1, 0.01, 0.6)),
      G('Shape', P.range('width', 'Building width', 0.02, 0.3, 0.005, 0.07)),
      G('Shape', P.range('lit', 'Lit windows', 0, 1, 0.01, 0.45)),
      G('Shape', P.bool('backRow', 'Back row of buildings', true)),
      G('Motion', P.speed(1, 'speed', 'Scroll speed')),
      G('Motion', P.int('flicker', 'Window changes per loop', 0, 12, 3)),
      G('Settings', P.seed(8)),
    ],
    draw(ctx, t, p, c) {
      const { W, H } = c;
      const row = (seed, hMul, col, windows) => {
        const r = rng(seed * 977), bs = [];
        let x = 0;
        while (x < W) { const w = Math.max(3, (p.width * (0.6 + r() * 0.9)) * W); bs.push([x, w, (0.35 + r() * 0.65) * p.height * H * hMul]); x += w; }
        // the back row scrolls slower for depth, but always a whole number of its own widths per loop
        const rate = windows ? p.speed : Math.sign(p.speed) * Math.max(1, Math.floor(Math.abs(p.speed) / 2));
        const total = x, off = ((t * rate * total) % total + total) % total;
        for (const [bx, bw, bh] of bs) for (const dx of [-total, 0, total]) {
          const X = bx - off + dx; if (X > W || X + bw < 0) continue;
          ctx.fillStyle = col; ctx.fillRect(Math.floor(X), H - bh, Math.ceil(bw) - 1, bh);
          if (!windows) continue;
          const ws = Math.max(2, H * 0.045), step = Math.floor(t * p.flicker);
          for (let wy = H - bh + ws; wy < H - ws; wy += ws * 1.8) for (let wx = X + ws * 0.7; wx < X + bw - ws; wx += ws * 1.6) {
            const hh = hash2(Math.round(bx + wx - X), Math.round(wy), seed + (hash2(Math.round(bx), Math.round(wy), 5) < 0.2 ? step : 0));
            if (hh > p.lit) continue;
            const wc = pickWeighted(p.pal, hash2(Math.round(bx), Math.round(wy * 3), seed + 2));
            ctx.fillStyle = rgbaStr(wc.c, wc.a ?? 1); ctx.fillRect(wx, wy, ws * 0.8, ws);
          }
        }
      };
      if (p.backRow) row(p.seed + 50, 1.25, p.back, false);
      row(p.seed, 1, p.body, true);
    },
  });

  elementLayer('lightning', {
    name: 'Lightning', desc: 'Jagged bolts that strike and flicker.',
    params: [
      G('Colours', P.color('core', 'Core', '#ffffff')),
      G('Glow and outline', P.color('glowColor', 'Glow colour', '#7fb2ff')),
      G('Glow and outline', P.range('glow', 'Glow', 0, 1, 0.01, 0.7)),
      G('Shape', P.int('bolts', 'Bolts per strike', 1, 6, 2)),
      G('Shape', P.range('jag', 'Jaggedness', 0, 1, 0.01, 0.5)),
      G('Shape', P.int('branches', 'Branches', 0, 6, 2)),
      G('Shape', P.range('thick', 'Thickness', 0.005, 0.1, 0.005, 0.025)),
      G('Motion', P.int('rate', 'Strikes per loop', 1, 12, 3)),
      G('Motion', P.range('on', 'Time lit', 0.05, 1, 0.01, 0.35)),
      G('Settings', P.seed(13)),
    ],
    draw(ctx, t, p, c) {
      const { W, H } = c, step = Math.floor(t * p.rate), local = fract(t * p.rate);
      if (local > p.on) return;
      const flick = local < p.on * 0.3 ? 1 : 0.55 + 0.45 * hash2(step, Math.floor(local * 40), p.seed);
      const r = rng(p.seed * 31 + step * 7);
      const bolt = (x0, y0, x1, y1, w, depth) => {
        const segs = 14, pts = [[x0, y0]];
        for (let i = 1; i < segs; i++) { const k = i / segs; pts.push([mix(x0, x1, k) + (r() - 0.5) * p.jag * H * 0.6, mix(y0, y1, k) + (r() - 0.5) * p.jag * H * 0.1]); }
        pts.push([x1, y1]);
        ctx.lineWidth = w; ctx.beginPath(); pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y))); ctx.stroke();
        if (depth < 1) for (let b = 0; b < p.branches; b++) { const s = pts[1 + Math.floor(r() * (segs - 2))]; bolt(s[0], s[1], s[0] + (r() - 0.5) * W * 0.25, s[1] + H * (0.2 + r() * 0.4), w * 0.5, depth + 1); }
      };
      ctx.globalAlpha = flick; ctx.strokeStyle = p.core; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
      ctx.shadowColor = p.glowColor; ctx.shadowBlur = p.glow * H * 0.3;
      for (let i = 0; i < p.bolts; i++) { const x = r() * W; bolt(x, -2, x + (r() - 0.5) * W * 0.2, H + 2, Math.max(1, p.thick * H), 0); }
    },
  });
}
