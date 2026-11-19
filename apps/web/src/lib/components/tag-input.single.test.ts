import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { cleanTag as rule } from '$lib/rules';
import { cleanTag as url } from './browse/params';

// One tag field for upload, the emote edit page and /search, with one cleaning rule, so a tag
// typed in the search filter finds the emote it was uploaded with.
const files = import.meta.glob('/src/**/*.svelte', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

describe('the one TagInput', () => {
  it('exists once, and nothing else draws a tag field', () => {
    const named = Object.keys(files).filter((f) => /tag.?input/i.test(f.split('/').pop() ?? ''));
    expect(named).toEqual(['/src/lib/components/TagInput.svelte']);
  });

  it('is what upload, the edit page and the search filters use, with the shared rule', () => {
    for (const f of ['/src/lib/upload/UploadForm.svelte', '/src/routes/(tool)/me/emotes/[id]/+page.svelte', '/src/lib/components/browse/BrowseFilters.svelte']) {
      expect(files[f], f).toMatch(/import TagInput from '\$lib\/components\/TagInput\.svelte'/);
      expect(files[f], f).toMatch(/<TagInput[\s\S]*?clean=\{cleanTag\}/);
    }
  });

  it('search cleans tags exactly like uploads store them', () => {
    for (const raw of ['  #Pog ', 'pepe the frog', 'w!ow?', '猫ダンス', 'x'.repeat(40), 'a_b-c']) expect(url(raw)).toBe(rule(raw));
    expect(url('pepe the frog')).toBe('pepe-the-frog');
  });
});
