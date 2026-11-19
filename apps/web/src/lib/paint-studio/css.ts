// CSS for a painted name in the studio, through the site's own paint renderer
// ($lib/cosmetics/paint), so a paint looks the same here as on profiles, in admin lists and in
// chat. The one thing the site renderer can't take is a local picture (a blob: URL from a render
// or a picked file, not yet on the CDN); those get the same declarations with that address.
import { paintFilter, paintImage, paintStyle as siteStyle, type PaintLike } from '$lib/cosmetics/paint';
import type { StudioPaint } from './format';

const like = (p: StudioPaint): PaintLike => ({
  kind: p.kind,
  angle: p.angle,
  repeat: p.repeat,
  shape: p.shape,
  color: p.color,
  stops: p.stops,
  shadows: p.shadows,
  image: null,
});

const quote = (url: string) => `url("${url.replace(/["\\\n\r()\s]/g, encodeURIComponent)}")`;

/** The `background-image` value: a gradient, the picture, or `none`. */
export function backgroundImage(p: StudioPaint, imageUrl?: string | null): string {
  if (p.kind === 'IMAGE') {
    const url = imageUrl ?? p.image?.url;
    return url ? quote(url) : 'none';
  }
  return paintImage(like(p)) ?? 'none';
}

/** The `filter` value for the shadows, or `none`. */
export const shadowFilter = (p: Pick<StudioPaint, 'shadows'>) => paintFilter(p) || 'none';

/** Inline style declarations that draw the paint into text, as the site draws it. */
export function paintStyleText(p: StudioPaint, imageUrl?: string | null): string {
  if (p.kind !== 'IMAGE') return siteStyle(like(p));
  const url = imageUrl ?? p.image?.url;
  const out: string[] = [];
  if (p.color) out.push(`color: ${p.color.toLowerCase()}`);
  if (url)
    out.push(
      // Under the picture, a flat layer in the text colour: a picture that can't load still
      // leaves a readable name (as the site draws it, lib/cosmetics/paint.ts).
      `background-image: ${quote(url)}, linear-gradient(currentColor, currentColor)`,
      `background-size: ${p.repeat ? 'auto 100%' : 'cover'}, 100% 100%`,
      `background-repeat: ${p.repeat ? 'repeat' : 'no-repeat'}, no-repeat`,
      'background-position: center',
      '-webkit-background-clip: text',
      'background-clip: text',
      '-webkit-text-fill-color: transparent',
    );
  const filter = paintFilter(p);
  if (filter) out.push(`filter: ${filter}`);
  return out.join('; ');
}

/** A stylesheet rule for the paint (the CSS export). */
export function paintCss(p: StudioPaint, selector = '.painted-name'): string {
  const decls = paintStyleText(p).split('; ').filter(Boolean);
  return [`${selector} {`, ...decls.map((d) => `  ${d};`), '}'].join('\n');
}
