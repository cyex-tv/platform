// @ts-nocheck
/* Starting points: static gradient paints and layered animated paints. */
export default function install(PL) {
  'use strict';
  const C = (hex, a = 1) => PL.hexAlphaToDec(hex, a);
  const st = (...pairs) => pairs.map(([at, hex, a]) => ({ at, color: C(hex, a ?? 1) }));
  const sh = (x, y, r, hex, a = 1) => ({ x_offset: x, y_offset: y, radius: r, color: C(hex, a) });
  const paint = (name, o) => Object.assign({ name, function: 'LINEAR_GRADIENT', repeat: false, angle: 90, shape: 'circle', image_url: '', stops: [], shadows: [] }, o);

  PL.STATIC_PRESETS = [
    paint('Icy Lake', { angle: 115, stops: st([0, '#d4f6ff'], [0, '#d4f6ff'], [0.15, '#4a8a8f'], [0.25, '#2b4a52'], [0.35, '#4a8a8f'], [0.45, '#d4f6ff'], [0.5, '#effcff'], [0.55, '#d4f6ff'], [0.65, '#4a8a8f'], [0.75, '#2b4a52'], [0.85, '#4a8a8f'], [1, '#d4f6ff']) }),
    paint('Sunset Strip', { angle: 90, stops: st([0, '#ff7e5f'], [0.5, '#feb47b'], [1, '#ff3e8a']), shadows: [sh(0, 0, 3, '#ff3e8a', 0.6)] }),
    paint('Gold Bar', { angle: 180, stops: st([0, '#fff1b8'], [0.35, '#e0aa3e'], [0.5, '#8a5a00'], [0.65, '#f6d365'], [1, '#b8860b']), shadows: [sh(0, 1, 0, '#3b2500', 1)] }),
    paint('Toxic', { angle: 135, stops: st([0, '#39ff14'], [0.5, '#ccff00'], [1, '#00ff9c']), shadows: [sh(0, 0, 4, '#39ff14', 0.8)] }),
    paint('Candy Cane', { angle: 45, repeat: true, stops: st([0, '#ffffff'], [0.1, '#ffffff'], [0.1, '#e8173a'], [0.2, '#e8173a']) }),
    paint('Neon Core', { function: 'RADIAL_GRADIENT', shape: 'ellipse', stops: st([0, '#ffffff'], [0.3, '#00f0ff'], [0.7, '#ff00e6'], [1, '#2b0057']), shadows: [sh(0, 0, 2, '#00f0ff', 0.9)] }),
    paint('Chrome', { angle: 180, stops: st([0, '#ffffff'], [0.45, '#9fb2c6'], [0.5, '#1c232c'], [0.56, '#d0dae3'], [1, '#6d7d8c']), shadows: [sh(0, 1, 0, '#000000', 0.8)] }),
    paint('Vaporwave', { angle: 90, stops: st([0, '#ff71ce'], [0.33, '#01cdfe'], [0.66, '#05ffa1'], [1, '#b967ff']) }),
    paint('Blood Moon', { function: 'RADIAL_GRADIENT', shape: 'circle', stops: st([0, '#ff4d4d'], [0.6, '#8b0000'], [1, '#1a0000']), shadows: [sh(0, 0, 3, '#ff0000', 0.5)] }),
    paint('Rainbow', { angle: 90, stops: st([0, '#ff3b3b'], [0.17, '#ffb13b'], [0.33, '#f6ff3b'], [0.5, '#3bff7a'], [0.67, '#3bc4ff'], [0.83, '#8a3bff'], [1, '#ff3b3b']) }),
    paint('Mint Glass', { angle: 160, stops: st([0, '#e6fff4'], [0.5, '#3ddc97'], [1, '#0f3d3e']), shadows: [sh(1, 1, 0, '#0f3d3e', 1)] }),
    paint('Fade Out', { angle: 90, stops: st([0, '#ffd54f'], [0.55, '#ff7043'], [1, '#ff7043', 0]) }),
  ];

  const E = PL.Engine;
  const L = (type, params = {}, o = {}) => E.newLayer(type, Object.assign({ params }, o));
  const F = (type, params = {}) => E.newEffect(type, { params });
  const doc = (name, layers, effects = [], over = {}) => Object.assign(E.blankDoc(), { name, layers, effects }, over);
  const pal = (k) => PL.clone(PL.PALETTES[k]);
  const S = (...pairs) => pairs.map(([at, color, a = 1]) => ({ at, color, a }));

  PL.ANIM_PRESETS = [
    () => doc('Liquid Gold', [L('chrome', { stops: pal('gold'), warp: 0.12, freq: 2 }), L('shine', { color: '#fff8dc', width: 0.1, hold: 0.4 }, { blend: 'screen' }), L('sparkles', { color: '#fff3c0', density: 0.25, twinkle: 2 }, { blend: 'add' })], [F('cylinder', { strength: 0.35, hi: 0.35, pos: 0.28 })], { shadows: [sh(0, 1, 0, '#3b2500', 1)] }),
    () => doc('Icy Lake Shimmer', [L('linear', { stops: pal('icyLake'), angle: 115, speed: 1 }), L('shine', { width: 0.08, hold: 0.5, intensity: 0.7 }, { blend: 'screen' }), L('glints', { density: 0.25, rate: 3 }, { blend: 'add', opacity: 0.8 })]),
    () => doc('Northern Lights', [L('solid', { color: '#050b1f' }), L('stars', { density: 0.5, speed: 0 }), L('aurora', { height: 0.6, speed: 1 }, { blend: 'screen' })], [F('glow', { threshold: 0.5, radius: 2, strength: 0.6 })]),
    () => doc('Hellfire', [L('solid', { color: '#120000' }), L('fire', { speed: 2, height: 0.95 }), L('particles', { mode: 'embers', color: '#ffb347', speed: 2, density: 0.35, size: 1.2 }, { blend: 'add' })], [F('glow', { threshold: 0.55, radius: 2, strength: 0.8 })], { shadows: [sh(0, 0, 3, '#ff3d00', 0.7)] }),
    () => doc('Holo Foil', [L('holo', { speed: 1 }), L('glints', { density: 0.5, rate: 4 }, { blend: 'add', opacity: 0.7 })], [F('gloss', { strength: 0.25 })]),
    () => doc('Galaxy', [L('clouds', { stops: pal('galaxy'), speed: 1, flow: 0.6 }), L('stars', { density: 0.45, speed: 1 }, { blend: 'screen' }), L('sparkles', { density: 0.12, size: 2.6 }, { blend: 'add' })]),
    () => doc('Polished Chrome', [L('chrome', { warp: 0.05, speed: 1 }), L('shine', { angle: 60, width: 0.07, hold: 0.55, double: true }, { blend: 'screen' })], [F('cylinder', { strength: 0.25, hi: 0.3 })], { shadows: [sh(0, 1, 0, '#000000', 0.9)] }),
    () => doc('Rainbow Road', [L('rainbow', { speed: 1, light: 0.58 })], [F('cylinder', { strength: 0.5, hi: 0.5 })]),
    () => doc('Toxic Waste', [L('clouds', { stops: pal('toxic'), sx: 6, contrast: 1.8, speed: 1 }), L('particles', { mode: 'bubbles', color: '#e6ff9e', speed: 1, size: 1.8, density: 0.3 }, { blend: 'screen' })], [F('glow', { threshold: 0.6, radius: 2, strength: 0.7 })], { shadows: [sh(0, 0, 3, '#39ff14', 0.6)] }),
    () => doc('Synthwave', [L('linear', { stops: pal('sunset'), angle: 180, speed: 0 }), L('grid', { line: '#ff2bd6', bgAlpha: 0, cols: 20, rows: 4, sx: 0, sy: 1, glow: 0.5 }, { blend: 'screen' })], [F('scanlines', { spacing: 3, dark: 0.25, speed: 1 })]),
    () => doc('Digital Rain', [L('matrix', { speed: 1 })], [F('glow', { threshold: 0.6, radius: 1, strength: 0.6 })]),
    () => doc('Candy Cane', [L('stripes', { a: '#ffffff', b: '#e8173a', angle: 45, count: 10, speed: 1, soft: 0.02 })], [F('cylinder', { strength: 0.55, hi: 0.55, pos: 0.3 }), F('gloss', { strength: 0.15 })]),
    () => doc('Lava Lamp', [L('lava', { speed: 1, count: 8 })], [F('glow', { threshold: 0.5, radius: 2, strength: 0.5 })]),
    () => doc('Deep Ocean', [L('waves', { stops: pal('ocean'), speed: 1, count: 1.5 }), L('caustics', { bg: '#000000', light: '#bff8ff', intensity: 0.8 }, { blend: 'screen', opacity: 0.6 })]),
    () => doc('Glitch Core', [L('linear', { stops: pal('neon'), angle: 90, speed: 1, count: 1 })], [F('glitch', { amount: 0.6, rate: 10, chance: 0.5 }), F('chroma', { offset: 1.5, wobble: 2 }), F('scanlines', { dark: 0.2 })]),
    () => doc('Opal', [L('opal', { speed: 1 })], [F('gloss', { strength: 0.2 })]),
    () => doc('Frozen Crystal', [L('voronoi', { stops: pal('ice'), mode: 'crystal', speed: 1, edge: 0.02 }), L('sparkles', { density: 0.2, twinkle: 2 }, { blend: 'add' })], [F('bevel', { strength: 0.5, depth: 3, spin: 1, soft: 1 })]),
    () => doc('Marble', [L('marble', { speed: 1 })], [F('gloss', { strength: 0.15 })]),
    () => doc('Sakura Snow', [L('linear', { stops: pal('sakura'), angle: 100, speed: 1, mirror: true }), L('particles', { mode: 'snow', color: '#ff6fa8', density: 0.4, size: 1.8, speed: 1, wind: 1 })]),
    () => doc('Electric', [L('solid', { color: '#05001a' }), L('smoke', { stops: S([0, '#000000', 0], [0.6, '#3a00ff', 0.5], [0.85, '#00e5ff'], [1, '#ffffff']), contrast: 2.4, speed: 1, drift: 2 }, { blend: 'screen' })], [F('glow', { threshold: 0.5, radius: 2, strength: 1 }), F('hue', { cycles: 0 })], { shadows: [sh(0, 0, 3, '#00e5ff', 0.6)] }),
    () => doc('Pastel Dream', [L('plasma', { stops: pal('pastel'), speed: 1, shift: 1 })]),
    () => doc('Carbon', [L('carbon', { speed: 1 }), L('shine', { color: '#9ad7ff', width: 0.1, intensity: 0.4, hold: 0.3 }, { blend: 'screen' })]),
    () => doc('Disco', [L('dots', { cols: 24, rows: 4, pulse: 2, dot: '#ffffff', bg: '#ff0080' })], [F('hue', { cycles: 1 })]),
    () => doc('Sunburst', [L('linear', { stops: pal('sunset'), angle: 180, speed: 0 }), L('rays', { speed: 1, intensity: 0.5 }, { blend: 'screen' })]),
  ];

  // ---- scenes built from elements ----
  const W = (hexes, w) => ({ colors: hexes.map((c, i) => ({ c, a: 1, w: w ? w[i] : 1 })), blend: 1, loop: false });
  PL.SCENE_PRESETS = [
    () => doc('Night Sky', [
      L('linear', { stops: pal('night'), angle: 180, speed: 0 }),
      L('stars', { density: 0.5, speed: 0, twinkle: 2 }, { blend: 'screen' }),
      L('shape', { kind: 'moon', fill: 'radial', pal: W(['#fffbe6', '#ffe9a8']), x: 0.82, y: 0.32, size: 0.5, motion: 'float', amount: 0.06, layout: 'single', glow: 0.5, glowColor: '#fff3b0' }, { name: 'Moon' }),
      L('shape', { kind: 'sparkle', fill: 'solid', pal: W(['#ffffff']), x: 0.05, y: 0.1, toX: 0.6, toY: 0.55, size: 0.18, motion: 'path', pingpong: false, mspeed: 1, blink: 0.8, pspeed: 1, layout: 'single', glow: 0.6 }, { name: 'Shooting star' }),
      L('mountains', { pal: W(['#8a6ee0', '#6a52c4', '#4a3a9e']), ridges: 3, y: 0.66, height: 0.3, speed: 1, rim: 0.5, rimColor: '#ffd1ec' }),
    ]),
    () => doc('Synthwave Sun', [
      L('linear', { stops: S([0, '#12002b'], [0.55, '#7b1fa2'], [1, '#ff6ec7']), angle: 180, speed: 0 }),
      L('shape', { kind: 'sun', fill: 'gradient', pal: W(['#ffe066', '#ff8a3d', '#ff2bd6']), x: 0.5, y: 0.55, size: 1.1, motion: 'none', layout: 'single', glow: 0.5, glowColor: '#ff5ec4' }, { name: 'Sun' }),
      L('mountains', { pal: W(['#4a1a6b', '#1b0b33']), ridges: 2, y: 0.62, height: 0.3, speed: 1, rim: 0.7, rimColor: '#00f0ff' }),
      L('grid', { line: '#ff2bd6', bgAlpha: 0, cols: 24, rows: 3, sx: 0, sy: 1, glow: 0.4, width: 0.05 }, { blend: 'screen', opacity: 0.35 }),
    ], [F('scanlines', { spacing: 3, dark: 0.2 })]),
    () => doc('Cherry Blossom', [
      L('mesh', { stops: W(['#ffe4ef', '#ffc2dc', '#fff6fb', '#ff9ec7'], [2, 1.5, 1, 0.6]), move: 0.2 }),
      L('glyph', { text: '🌸', size: 0.35, motion: 'fall', mspeed: 1, spin: 1, layout: 'scatter', count: 16, sizeVar: 0.5 }, { name: 'Petals' }),
      L('sparkles', { color: '#ffffff', density: 0.2 }, { blend: 'screen' }),
    ]),
    () => doc('Floating Hearts', [
      L('linear', { stops: pal('blood'), angle: 160, speed: 0 }),
      L('shape', { kind: 'heart', fill: 'mixed', pal: W(['#ff4f8b', '#ff9ec7', '#ffffff'], [3, 2, 0.6]), motion: 'rise', mspeed: 1, layout: 'scatter', count: 18, size: 0.35, sizeVar: 0.5, glow: 0.4, glowColor: '#ff7eb3' }, { name: 'Hearts' }),
    ]),
    () => doc('Snow Day', [
      L('linear', { stops: pal('ice'), angle: 180, speed: 0 }),
      L('glyph', { text: '❄', fill: 'solid', pal: W(['#ffffff']), size: 0.4, motion: 'fall', mspeed: 1, spin: 1, layout: 'scatter', count: 12, sizeVar: 0.6, glow: 0.3, glowColor: '#bff4ff' }, { name: 'Snowflakes' }),
      L('particles', { mode: 'snow', density: 0.5, size: 1.2, speed: 1 }),
    ]),
    () => doc('City Lights', [
      L('linear', { stops: pal('dusk'), angle: 180, speed: 0 }),
      L('stars', { density: 0.3, speed: 0 }, { blend: 'screen' }),
      L('skyline', { height: 0.7, speed: 1, flicker: 4 }),
    ]),
    () => doc('Ocean Sunset', [
      L('linear', { stops: S([0, '#2b0f4a'], [0.45, '#ff7043'], [0.55, '#ffb07a'], [1, '#0a3d62']), angle: 180, speed: 0 }),
      L('shape', { kind: 'circle', fill: 'gradient', pal: W(['#fff3b0', '#ff9a3c']), x: 0.5, y: 0.5, size: 0.7, motion: 'none', layout: 'single', glow: 0.6, glowColor: '#ffb347' }, { name: 'Sun' }),
      L('ribbon', { fill: 'mixed', stops: undefined, pal: W(['#0a5c8a', '#2ec4d6', '#d9fbff'], [2, 1.5, 0.5]), lines: 5, y: 0.78, gap: 0.07, amp: 0.05, waves: 3, thick: 0.07, speed: 1, glow: 0 }, { name: 'Waves' }),
    ]),
    () => doc('Outer Space', [
      L('clouds', { stops: pal('galaxy'), speed: 1, flow: 0.5 }),
      L('stars', { density: 0.5, speed: 1 }, { blend: 'screen' }),
      L('shape', { kind: 'planet', fill: 'radial', pal: W(['#ffd6a5', '#c77dff', '#ffffff']), x: 0.75, y: 0.5, size: 0.55, motion: 'float', amount: 0.08, layout: 'single', rot: -10 }, { name: 'Planet' }),
    ]),
    () => doc('Bokeh Lights', [
      L('mesh', { stops: W(['#1b1036', '#3a1c71', '#d76d77'], [3, 2, 0.8]) }),
      L('shape', { kind: 'orb', fill: 'mixed', pal: W(['#ffaf7b', '#d76d77', '#fff3c4', '#7f7fd5'], [2, 1.5, 1, 1]), motion: 'wander', amount: 0.25, layout: 'scatter', count: 22, size: 0.6, sizeVar: 0.7 }, { name: 'Lights', blend: 'screen' }),
    ]),
    () => doc('Thunderstorm', [
      L('solid', { color: '#0b1020' }),
      L('smoke', { stops: S([0, '#000000', 0], [0.6, '#2c3550', 0.7], [1, '#8a94b8']), contrast: 1.4, speed: 1, drift: 1 }, { blend: 'screen' }),
      L('lightning', { rate: 2, bolts: 2 }),
      L('particles', { mode: 'rain', color: '#9fb6ff', density: 0.5, speed: 3, wind: 1 }),
    ]),
    () => doc('Neon Ribbons', [L('solid', { color: '#07041a' }), L('ribbon', { lines: 4, glow: 0.6, speed: 1 })], [F('glow', { threshold: 0.4, radius: 2, strength: 0.6 })]),
    () => doc('Mesh Gradient', [L('mesh', { stops: pal('aurora') })]),
    () => doc('Silk', [L('silk', {})]),
    () => doc('Hypnotic', [L('spiral', {})]),
    () => doc('Tie-dye', [L('tiedye', {})]),
    () => doc('Royal', [
      L('liquid', { stops: pal('gold') }),
      L('shape', { kind: 'crown', fill: 'gradient', pal: W(['#fff1b8', '#d4a017', '#8a5a00']), layout: 'row', count: 5, size: 0.5, sizeVar: 0, motion: 'bounce', amount: 0.15, stagger: 1, outline: 0.05, outlineColor: '#3b2500' }, { name: 'Crowns' }),
      L('sparkles', { color: '#fff3c0', density: 0.3 }, { blend: 'add' }),
    ]),
    () => doc('Music', [
      L('linear', { stops: pal('vapor'), angle: 90, speed: 1 }),
      L('shape', { kind: 'note', fill: 'solid', pal: W(['#ffffff']), layout: 'row', count: 6, size: 0.55, motion: 'bounce', amount: 0.2, stagger: 1, glow: 0.4 }, { name: 'Notes' }),
    ]),
  ];
  PL.ANIM_PRESETS.unshift(...PL.SCENE_PRESETS.splice(0));
  PL.SCENE_COUNT = 17;
}
