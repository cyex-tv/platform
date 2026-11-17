// CSS for a painted name, from the cyex.tv paint model (the API's descriptions: `linear-gradient(<angle>deg,
// <stops>)`, `radial-gradient(<shape>, <stops>)`, `repeating-*` with repeat, the picture for IMAGE,
// shadows as `drop-shadow(x y radius color)`), clipped to the text. This is the studio's local
// renderer; the site's own paint renderer replaces it once it exists.
import type { StudioPaint } from './format';

const pct = (at: number) => `${Math.round(at * 100000) / 1000}%`;

/** The `background-image` value: a gradient, the picture, or `none`. */
export function backgroundImage(p: StudioPaint, imageUrl?: string | null): string {
  if (p.kind === 'IMAGE') {
    const url = imageUrl ?? p.image?.url;
    return url ? `url("${url.replace(/["\\\n\r]/g, encodeURIComponent)}")` : 'none';
  }
  const stops = p.stops.map((s) => `${s.color} ${pct(s.at)}`).join(', ');
  const head = p.kind === 'LINEAR_GRADIENT' ? `${p.angle}deg` : p.shape.toLowerCase();
  const fn = p.kind === 'LINEAR_GRADIENT' ? 'linear-gradient' : 'radial-gradient';
  return `${p.repeat ? 'repeating-' : ''}${fn}(${head}, ${stops})`;
}

/** The `filter` value for the shadows, or `none`. */
export const shadowFilter = (p: Pick<StudioPaint, 'shadows'>) =>
  p.shadows.length ? p.shadows.map((s) => `drop-shadow(${s.x}px ${s.y}px ${s.radius}px ${s.color})`).join(' ') : 'none';

export type PaintStyle = {
  backgroundImage: string;
  backgroundSize: string;
  backgroundRepeat: string;
  filter: string;
  color: string;
};

/** Every declaration a painted name needs besides the clip (see `paintedClass`). */
export function paintStyle(p: StudioPaint, imageUrl?: string | null): PaintStyle {
  const image = p.kind === 'IMAGE';
  return {
    backgroundImage: backgroundImage(p, imageUrl),
    // a picture stretches over the name, or tiles along it at the name's height with repeat
    backgroundSize: image ? (p.repeat ? 'auto 100%' : '100% 100%') : 'auto',
    backgroundRepeat: image ? (p.repeat ? 'repeat-x' : 'no-repeat') : 'no-repeat',
    filter: shadowFilter(p),
    color: p.color ?? 'currentColor',
  };
}

/** The same as an inline `style` string. */
export function paintStyleText(p: StudioPaint, imageUrl?: string | null): string {
  const s = paintStyle(p, imageUrl);
  return `background-image:${s.backgroundImage};background-size:${s.backgroundSize};background-repeat:${s.backgroundRepeat};filter:${s.filter};color:${s.color}`;
}

/** A stylesheet rule for the paint (for the CSS export). */
export function paintCss(p: StudioPaint, selector = '.painted-name'): string {
  const s = paintStyle(p);
  const lines = [
    `${selector} {`,
    `  color: ${s.color};`,
    `  background-image: ${s.backgroundImage};`,
    `  background-size: ${s.backgroundSize};`,
    `  background-repeat: ${s.backgroundRepeat};`,
    '  -webkit-background-clip: text;',
    '  background-clip: text;',
    '  -webkit-text-fill-color: transparent;',
  ];
  if (s.filter !== 'none') lines.push(`  filter: ${s.filter};`);
  lines.push('}');
  return lines.join('\n');
}
