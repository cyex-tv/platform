// Which file of a badge picture to draw: the cyex badge size ladder (18, 36, 72, 128, 256 and
// 512 px; `1x`, `2x`, `4x`, then `128`, `256`, `512`) and its picking rule, the smallest size at
// or above the drawn size × the screen's pixel ratio, else the largest there is. Pictures made
// before the ladder have only 18, 36 and 72; an animated one may lack an animated 128/256/512
// (over its cap) and then animates at the largest size that has one.

/** The ladder, smallest first: file name stem and px. */
export const BADGE_LADDER = [
  { stem: '1x', px: 18 },
  { stem: '2x', px: 36 },
  { stem: '4x', px: 72 },
  { stem: '128', px: 128 },
  { stem: '256', px: 256 },
  { stem: '512', px: 512 },
] as const;

/** The smallest of `sizes` (ascending) at or above `need`, else the largest; null when empty. */
export function pickSize(sizes: readonly number[], need: number): number | null {
  return sizes.find((px) => px >= need) ?? sizes[sizes.length - 1] ?? null;
}

/** A ladder of sizes: file name stem and px, smallest first (the badge ladder, or paints'). */
export type Ladder = readonly { stem: string; px: number }[];

export type BadgeChoice = { name: string; px: number };

/**
 * The files of a picture that can be drawn, smallest first: the animated (or still) WebP of
 * each size, or with `still` the first frame of an animated picture (its `_static` file, else
 * the file itself, which is all a still picture has).
 */
export function badgeFiles(names: Iterable<string>, still: boolean, ladder: Ladder = BADGE_LADDER): BadgeChoice[] {
  const has = new Set(names);
  const out: BadgeChoice[] = [];
  for (const { stem, px } of ladder) {
    const name = still && has.has(`${stem}_static.webp`) ? `${stem}_static.webp` : `${stem}.webp`;
    if (has.has(name)) out.push({ name, px });
  }
  return out;
}

/** The file to draw a badge `drawn` CSS px wide with on a `dpr` screen (0 or less counts as 1). */
export function pickBadgeFile(names: Iterable<string>, drawn: number, dpr: number, still = false): BadgeChoice | null {
  const files = badgeFiles(names, still);
  const px = pickSize(
    files.map((f) => f.px),
    drawn * (dpr > 0 ? dpr : 1),
  );
  return files.find((f) => f.px === px) ?? null;
}

/**
 * Whether a made picture lacks a size of the ladder (made before 128, 256 and 512 px): neither
 * its file nor its still frame is there. Remaking its files fills them in.
 */
export function lacksLadder(names: Iterable<string>, ladder: Ladder = BADGE_LADDER): boolean {
  const has = new Set(names);
  if (!has.size) return false;
  return ladder.some(({ stem }) => !has.has(`${stem}.webp`) && !has.has(`${stem}_static.webp`));
}

/** The file name stem of the picture at its real size (at most 512 px), beside the ladder. */
export const ORIGINAL_STEM = 'original';

export type OriginalFile = { name: string; width: number; height: number };

/**
 * The original of a badge picture: the picture at its real size (up to 512 px; a bigger upload
 * was scaled down to 512), with `still` its first frame when it is animated. Null for a picture
 * made before originals existed (its largest ladder size stands in).
 */
export function badgeOriginal(files: readonly { name: string; width: number; height: number }[], still = false): OriginalFile | null {
  const pick = (name: string) => files.find((f) => f.name === name);
  const f = (still && pick(`${ORIGINAL_STEM}_static.webp`)) || pick(`${ORIGINAL_STEM}.webp`);
  return f ? { name: f.name, width: f.width, height: f.height } : null;
}

export type FullFile = OriginalFile & { original: boolean };

/**
 * The biggest picture a badge has, for a large preview: its original at its real size, else the
 * largest size of the ladder (a picture made before originals existed), `original` false then.
 * Null when it has no file the picture could be drawn from.
 */
export function badgeFull(files: readonly { name: string; width: number; height: number }[], still = false): FullFile | null {
  const original = badgeOriginal(files, still);
  if (original) return { ...original, original: true };
  const last = badgeFiles(files.map((f) => f.name), still).at(-1);
  if (!last) return null;
  const entry = files.find((f) => f.name === last.name);
  return { name: last.name, width: entry?.width || last.px, height: entry?.height || last.px, original: false };
}

/** Whether a made picture lacks its original (made before it existed). Remaking fills it in. */
export function lacksOriginal(names: Iterable<string>): boolean {
  const has = new Set(names);
  return has.size > 0 && !has.has(`${ORIGINAL_STEM}.webp`) && !has.has(`${ORIGINAL_STEM}_static.webp`);
}

/**
 * How to scale a picture of `px` (its longest side) when it is drawn larger than it is: small
 * pixel art stays crisp (`pixelated`), a bigger picture scales smoothly.
 */
export const enlarge = (px: number): 'pixelated' | 'auto' => (px < 256 ? 'pixelated' : 'auto');
