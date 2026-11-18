import { describe, expect, it } from 'vitest';
import { fileProblem, nameFromFile, nameProblem, parseTags, tagsProblem } from './rules';

describe('rules', () => {
  it('checks names like the API', () => {
    expect(nameProblem('catJAM')).toBeNull();
    expect(nameProblem('  ')).toBe('Give it a name.');
    expect(nameProblem('two words')).toBe('No spaces in names.');
    expect(nameProblem('x'.repeat(101))).toBe('At most 100 characters.');
    expect(nameProblem('ぽ'.repeat(100))).toBeNull();
  });

  it('makes a name from a file name', () => {
    expect(nameFromFile('catJAM.gif')).toBe('catJAM');
    expect(nameFromFile('cat jam\t2.final.webp')).toBe('catjam2.final');
    expect(nameFromFile('.png')).toBe('');
    expect(nameProblem(nameFromFile(`${'x'.repeat(140)}.png`))).toBeNull();
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

describe('upload batches', () => {
  it('knows the same file picked twice', async () => {
    const { sameFile } = await import('./rules');
    const a = { name: 'cat.png', size: 10, lastModified: 1 };
    expect(sameFile(a, { ...a })).toBe(true);
    expect(sameFile(a, { ...a, size: 11 })).toBe(false);
    expect(sameFile(a, { ...a, name: 'dog.png' })).toBe(false);
  });

  it('explains every refusal the API names, and keeps its own words for names and tags', async () => {
    const { uploadProblem } = await import('./rules');
    expect(uploadProblem('blocked', 'x')).toMatch(/taken down/);
    expect(uploadProblem('too_large', 'the file is over 5 MB')).toMatch(/5 MB, 1000 × 1000 px/);
    expect(uploadProblem('unsupported_format', 'x')).toMatch(/PNG, APNG, GIF or WebP/);
    expect(uploadProblem('rate_limited', 'x')).toMatch(/20 uploads/);
    expect(uploadProblem('bad_name', 'Names have no spaces.')).toBe('Names have no spaces.');
    expect(uploadProblem('something_new', 'As the API said.')).toBe('As the API said.');
    expect(uploadProblem('something_new', '')).toBe('The upload didn’t go through.');
  });
});
