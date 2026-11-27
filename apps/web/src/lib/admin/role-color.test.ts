import { describe, expect, it } from 'vitest';
import { dot, hint, normalize, PRESETS, readability } from './role-color';

describe('role colour', () => {
  it('accepts #rrggbb in any case, with or without the #', () => {
    expect(normalize('#3B82F6')).toBe('#3b82f6');
    expect(normalize(' 3b82f6 ')).toBe('#3b82f6');
  });
  it('refuses anything else', () => {
    for (const bad of ['', '#fff', '#12345', '#1234567', 'blue', '#gggggg', 'rgb(0,0,0)', '#12345g']) expect(normalize(bad)).toBeNull();
  });
  it('every preset is valid and readable on both themes', () => {
    for (const p of PRESETS) {
      expect(normalize(p.hex)).toBe(p.hex);
      expect(readability(p.hex).ok, p.name).toBe(true);
    }
  });
  it('says which theme a colour fails on', () => {
    expect(readability('#ffffff')).toMatchObject({ lightOk: false, darkOk: true });
    expect(readability('#000000')).toMatchObject({ lightOk: true, darkOk: false });
    expect(hint('#ffffff')).toContain('light theme');
    expect(hint('#000000')).toContain('dark theme');
    expect(hint('#3b82f6')).toContain('both themes');
  });
  it('dot ignores a missing or malformed colour', () => {
    expect(dot(null)).toBeNull();
    expect(dot('red')).toBeNull();
    expect(dot('#3b82f6')).toBe('#3b82f6');
  });
});
