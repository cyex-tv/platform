// Starting points for gradient paints (from the cyex paints tool, within cyex.tv's limits).
import { withAlpha, type Shadow, type StudioPaint } from './format';

const st = (...pairs: [number, string, number?][]) => pairs.map(([at, hex, a]) => ({ at, color: a == null ? hex.toUpperCase() : withAlpha(hex, a) }));
const sh = (x: number, y: number, radius: number, hex: string, a = 1): Shadow => ({ x, y, radius, color: withAlpha(hex, a) });
const paint = (name: string, o: Partial<StudioPaint>): StudioPaint => ({
  name, kind: 'LINEAR_GRADIENT', angle: 90, repeat: false, shape: 'CIRCLE', stops: [], shadows: [], image: null,
  ...o, color: o.color ?? (o.stops?.[0]?.color.slice(0, 7) ?? null),
});

export const STATIC_PRESETS: StudioPaint[] = [
  paint('Icy Lake', { angle: 115, stops: st([0, '#d4f6ff'], [0.15, '#4a8a8f'], [0.25, '#2b4a52'], [0.35, '#4a8a8f'], [0.45, '#d4f6ff'], [0.55, '#d4f6ff'], [0.65, '#4a8a8f'], [0.75, '#2b4a52'], [0.85, '#4a8a8f'], [1, '#d4f6ff']) }),
  paint('Sunset Strip', { stops: st([0, '#ff7e5f'], [0.5, '#feb47b'], [1, '#ff3e8a']), shadows: [sh(0, 0, 3, '#ff3e8a', 0.6)] }),
  paint('Gold Bar', { angle: 180, stops: st([0, '#fff1b8'], [0.35, '#e0aa3e'], [0.5, '#8a5a00'], [0.65, '#f6d365'], [1, '#b8860b']), shadows: [sh(0, 1, 0, '#3b2500')] }),
  paint('Toxic', { angle: 135, stops: st([0, '#39ff14'], [0.5, '#ccff00'], [1, '#00ff9c']), shadows: [sh(0, 0, 4, '#39ff14', 0.8)] }),
  paint('Candy Cane', { angle: 45, repeat: true, stops: st([0, '#ffffff'], [0.1, '#ffffff'], [0.1, '#e8173a'], [0.2, '#e8173a']) }),
  paint('Neon Core', { kind: 'RADIAL_GRADIENT', shape: 'ELLIPSE', stops: st([0, '#ffffff'], [0.3, '#00f0ff'], [0.7, '#ff00e6'], [1, '#2b0057']), shadows: [sh(0, 0, 2, '#00f0ff', 0.9)] }),
  paint('Chrome', { angle: 180, stops: st([0, '#ffffff'], [0.45, '#9fb2c6'], [0.5, '#1c232c'], [0.56, '#d0dae3'], [1, '#6d7d8c']), shadows: [sh(0, 1, 0, '#000000', 0.8)] }),
  paint('Vaporwave', { stops: st([0, '#ff71ce'], [0.33, '#01cdfe'], [0.66, '#05ffa1'], [1, '#b967ff']) }),
  paint('Blood Moon', { kind: 'RADIAL_GRADIENT', stops: st([0, '#ff4d4d'], [0.6, '#8b0000'], [1, '#1a0000']), shadows: [sh(0, 0, 3, '#ff0000', 0.5)] }),
  paint('Rainbow', { stops: st([0, '#ff3b3b'], [0.17, '#ffb13b'], [0.33, '#f6ff3b'], [0.5, '#3bff7a'], [0.67, '#3bc4ff'], [0.83, '#8a3bff'], [1, '#ff3b3b']) }),
  paint('Mint Glass', { angle: 160, stops: st([0, '#e6fff4'], [0.5, '#3ddc97'], [1, '#0f3d3e']), shadows: [sh(1, 1, 0, '#0f3d3e')] }),
  paint('Fade Out', { stops: st([0, '#ffd54f'], [0.55, '#ff7043'], [1, '#ff7043', 0]) }),
];
