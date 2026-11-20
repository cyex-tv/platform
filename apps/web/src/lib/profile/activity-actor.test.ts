import { describe, expect, it } from 'vitest';

import { namesActor } from './activity-view';

const me = { id: 'me', displayName: 'lumen_fox' };
const nora = { id: 'nora', displayName: 'Nora' };

describe('namesActor ("by <name>" on an activity line)', () => {
  it('says nothing when the viewer did it themselves', () => {
    expect(namesActor({ kind: 'added', actor: me, editor: null }, 'me')).toBe(false);
  });
  it('names someone else, also to a signed-out viewer', () => {
    expect(namesActor({ kind: 'added', actor: nora, editor: null }, 'me')).toBe(true);
    expect(namesActor({ kind: 'added', actor: nora, editor: null }, null)).toBe(true);
  });
  it('has nobody to name without an actor', () => {
    expect(namesActor({ kind: 'added', actor: null, editor: null }, 'me')).toBe(false);
  });
  it('does not name an editor who removed themselves', () => {
    expect(namesActor({ kind: 'editorRemoved', actor: nora, editor: nora }, 'me')).toBe(false);
    expect(namesActor({ kind: 'editorRemoved', actor: me, editor: nora }, 'x')).toBe(true);
  });
});
