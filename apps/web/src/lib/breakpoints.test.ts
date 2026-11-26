// Every layout width in the site is one of BP (breakpoints.ts, listed in tokens.css): this reads
// the source and fails on any other `@media` / `matchMedia` width, so the set cannot drift again.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { BP, below, from } from './breakpoints';

const SET: number[] = Object.values(BP);
// One page-specific width kept on purpose: the admin account page's second column at 2200px (the
// look at 1920 must not change), so it is not a layout breakpoint.
const EXCEPTIONS = new Set(['min:2200']);

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    if (statSync(p).isDirectory()) return n === 'node_modules' ? [] : files(p);
    return /\.(svelte|css|ts)$/.test(n) && !/\.test\.ts$/.test(n) ? [p] : [];
  });
}

describe('breakpoints', () => {
  it('names the queries it builds', () => {
    expect(below(BP.narrow)).toBe('(max-width: 899.98px)');
    expect(from(BP.panel)).toBe('(min-width: 1100px)');
  });

  it('every @media / matchMedia width in src is in the set', () => {
    const bad: string[] = [];
    for (const f of files(join(__dirname, '..'))) {
      const src = readFileSync(f, 'utf8');
      for (const line of src.split('\n')) {
        if (!/@media|matchMedia/.test(line) || /@container/.test(line)) continue;
        for (const [, kind, num] of line.matchAll(/\((max|min)-width:\s*([0-9.]+)px\)/g)) {
          const w = Number(num);
          const ok = kind === 'min' ? SET.includes(w) : SET.some((s) => Math.abs(s - 0.02 - w) < 1e-6);
          if (!ok && !EXCEPTIONS.has(`${kind}:${w}`)) bad.push(`${f}: ${kind}-width ${num}`);
        }
      }
    }
    expect(bad).toEqual([]);
  });
});
