import { describe, expect, it } from 'vitest';
import { mergedPath, mergeProblem, mergeSummary, mergeTarget } from './merge';

describe('mergeTarget', () => {
  it('takes a bare Snowflake or ULID', () => {
    expect(mergeTarget(' 123456789012345678 ')).toBe('123456789012345678');
    expect(mergeTarget('01JD8X00000000000000000042')).toBe('01JD8X00000000000000000042');
    expect(mergeTarget('01jd8x00000000000000000042')).toBe('01jd8x00000000000000000042');
  });
  it('takes an emote page link on any origin, ignoring the query and the hash', () => {
    expect(mergeTarget('https://cyex.tv/emotes/777?tab=activity#top')).toBe('777');
    expect(mergeTarget('http://localhost:5545/emotes/777/')).toBe('777');
    expect(mergeTarget('cyex.tv/emotes/777')).toBe('777');
    expect(mergeTarget('/emotes/777?x=1')).toBe('777');
    expect(mergeTarget('https://example.org/emotes/01JD8X00000000000000000042')).toBe('01JD8X00000000000000000042');
  });
  it('refuses what is not an emote', () => {
    for (const t of ['', '   ', 'Pog', 'https://cyex.tv/users/12', 'https://cyex.tv/emotes/', 'https://cyex.tv/emotes/pog', '12abc', '0123']) expect(mergeTarget(t), t).toBeNull();
  });
});

describe('mergedPath', () => {
  it('goes to the target and keeps the query', () => {
    expect(mergedPath('42', '?tab=activity')).toBe('/emotes/42?tab=activity');
    expect(mergedPath('42')).toBe('/emotes/42');
  });
});

describe('mergeProblem and mergeSummary', () => {
  it('words each code and falls back to the server message', () => {
    expect(mergeProblem('merge_into_self', 'x')).toMatch(/this emote/);
    expect(mergeProblem('merge_target_unavailable', 'x')).toMatch(/approved/);
    expect(mergeProblem('other', 'Nope.')).toBe('Nope.');
  });
  it('summarises', () => {
    expect(mergeSummary('Pogg', 'Pog', 12, 1)).toBe('Merged Pogg into Pog: 12 sets moved, 1 already had it.');
    expect(mergeSummary('A', 'B', 1, 0)).toBe('Merged A into B: 1 set moved.');
    expect(mergeSummary('A', 'B', 0, 0)).toBe('Merged A into B: no sets to move.');
  });
});
