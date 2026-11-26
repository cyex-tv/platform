// An emote's pictures on cdn.cyex.tv: `{host}/{scale}x[_static].webp` (ADR 0003, ADR 0012).

export type Picture = {
  host: string;
  animated: boolean;
  /** Absent on older answers: then only the files tell. */
  processing?: 'READY' | 'PROCESSING' | 'FAILED';
  files: { name: string; width: number; height: number; isStatic: boolean }[];
};

/** Whether the worker has made the emote's files yet. */
export function ready(e: Picture): boolean {
  return e.host !== '' && e.files.some((f) => f.name === '1x.webp');
}

/** The transcode ran out of attempts: the emote never gets files (delete it, upload again). */
export function failed(e: Picture): boolean {
  return e.processing === 'FAILED' && !ready(e);
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

/** Wide emotes (about twice as wide as tall or more) take two cells in a grid. */
export function isWide(e: Picture): boolean {
  const { width, height } = size(e);
  return width / height >= 1.75;
}

/**
 * The density a `1x, 2x, 4x` srcset would pick on a screen with this pixel ratio, out of the
 * scales on offer (the smallest that is sharp enough, else the biggest).
 */
export function densityFor(dpr: number, scales: readonly number[] = [1, 2, 4]): number {
  return scales.find((s) => s >= dpr) ?? scales[scales.length - 1] ?? 1;
}

/**
 * `src` and `srcset` for an emote's <img>, shown at its 1x size or, with `shown`, at that CSS
 * width. While `dpr` is 0 (the server render and hydration) it offers a srcset and the browser
 * picks; after that the file is picked here and there is no srcset: Chromium keeps a removed
 * <img> that has a srcset (or a <picture><source media>) registered as a viewport listener when
 * Svelte has moved it through a DocumentFragment, and with it the whole removed page.
 */
export function pictureFor(e: Picture, dpr: number, still = false, shown?: number): { src: string; srcset: string | undefined } {
  if (shown === undefined) {
    if (dpr === 0) return { src: src(e, 1, still), srcset: srcset(e, still) };
    return { src: src(e, densityFor(dpr) as 1 | 2 | 4, still), srcset: undefined };
  }
  if (dpr === 0) return { src: src(e, 1, still), srcset: srcsetW(e, still) };
  const suffix = still && e.animated ? '_static' : '';
  const files = ([1, 2, 3, 4] as const)
    .map((s) => e.files.find((x) => x.name === `${s}x${suffix}.webp`))
    .filter((f) => f !== undefined);
  const best = files.find((f) => f.width >= shown * dpr) ?? files[files.length - 1];
  return { src: best ? `${e.host}/${best.name}` : src(e, 1, still), srcset: undefined };
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
