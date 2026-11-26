// Every set is public (owner, 2026/11/22): nobody picks listed, unlisted or private when making,
// copying or editing a set.
import { describe, expect, it } from 'vitest';
import newSet from '../profile/NewSetDialog.svelte?raw';
import cloneDialog from '../../routes/sets/[id]/CloneDialog.svelte?raw';
import setEditor from '../../routes/(tool)/me/sets/[id]/+page.svelte?raw';

describe('no visibility choice for sets', () => {
  it('is in none of the set forms', () => {
    for (const src of [newSet, cloneDialog, setEditor]) {
      expect(src).not.toMatch(/Who can open|visibilityOptions|visibility:/);
    }
  });
});
