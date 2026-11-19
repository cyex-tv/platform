import { describe, expect, it } from 'vitest';
import { MAX_UPLOAD_BYTES, cleanTag, fileProblem, isWide, nameFromFile, nameProblem, parseTags, tagsProblem } from './rules';
import { addTags } from './components/tag-input';

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

  it('checks files: the types the API decodes, up to 8 MB', () => {
    for (const type of ['image/png', 'image/apng', 'image/gif', 'image/webp', 'image/jpeg']) expect(fileProblem({ type, size: 1000 })).toBeNull();
    expect(fileProblem({ type: 'image/avif', size: 1000 })).toBe('PNG, APNG, GIF, WebP or JPEG only.');
    expect(fileProblem({ type: 'image/svg+xml', size: 1000 })).toBe('PNG, APNG, GIF, WebP or JPEG only.');
    expect(fileProblem({ type: 'image/gif', size: 6 * 1024 * 1024 })).toBeNull();
    expect(fileProblem({ type: 'image/gif', size: MAX_UPLOAD_BYTES })).toBeNull();
    expect(fileProblem({ type: 'image/gif', size: MAX_UPLOAD_BYTES + 1 })).toBe('It’s 8,1 MB; at most 8 MB.');
    expect(fileProblem({ type: 'image/webp', size: 9.25 * 1024 * 1024 })).toBe('It’s 9,3 MB; at most 8 MB.');
    // No type from the browser: the extension decides.
    expect(fileProblem({ type: '', size: 10, name: 'catJAM.APNG' })).toBeNull();
    expect(fileProblem({ type: '', size: 10, name: 'notes.txt' })).toBe('PNG, APNG, GIF, WebP or JPEG only.');
  });

  it('cleans typed tags into what the API takes', () => {
    expect(cleanTag('  #Pog ')).toBe('pog');
    expect(cleanTag('pepe the frog')).toBe('pepe-the-frog');
    expect(cleanTag('w!ow?')).toBe('wow');
    expect(cleanTag('!!!')).toBe('');
    expect(cleanTag('猫ダンス')).toBe('猫ダンス');
    expect([...cleanTag('x'.repeat(40))]).toHaveLength(32);
    // Every cleaned tag passes the API's check.
    for (const raw of ['Ab C', '#_x-', 'ü ß', 'a'.repeat(50)]) expect(tagsProblem([cleanTag(raw)])).toBeNull();
    // With the tag field's rules: deduped after cleaning, at most 10.
    const r = addTags(['pog'], 'POG, #cat, cat ,dog', { clean: cleanTag, max: 3 });
    expect(r.tags).toEqual(['pog', 'cat', 'dog'].slice(0, 3));
    expect(r.repeated).toEqual(['pog', 'cat']);
    const full = addTags(Array.from({ length: 10 }, (_, i) => `t${i}`), 'one more', { clean: cleanTag, max: 10 });
    expect(full.over).toEqual(['one-more']);
  });

  it('knows a wide emote as the API does (1.75:1 or wider)', () => {
    expect(isWide(175, 100)).toBe(true);
    expect(isWide(174, 100)).toBe(false);
    expect(isWide(112, 32)).toBe(true);
    expect(isWide(32, 32)).toBe(false);
    expect(isWide(10, 0)).toBe(false);
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
    expect(uploadProblem('too_large', 'The file is over 8 MB, the most an upload can be.')).toMatch(/8 MB, 1000 × 1000 px/);
    expect(uploadProblem('unsupported_format', 'x')).toMatch(/PNG, APNG, GIF, WebP or JPEG/);
    expect(uploadProblem('rate_limited', 'x')).toMatch(/20 uploads/);
    expect(uploadProblem('bad_name', 'Names have no spaces.')).toBe('Names have no spaces.');
    expect(uploadProblem('something_new', 'As the API said.')).toBe('As the API said.');
    expect(uploadProblem('something_new', '')).toBe('The upload didn’t go through.');
  });
});
