// An emote's pictures on cdn.cyex.tv: `{host}/{scale}x[_static].webp` (ADR 0003, ADR 0012).

export type Picture = {
  host: string;
  animated: boolean;
  files: { name: string; width: number; height: number; isStatic: boolean }[];
};

/** Whether the worker has made the emote's files yet. */
export function ready(e: Picture): boolean {
  return e.host !== '' && e.files.some((f) => f.name === '1x.webp');
}

/** The WebP at a scale (1 to 4). `still` takes the first frame of an animated emote. */
export function src(e: Picture, scale: 1 | 2 | 3 | 4 = 2, still = false): string {
  const suffix = still && e.animated ? '_static' : '';
  return `${e.host}/${scale}x${suffix}.webp`;
}

/** `srcset` for an image shown at 1x size, so sharp screens get the bigger files. */
export function srcset(e: Picture, still = false): string {
  return ([1, 2, 4] as const).map((s) => `${src(e, s, still)} ${s}x`).join(', ');
}

/** The 1x size, for `width` and `height` (no layout shift while it loads). */
export function size(e: Picture): { width: number; height: number } {
  const one = e.files.find((f) => f.name === '1x.webp');
  return { width: one?.width ?? 32, height: one?.height ?? 32 };
}

/** Whether the emote has a still first frame (`1x_static.webp`) to show until it plays. */
export function hasStill(e: Picture): boolean {
  return e.animated && e.files.some((f) => f.name === '1x_static.webp');
}

/**
 * Which picture an animated emote with a still gets, by the site's reduced-motion setting:
 * "reduce" the still, "full" the moving one even when the OS asks for less, "system" the
 * browser's call (a `<source media>` in a <picture>, which also holds without script).
 */
export function stillBy(pref: 'system' | 'reduce' | 'full'): 'still' | 'moving' | 'browser' {
  return pref === 'reduce' ? 'still' : pref === 'full' ? 'moving' : 'browser';
}

/** Wide emotes (about twice as wide as tall or more) take two cells in a grid. */
export function isWide(e: Picture): boolean {
  const { width, height } = size(e);
  return width / height >= 1.75;
}

/** `srcset` with width descriptors, for an emote shown at a set CSS size (pair it with `sizes`). */
export function srcsetW(e: Picture, still = false): string {
  const suffix = still && e.animated ? '_static' : '';
  const out: string[] = [];
  for (const s of [1, 2, 3, 4] as const) {
    const f = e.files.find((x) => x.name === `${s}x${suffix}.webp`);
    if (f) out.push(`${e.host}/${f.name} ${f.width}w`);
  }
  return out.join(', ');
}
