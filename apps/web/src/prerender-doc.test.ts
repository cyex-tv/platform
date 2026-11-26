import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { describe, expect, it } from 'vitest';

const routes = join(__dirname, 'routes');
const doc = readFileSync(join(__dirname, '..', '..', '..', 'docs', 'setup', 'owner-steps.md'), 'utf8');

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
}

/** The URL path of each page route that is prerendered (JSON endpoints aside). */
function prerenderedPages(): string[] {
  const out = new Set<string>();
  for (const f of walk(routes)) {
    if (!/\+page(\.server)?\.ts$/.test(f)) continue;
    if (!/^export const prerender = true;?\s*$/m.test(readFileSync(f, 'utf8'))) continue;
    const dir = relative(routes, join(f, '..')).split(sep).join('/');
    out.add('/' + dir);
  }
  return [...out].sort();
}

describe('owner-steps Cloudflare CSP rule', () => {
  const expression = doc.match(/http\.request\.uri\.path in \{([^}]*)\}/)?.[1] ?? '';
  const listed = new Set([...expression.matchAll(/"([^"]+)"/g)].map((m) => m[1]));

  it('finds the prerendered pages', () => {
    expect(prerenderedPages()).toEqual(expect.arrayContaining(['/developers', '/help', '/guidelines', '/changelog', '/download']));
  });

  it('lists every prerendered page, with and without a trailing slash', () => {
    for (const path of prerenderedPages()) {
      expect(listed, path).toContain(path);
      expect(listed, path + '/').toContain(path + '/');
    }
  });
});
