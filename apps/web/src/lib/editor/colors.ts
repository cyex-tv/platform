// Recent drawing colours: the last eight picked, newest first, kept in this browser only (a
// convenience; nothing breaks when storage is off).
const KEY = 'cyexEditorColors';
export const PRESETS = ['#ffffff', '#000000', '#ff4d4d', '#ffb020', '#ffe14d', '#3ddc84', '#38bdf8', '#a78bfa'];

const HEX = /^#[0-9a-f]{6}$/;

export function readRecent(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((c): c is string => typeof c === 'string' && HEX.test(c)).slice(0, 8) : [];
  } catch {
    return [];
  }
}

/** `list` with `color` first, at most eight, no repeats. */
export function withRecent(list: readonly string[], color: string): string[] {
  const c = color.toLowerCase();
  if (!HEX.test(c)) return [...list];
  return [c, ...list.filter((x) => x !== c)].slice(0, 8);
}

export function saveRecent(list: readonly string[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {
    /* storage off: the list lasts this visit */
  }
}
