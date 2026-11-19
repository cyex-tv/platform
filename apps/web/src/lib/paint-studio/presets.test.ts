import { describe, expect, it } from 'vitest';
import { presetGround, STATIC_PRESETS } from './presets';

const by = (name: string) => STATIC_PRESETS.find((p) => p.name === name)!;

describe('the ground a preset chip reads on', () => {
  it('puts dark paints on the light stage so the name stays readable', () => {
    expect(presetGround(by('Blood Moon'))).toBe('light');
    expect(presetGround(by('Chrome'))).toBe('dark');
  });
  it('keeps bright paints on the dark stage', () => {
    for (const n of ['Toxic', 'Vaporwave', 'Rainbow', 'Sunset Strip']) expect(presetGround(by(n))).toBe('dark');
  });
  it('ignores a transparent end stop', () => {
    expect(presetGround(by('Fade Out'))).toBe('dark');
  });
});
