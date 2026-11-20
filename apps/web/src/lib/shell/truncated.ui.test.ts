// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { clippedUntitled } from './truncated';

function line(attrs: Partial<HTMLElement> = {}, scroll = 200) {
  const el = document.createElement('span');
  el.textContent = 'a very long name';
  el.style.textOverflow = 'ellipsis';
  Object.assign(el, attrs);
  Object.defineProperty(el, 'scrollWidth', { value: scroll });
  Object.defineProperty(el, 'clientWidth', { value: 100 });
  document.body.append(el);
  return el;
}

describe('clippedUntitled', () => {
  it('finds a clipped line with no title', () => {
    expect(clippedUntitled(line())).not.toBeNull();
  });
  it('leaves one that already has a title', () => {
    expect(clippedUntitled(line({ title: 'x' }))).toBeNull();
  });
  it('ignores a line that fits', () => {
    expect(clippedUntitled(line({}, 100))).toBeNull();
  });
  it('finds it from a child', () => {
    const el = line();
    const inner = document.createElement('bdi');
    el.append(inner);
    expect(clippedUntitled(inner)).toBe(el);
  });
});
