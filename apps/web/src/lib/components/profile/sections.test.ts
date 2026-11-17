import { describe, expect, it } from 'vitest';
import { currentSection, sectionRail, type Section } from './sections';

const sections: Section<'a' | 'b'>[] = [
  { key: 'a', label: 'A' },
  { key: 'b', label: 'B', count: 3 },
];

describe('profile sections', () => {
  it('defaults to the first, ignores unknown values', () => {
    expect(currentSection(sections, new URLSearchParams(''))).toBe('a');
    expect(currentSection(sections, new URLSearchParams('s=zzz'))).toBe('a');
    expect(currentSection(sections, new URLSearchParams('s=b'))).toBe('b');
  });
  it('links the first section without a parameter and marks the current one', () => {
    const [g] = sectionRail(sections, '/users/1', 'b');
    expect(g.items.map((i) => i.href)).toEqual(['/users/1', '/users/1?s=b']);
    expect(g.items.map((i) => i.current)).toEqual([false, true]);
  });
});
