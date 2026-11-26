// The owner (2026/11/22): "emote set chat previews are not necessary". Neither the set page nor the
// set editor shows one, and the set chat component is gone.
import { describe, expect, it } from 'vitest';
import setPage from '../../routes/sets/[id]/+page.svelte?raw';
import setEditor from '../../routes/(tool)/me/sets/[id]/+page.svelte?raw';

const files = import.meta.glob('../../routes/sets/**/*.svelte', { query: '?raw', import: 'default', eager: true });

describe('no set chat preview', () => {
  it('is not on the set page or in the set editor', () => {
    for (const src of [setPage, setEditor]) {
      expect(src).not.toMatch(/Chat preview|SetChat|ChatPreview/);
    }
  });

  it('has no component left', () => {
    expect(Object.keys(files).some((f) => f.endsWith('SetChat.svelte'))).toBe(false);
  });
});
