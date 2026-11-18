import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { ALL_QUESTIONS, HELP, matches } from './faq';

const ROUTES = resolve(__dirname, '../../routes');
// A site path has a page when one of the route folders (plain or in a group) has a +page.svelte.
const routed = (path: string) => {
  const parts = path.split(/[?#]/)[0].split('/').filter(Boolean);
  return ['', '(tool)'].some((g) => existsSync(resolve(ROUTES, g, ...parts, '+page.svelte')));
};

describe('help questions', () => {
  it('has the eight groups in order', () => {
    expect(HELP.map((g) => g.title)).toEqual([
      'Getting started', 'The extension', 'Uploading', 'Sets and channels', 'Editors', 'Cosmetics', 'Account and privacy', 'Moderation and takedowns',
    ]);
  });

  it('gives every question a unique id that is a usable anchor', () => {
    const ids = ALL_QUESTIONS.map((q) => q.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) expect(id).toMatch(/^[a-z0-9-]+$/);
  });

  it('keeps answers short', () => {
    for (const q of ALL_QUESTIONS) expect(q.hay.length, q.id).toBeLessThan(500);
  });

  it('links only to pages that exist, to its own anchors, or to https', () => {
    const ids = new Set(ALL_QUESTIONS.map((q) => q.id));
    for (const q of ALL_QUESTIONS) {
      for (const s of q.a.flat()) {
        if (!s.href) continue;
        if (s.href.startsWith('#')) expect(ids.has(s.href.slice(1)), s.href).toBe(true);
        else if (s.href.startsWith('/')) expect(routed(s.href), s.href).toBe(true);
        else expect(s.href).toMatch(/^https:\/\//);
      }
    }
  });

  it('matches every word, in the question or the answer, any case', () => {
    const del = ALL_QUESTIONS.find((q) => q.id === 'delete-account')!;
    expect(matches(del, 'delete account')).toBe(true);
    expect(matches(del, 'DELETE')).toBe(true);
    expect(matches(del, 'delete paint')).toBe(false);
    expect(matches(ALL_QUESTIONS.find((q) => q.id === 'what-files')!, '5 mb')).toBe(true);
    expect(matches(del, '   ')).toBe(true);
  });
});
