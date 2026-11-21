// Which file of a paint picture to draw: paint pictures are made 64, 128, 256 and 512 px tall
// (`1x`, `2x`, `4x`, `512`; ADR 0026, never above the original). The picking rule is the badges'
// (see ./badgeFile): the smallest file at or above what is drawn × the screen's pixel ratio, else
// the largest. What is drawn is the height the picture covers: the text box's height, or for a
// name wider than the picture's shape (`background-size: cover`) the name's width over the
// picture's aspect, whichever is more. Where the size isn't known, 2x on a dense screen, else 1x.

import { type Ladder, badgeFiles, lacksLadder } from './badgeFile';

/** The ladder, smallest first: file name stem and px tall. */
export const PAINT_LADDER: Ladder = [
  { stem: '1x', px: 64 },
  { stem: '2x', px: 128 },
  { stem: '4x', px: 256 },
  { stem: '512', px: 512 },
];

/** A paint picture as the API gives it (the parts picking reads). */
export type PaintPicture = {
  width?: number;
  height?: number;
  animated: boolean;
  files: readonly { name: string; width?: number; height?: number }[];
};

/** What a name is drawn at, in CSS px; `dpr` is the screen's pixel ratio (0 or less: 1). */
export type PaintFit = { height?: number; width?: number; dpr?: number };

/** From a font size in px: the text box is one line tall, and a name `chars` long about this wide. */
export function fitOfText(fontPx: number, chars: number, dpr?: number): PaintFit {
  if (!(fontPx > 0)) return { dpr };
  return { height: fontPx * 1.2, width: Math.max(1, chars) * fontPx * 0.6, dpr };
}

/** The shape of the picture (width ÷ height), from its files or its own size; 0 when unknown. */
function aspectOf(img: PaintPicture): number {
  const f = img.files.find((x) => x.width && x.height);
  const [w, h] = f ? [f.width!, f.height!] : [img.width ?? 0, img.height ?? 0];
  return w > 0 && h > 0 ? w / h : 0;
}

/**
 * How many px tall the file must be: the drawn height, or the height the picture is scaled to to
 * cover the name's width, times the pixel ratio. A repeating picture is `auto 100%`: its height
 * is the box's. Null when the size isn't known.
 */
export function neededHeight(fit: PaintFit | undefined, aspect: number, repeat: boolean): number | null {
  if (!fit?.height || !(fit.height > 0)) return null;
  const dpr = fit.dpr && fit.dpr > 0 ? fit.dpr : 1;
  const covers = !repeat && fit.width && aspect > 0 ? fit.width / aspect : 0;
  return Math.max(fit.height, covers) * dpr;
}

/**
 * The file name to draw an image paint with, or null when it has none: the first frame (`_static`)
 * when `still` and the picture is animated, else the animated (or only) file. `fit` is what it is
 * drawn at; without it 2x on a screen of 1.5 or more, else 1x.
 */
export function pickPaintFile(img: PaintPicture, still: boolean, fit?: PaintFit, repeat = false): string | null {
  const files = badgeFiles(
    img.files.map((f) => f.name),
    still && img.animated,
    PAINT_LADDER,
  );
  if (!files.length) return null;
  // Files made smaller than their step (the original was smaller) report their real height.
  const heights = new Map(img.files.map((f) => [f.name, f.height ?? 0]));
  const px = files.map((f) => heights.get(f.name) || f.px);
  const need = neededHeight(fit, aspectOf(img), repeat) ?? ((fit?.dpr ?? 0) >= 1.5 ? 128 : 64);
  // The smallest at or above `need`; else the first of the tallest (a picture smaller than its
  // steps has several files of its own height: the first is the lightest).
  let i = px.findIndex((v) => v >= need);
  if (i < 0) i = px.indexOf(Math.max(...px));
  return files[i]!.name;
}

/** Whether a made paint picture lacks a size of the ladder (made before 512 px). */
export const lacksPaintSizes = (names: Iterable<string>) => lacksLadder(names, PAINT_LADDER);
