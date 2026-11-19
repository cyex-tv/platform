import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { lift } from './bottom-bars.svelte';

const box = (top: number, bottom: number) => ({ top, bottom, height: bottom - top });
const src = (p: string) => readFileSync(new URL(p, import.meta.url), 'utf8');

describe('toast lift over bottom bars', () => {
  it('is 0 with no bars', () => expect(lift([], 900)).toBe(0));

  it('clears a sticky bar stuck 12px above the bottom', () => {
    // The /search select bar: 56px tall, stuck at bottom: 12px.
    expect(lift([box(832, 888)], 900)).toBe(68);
  });

  it('clears the phone action bar fixed to the bottom', () => expect(lift([box(747, 812)], 812)).toBe(65));

  it('ignores hidden bars, bars off screen and bars higher up the page', () => {
    expect(lift([{ top: 0, bottom: 0, height: 0 }], 900)).toBe(0);
    expect(lift([box(950, 1000)], 900)).toBe(0);
    expect(lift([box(300, 360)], 900)).toBe(0);
  });

  it('takes the tallest of several bars', () => expect(lift([box(840, 890), box(800, 900)], 900)).toBe(100));

  it('is wired: both bottom bars register and the toast stack reads the lift', () => {
    expect(src('../components/browse/SelectBar.svelte')).toMatch(/class="bar"[^>]*use:bottomBar/);
    expect(src('../components/emote/EmoteView.svelte')).toMatch(/class="bar" use:bottomBar/);
    expect(src('./Toasts.svelte')).toMatch(/var\(--raise/);
  });
});
