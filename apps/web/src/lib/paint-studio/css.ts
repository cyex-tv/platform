// CSS for a painted name in the studio, through the site's own paint renderer
// ($lib/cosmetics/paint), so a paint looks the same here as on profiles, in admin lists and in
// chat. A picture not on the CDN yet (a render or a picked file, as a blob: URL) is passed to the
// same renderer as its preview address.
import { paintFilter, paintImage, paintStyle as siteStyle, type PaintLike } from '$lib/cosmetics/paint';
import type { StudioPaint } from './format';

/** The studio's paint in the site renderer's shape. */
export const like = (p: StudioPaint): PaintLike => ({
  kind: p.kind,
  angle: p.angle,
  repeat: p.repeat,
  shape: p.shape,
  color: p.color,
  stops: p.stops,
  shadows: p.shadows,
  image: null,
});

/** The picture an image paint previews with: the one given, else the saved one. */
const pictureOf = (p: StudioPaint, imageUrl?: string | null) => (p.kind === 'IMAGE' ? (imageUrl ?? p.image?.url ?? null) : undefined);

/** The `background-image` value: a gradient, the picture, or `none`. */
export function backgroundImage(p: StudioPaint, imageUrl?: string | null): string {
  return paintImage(like(p), false, pictureOf(p, imageUrl)) ?? 'none';
}

/** The `filter` value for the shadows, or `none`. */
export const shadowFilter = (p: Pick<StudioPaint, 'shadows'>) => paintFilter(p) || 'none';

/** Inline style declarations that draw the paint into text, exactly as the site draws it. */
export function paintStyleText(p: StudioPaint, imageUrl?: string | null): string {
  return siteStyle(like(p), false, pictureOf(p, imageUrl));
}

/** A stylesheet rule for the paint (the CSS export). */
export function paintCss(p: StudioPaint, selector = '.painted-name'): string {
  const decls = paintStyleText(p).split('; ').filter(Boolean);
  return [`${selector} {`, ...decls.map((d) => `  ${d};`), '}'].join('\n');
}
