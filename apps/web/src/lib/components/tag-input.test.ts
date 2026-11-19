import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import TagInput from './TagInput.svelte';
import { addTags, defaultClean, keyAction, pieces, removeAt } from './tag-input';

describe('TagInput rules', () => {
  it('cleans a tag: trimmed, lower case, no leading #', () => {
    expect(defaultClean('  #Pog ')).toBe('pog');
    expect(defaultClean('##a')).toBe('a');
    expect(defaultClean('   ')).toBe('');
  });
  it('Enter (or a comma) adds what is typed; Backspace in an empty field takes the last tag off', () => {
    expect(keyAction('Enter', 'cat', 0)).toBe('add');
    expect(keyAction(',', 'cat', 2)).toBe('add');
    expect(keyAction('Enter', '   ', 2)).toBe('none'); // Enter on nothing lets a form submit
    expect(keyAction('Backspace', '', 2)).toBe('pop');
    expect(keyAction('Backspace', 'c', 2)).toBe('none'); // deletes a character instead
    expect(keyAction('Backspace', '', 0)).toBe('none');
    expect(keyAction('a', 'c', 1)).toBe('none');
  });
  it('adds at the end, without empties or repeats', () => {
    expect(addTags(['cat'], 'Dog')).toEqual({ tags: ['cat', 'dog'], repeated: [], over: [] });
    expect(addTags(['cat'], '#CAT')).toEqual({ tags: ['cat'], repeated: ['cat'], over: [] });
    expect(addTags([], '  ').tags).toEqual([]);
  });
  it('splits pasted lists at commas and new lines', () => {
    expect(pieces('a, b\nc')).toEqual(['a', ' b', 'c']);
    expect(addTags([], 'a, b,,\nc,a').tags).toEqual(['a', 'b', 'c']);
  });
  it('stops at max and says what was left out', () => {
    expect(addTags(['a', 'b'], 'c, d', { max: 3 })).toEqual({ tags: ['a', 'b', 'c'], repeated: [], over: ['d'] });
  });
  it('takes a custom clean (e.g. the API tag pattern)', () => {
    const clean = (t: string) => defaultClean(t).replace(/\s+/g, '_').replace(/[^a-z0-9_]/g, '');
    expect(addTags([], 'Happy Cat!, ✨', { clean }).tags).toEqual(['happy_cat']);
  });
  it('removes one by index', () => {
    expect(removeAt(['a', 'b', 'c'], 1)).toEqual(['a', 'c']);
    expect(removeAt(['a'], 5)).toEqual(['a']);
  });
});

describe('TagInput markup', () => {
  it('draws a chip with a named remove button per tag, and a labelled field', () => {
    const { body } = render(TagInput, { props: { tags: ['cat', 'dog'], onchange: () => {}, label: 'Tags', id: 't' } });
    expect(body).toContain('<label');
    expect(body).toContain('for="t"');
    expect(body.match(/aria-label="Remove #/g)).toHaveLength(2);
    expect(body).toContain('Remove #dog');
    expect(body).toContain('placeholder="Add another"');
  });
  it('says when the list is full but keeps the field on (Backspace still removes)', () => {
    const { body } = render(TagInput, { props: { tags: ['a', 'b'], onchange: () => {}, label: 'Tags', max: 2, id: 't' } });
    expect(body).not.toMatch(/<input[^>]*disabled/);
    expect(body).toContain('Up to 2 tags.');
    expect(body).toContain('aria-describedby="t-note"');
  });
});
