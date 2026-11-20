import { describe, expect, it, vi } from 'vitest';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
const { reachMark } = await import('./EmoteTile.svelte');

const e = (over: Record<string, unknown> = {}) => ({ id: 'E', name: 'n', host: 'h', animated: false, files: [], owner: { id: 'me', displayName: 'Me' }, visibility: 'PUBLIC', ...over });

describe('tile marks for your own emotes', () => {
  it('your unlisted and private emotes get a mark with the emote page’s words', () => {
    expect(reachMark(e({ visibility: 'UNLISTED' }), 'me')).toMatchObject({ key: 'unlisted', label: 'Unlisted' });
    expect(reachMark(e({ visibility: 'PRIVATE' }), 'me')).toMatchObject({ key: 'private', label: 'Private' });
    expect(reachMark(e({ flags: { zeroWidth: false, private: true } }), 'me')).toMatchObject({ key: 'private' });
  });

  it('public ones, someone else’s, and signed out: nothing', () => {
    expect(reachMark(e(), 'me')).toBeNull();
    expect(reachMark(e({ visibility: 'UNLISTED' }), 'other')).toBeNull();
    expect(reachMark(e({ visibility: 'UNLISTED' }), null)).toBeNull();
    expect(reachMark(e({ visibility: 'UNLISTED', owner: null }), 'me')).toBeNull();
  });
});
