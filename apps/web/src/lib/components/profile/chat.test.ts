import { describe, expect, it } from 'vitest';
import { chatLines } from './chat';

const e = (name: string, zeroWidth = false) => ({ host: 'h', animated: false, files: [], name, zeroWidth });

describe('chatLines', () => {
  it('is empty without a base emote', () => {
    expect(chatLines([])).toEqual([]);
    expect(chatLines([e('a', true)])).toEqual([]);
  });
  it('uses only the given emotes and made-up viewers', () => {
    const lines = chatLines([e('a'), e('b')], 4);
    expect(lines).toHaveLength(4);
    for (const l of lines) for (const p of l.parts) if ('emote' in p) expect(['a', 'b']).toContain(p.emote.name);
  });
});
