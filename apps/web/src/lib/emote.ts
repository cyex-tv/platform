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
