// The two-column profile tabs (Editors, Account) draw a hairline between their columns. Owner
// screenshot 2026/11/22: the left column's buttons ran into that line (the second column pulled
// itself 40px left with a negative margin, so the line sat on the left column's edge). The line
// now sits in the middle of the gutter, with the same air on both sides, as the Overview tiles'
// column rule. jsdom has no layout, so the rules are checked here and the gap is measured in the
// running app (.claude/runs/2026-11-20-redesign/screens/editors-transfer/shoot.mjs).
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const src = (name: string) => readFileSync(new URL(`./sections/${name}`, import.meta.url), 'utf8');
const px = (css: string, re: RegExp) => Number(re.exec(css)?.[1] ?? NaN);

describe('the column line on the profile tabs', () => {
  for (const name of ['EditorsSection.svelte', 'AccountSection.svelte']) {
    it(`${name}: has a gutter on both sides of the line`, () => {
      const css = src(name);
      expect(css).not.toMatch(/margin-left:\s*-\d/);
      const right = px(css, /\.col:first-child\s*\{[^}]*padding-right:\s*(\d+)px/);
      const left = px(css, /\.col \+ \.col\s*\{[^}]*padding-left:\s*(\d+)px[^}]*border-left:/);
      expect(right).toBeGreaterThanOrEqual(16);
      expect(left).toBe(right);
    });
  }

  it('Overview: the tiles rule has the same air on each side', () => {
    const css = src('Overview.svelte');
    const gap = px(css, /\.tiles\s*\{[^}]*column-gap:\s*(\d+)px[^}]*column-rule:/);
    const editors = px(src('EditorsSection.svelte'), /\.col:first-child\s*\{[^}]*padding-right:\s*(\d+)px/);
    expect(gap / 2).toBe(editors);
  });
});
