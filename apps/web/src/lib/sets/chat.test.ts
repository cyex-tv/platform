import { describe, expect, it } from 'vitest';
import { setChatLines } from './chat';

const pic = (name: string, w = 32, zeroWidth = false) => ({
  name, zeroWidth, emote: { host: 'h', animated: false, files: [{ name: '1x.webp', width: w, height: 32, isStatic: false }] },
});
const emotesOf = (l: ReturnType<typeof setChatLines>[number]) => l.parts.filter((p) => 'emote' in p).map((p) => ('emote' in p ? p.emote.name : ''));

describe('setChatLines', () => {
  it('says nothing for a set with no ordinary emote', () => {
    expect(setChatLines([], null)).toEqual([]);
    expect(setChatLines([pic('Over', 32, true)], null)).toEqual([]);
  });
  it('puts a zero-width emote right after an ordinary one, and a wide emote in a line', () => {
    const lines = setChatLines([pic('A'), pic('B'), pic('C'), pic('Wide', 96), pic('D'), pic('Over', 32, true)], null);
    const withOver = lines.find((l) => emotesOf(l).includes('Over'))!;
    const names = emotesOf(withOver);
    expect(names[names.indexOf('Over') - 1]).not.toBe('Over');
    expect(lines.some((l) => emotesOf(l).includes('Wide'))).toBe(true);
  });
  it('lets the owner speak, with their look, and invents the others', () => {
    const look = { badge: null, paint: null };
    const lines = setChatLines([pic('A')], { name: 'Ana', look });
    expect(lines.filter((l) => l.user === 'Ana')).toHaveLength(2);
    expect(lines.find((l) => l.user === 'Ana')!.look).toBe(look);
    expect(lines.filter((l) => l.user !== 'Ana').every((l) => !l.look)).toBe(true);
  });
  it('keeps a one-emote set working', () => {
    expect(setChatLines([pic('Solo')], null)).toHaveLength(6);
  });
});
