// A name paint as CSS (ADR 0015). Pure: a Paint in, style declarations out, so the same paint
// looks the same on the profile, in the account menu, in admin previews and on channel pages.
// Everything that reaches CSS is checked first: colours against a strict #RRGGBB(AA) pattern,
// numbers clamped to the API's ranges, image addresses limited to http(s) with no characters that
// could close the url(). A paint that fails the checks falls back to its plain colour, or to the
// inherited text colour.

export type PaintStop = { at: number; color: string };
export type PaintShadow = { x: number; y: number; radius: number; color: string };
export type PaintImage = { host: string; animated: boolean; ready?: boolean; files: { name: string }[] };
export type PaintLike = {
  kind: 'LINEAR_GRADIENT' | 'RADIAL_GRADIENT' | 'IMAGE';
  angle: number;
  repeat: boolean;
  shape: 'CIRCLE' | 'ELLIPSE';
  color?: string | null;
  stops: readonly PaintStop[];
  shadows: readonly PaintShadow[];
  image?: PaintImage | null;
};

const COLOR = /^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/;
/** Characters allowed in an image address inside url("…"): no quotes, parentheses, backslashes or spaces. */
const URL_SAFE = /^[A-Za-z0-9\-._~:/?#[\]@!$&*+,;=%]+$/;

export const MAX_SHADOWS = 4;
export const MAX_STOPS = 10;

/** A colour the API allows (`#RRGGBB` or `#RRGGBBAA`), or null. */
export function safeColor(c: string | null | undefined): string | null {
  return typeof c === 'string' && COLOR.test(c) ? c.toLowerCase() : null;
}

const clamp = (v: number, lo: number, hi: number) => (Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : lo);
const num = (v: number) => String(Math.round(v * 100) / 100);

/** An http(s) address safe to put in url("…"), or null. */
export function safeUrl(u: string): string | null {
  try {
    const parsed = new URL(u);
    if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') return null;
  } catch {
    return null;
  }
  return URL_SAFE.test(u) ? u : null;
}

/** The gradient stops, clamped to 0..1 and never going backwards; null when fewer than 2 are valid. */
function stopList(stops: readonly PaintStop[]): string | null {
  const out: string[] = [];
  let last = 0;
  for (const s of stops.slice(0, MAX_STOPS)) {
    const c = safeColor(s.color);
    if (!c) return null;
    last = Math.max(last, clamp(s.at, 0, 1));
    out.push(`${c} ${num(last * 100)}%`);
  }
  return out.length >= 2 ? out.join(', ') : null;
}

/** The image paint's picture: the first frame when `still` (reduced motion) and it has one. */
function imageUrl(img: PaintImage | null | undefined, still: boolean): string | null {
  if (!img || img.ready === false || !img.host) return null;
  const names = new Set(img.files.map((f) => f.name));
  const name = still && img.animated && names.has('1x_static.webp') ? '1x_static.webp' : '1x.webp';
  if (!names.has(name)) return null;
  return safeUrl(`${img.host.replace(/\/+$/, '')}/${name}`);
}

/** The paint's `background-image` value (gradient or url), or null when it can't be drawn. */
export function paintImage(p: PaintLike, still = false): string | null {
  const rep = p.repeat ? 'repeating-' : '';
  if (p.kind === 'IMAGE') {
    const url = imageUrl(p.image, still);
    return url ? `url("${url}")` : null;
  }
  const stops = stopList(p.stops);
  if (!stops) return null;
  if (p.kind === 'RADIAL_GRADIENT') return `${rep}radial-gradient(${p.shape === 'ELLIPSE' ? 'ellipse' : 'circle'}, ${stops})`;
  if (p.kind === 'LINEAR_GRADIENT') return `${rep}linear-gradient(${num(clamp(p.angle, 0, 360))}deg, ${stops})`;
  return null;
}

/** `drop-shadow(…)` filters, at most 4, values clamped to the API's ranges; '' for none. */
export function paintFilter(p: Pick<PaintLike, 'shadows'>): string {
  return p.shadows
    .slice(0, MAX_SHADOWS)
    .map((s) => {
      const c = safeColor(s.color);
      if (!c) return '';
      return `drop-shadow(${num(clamp(s.x, -16, 16))}px ${num(clamp(s.y, -16, 16))}px ${num(clamp(s.radius, 0, 16))}px ${c})`;
    })
    .filter(Boolean)
    .join(' ');
}

/** A plain colour for the paint: its fallback, else its first stop; null when it has neither. */
export function paintColor(p: PaintLike): string | null {
  return safeColor(p.color) ?? safeColor(p.stops[0]?.color);
}

/**
 * Style declarations that draw the paint into text (`style="…"` on an inline element). Without a
 * drawable paint, only the fallback colour (or nothing). `still` uses an image paint's first frame.
 */
export function paintStyle(p: PaintLike | null | undefined, still = false): string {
  if (!p) return '';
  const fallback = paintColor(p);
  const image = paintImage(p, still);
  const out: string[] = [];
  if (fallback) out.push(`color: ${fallback}`);
  if (image) {
    out.push(
      `background-image: ${image}`,
      p.kind === 'IMAGE' ? `background-size: ${p.repeat ? 'auto 100%' : 'cover'}` : 'background-size: 100% 100%',
      `background-repeat: ${p.kind === 'IMAGE' && p.repeat ? 'repeat' : 'no-repeat'}`,
      'background-position: center',
      '-webkit-background-clip: text',
      'background-clip: text',
      '-webkit-text-fill-color: transparent',
    );
  }
  const filter = paintFilter(p);
  if (filter) out.push(`filter: ${filter}`);
  return out.join('; ');
}

/** A ring (an avatar's border) in the paint: the gradient as a background, or its plain colour. */
export function paintRing(p: PaintLike | null | undefined): string {
  if (!p) return '';
  if (p.kind !== 'IMAGE') {
    const image = paintImage({ ...p, repeat: false });
    if (image) return `background: ${image}`;
  }
  const c = paintColor(p);
  return c ? `background: ${c}` : '';
}
