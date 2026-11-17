import { describe, expect, it } from 'vitest';
import { fileProblem, nameProblem, parseTags, tagsProblem } from './rules';

describe('rules', () => {
  it('checks names like the API', () => {
    expect(nameProblem('catJAM')).toBeNull();
    expect(nameProblem('  ')).toBe('Give it a name.');
    expect(nameProblem('two words')).toBe('No spaces in names.');
    expect(nameProblem('x'.repeat(101))).toBe('At most 100 characters.');
    expect(nameProblem('ぽ'.repeat(100))).toBeNull();
  });

  it('parses and checks tags', () => {
    expect(parseTags(' Pog, funny ,,cat,pog ')).toEqual(['pog', 'funny', 'cat']);
    expect(tagsProblem(['ok', 'fine-tag', 'snake_case'])).toBeNull();
    expect(tagsProblem(['no spaces'])).toContain('isn’t a tag');
    expect(tagsProblem(Array.from({ length: 11 }, (_, i) => `t${i}`))).toBe('At most 10 tags.');
  });

  it('checks files', () => {
    expect(fileProblem({ type: 'image/png', size: 1000 })).toBeNull();
    expect(fileProblem({ type: 'image/jpeg', size: 1000 })).toBe('PNG, APNG, GIF or WebP only.');
    expect(fileProblem({ type: 'image/gif', size: 6 * 1024 * 1024 })).toBe('At most 5 MB.');
  });
});
