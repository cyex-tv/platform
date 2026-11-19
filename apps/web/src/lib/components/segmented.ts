// The arrow-key rule of Segmented, kept pure so it is tested without a DOM: which option a key
// moves to, from the focused one, skipping disabled options and wrapping at the ends.
export type SegKey = 'ArrowRight' | 'ArrowDown' | 'ArrowLeft' | 'ArrowUp' | 'Home' | 'End';

const KEYS = new Set<string>(['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp', 'Home', 'End']);

/** The index to move to, or -1 when the key isn't one of ours or nothing is enabled. */
export function nextSegment(key: string, from: number, disabled: boolean[]): number {
  if (!KEYS.has(key)) return -1;
  const n = disabled.length;
  if (!n) return -1;
  const step = key === 'ArrowRight' || key === 'ArrowDown' ? 1 : key === 'ArrowLeft' || key === 'ArrowUp' ? -1 : 0;
  if (step === 0) {
    const order = key === 'Home' ? [...disabled.keys()] : [...disabled.keys()].reverse();
    return order.find((i) => !disabled[i]) ?? -1;
  }
  for (let k = 1; k <= n; k++) {
    const i = (((from + step * k) % n) + n) % n;
    if (!disabled[i]) return i;
  }
  return -1;
}
