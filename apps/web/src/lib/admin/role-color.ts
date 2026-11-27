// A role's colour: `#rrggbb`, shown as a dot on role chips and as the ring round the profile
// picture of the holder's highest coloured role. The API checks the same shape (`bad_color`).
import { contrast, FLOOR_RATIO, GROUND } from '$lib/cosmetics/paint';

const HEX = /^#[0-9a-f]{6}$/i;

/** Ready-made colours, mid-tones that stay readable on both grounds. */
export const PRESETS: readonly { name: string; hex: string }[] = [
  { name: 'Red', hex: '#e5484d' },
  { name: 'Orange', hex: '#e5731f' },
  { name: 'Amber', hex: '#b07d00' },
  { name: 'Green', hex: '#2f9e5b' },
  { name: 'Teal', hex: '#1f9d99' },
  { name: 'Blue', hex: '#3b82f6' },
  { name: 'Violet', hex: '#8b5cf6' },
  { name: 'Pink', hex: '#d6409f' },
];

/** Typed text as `#rrggbb` (lower case), adding a missing `#`; null when it isn't one. */
export function normalize(raw: string): string | null {
  const t = raw.trim();
  const c = t.startsWith('#') ? t : `#${t}`;
  return HEX.test(c) ? c.toLowerCase() : null;
}

export type Readability = { light: number; dark: number; lightOk: boolean; darkOk: boolean; ok: boolean };

/** Contrast against the light and dark grounds; a dot or ring needs 3:1 (WCAG graphics) on both. */
export function readability(hex: string): Readability {
  const light = contrast(hex, GROUND.light);
  const dark = contrast(hex, GROUND.dark);
  const lightOk = light >= FLOOR_RATIO;
  const darkOk = dark >= FLOOR_RATIO;
  return { light, dark, lightOk, darkOk, ok: lightOk && darkOk };
}

/** One line for the editor: what is wrong on which theme, or that it reads on both. */
export function hint(hex: string): string {
  const r = readability(hex);
  if (r.ok) return `Readable on both themes (${r.light.toFixed(1)}:1 light, ${r.dark.toFixed(1)}:1 dark).`;
  if (!r.lightOk && !r.darkOk) return `Too faint on both themes (${r.light.toFixed(1)}:1 light, ${r.dark.toFixed(1)}:1 dark); pick a mid-tone.`;
  return r.lightOk
    ? `Hard to see on the dark theme (${r.dark.toFixed(1)}:1); pick a lighter colour.`
    : `Hard to see on the light theme (${r.light.toFixed(1)}:1); pick a darker colour.`;
}

/** A role's colour for a dot or ring, or null when it has none or it isn't a hex colour. */
export const dot = (c: string | null | undefined): string | null => (c && HEX.test(c) ? c : null);
