import { describe, expect, it } from 'vitest';
import { filterChips, groupOf, groupPrefs, kindInfo, KIND_GROUPS, RETIRED_KINDS } from './prefs';

const pref = (kind: string, required = false) => ({ kind, notify: true, toast: true, required });

describe('notification kinds in words', () => {
  it('names every kind the API lists today', () => {
    const api = [
      'emote.approved', 'emote.rejected', 'emote.failed', 'emote.taken_down', 'emote.merged', 'emote.transfer_requested', 'emote.transferred', 'emote.transfer_declined',
      'emote.transfer_cancelled', 'editor.added', 'editor.changed', 'editor.removed', 'cosmetic.granted', 'cosmetic.revoked',
      'webhook.disabled',
    ];
    const known = KIND_GROUPS.flatMap((g) => g.kinds.map((k) => k.kind));
    expect(known.sort()).toEqual([...api].sort());
  });

  it('has no switch for the retired transfer request kinds, but still words old ones', () => {
    const groups = groupPrefs([pref('emote.transfer_requested', true), pref('emote.transfer_declined'), pref('emote.transferred'), pref('emote.transfer_cancelled')]);
    expect(groups.flatMap((g) => g.items.map((i) => i.kind))).toEqual(['emote.transferred', 'emote.transfer_cancelled']);
    expect([...RETIRED_KINDS].sort()).toEqual(['emote.transfer_declined', 'emote.transfer_requested']);
    expect(groupOf('emote.transfer_requested')).toBe('transfers');
  });

  it('puts the API’s kinds in the site’s groups, unknown ones in Other', () => {
    const groups = groupPrefs([pref('cosmetic.granted'), pref('emote.taken_down', true), pref('emote.new_thing')]);
    expect(groups.map((g) => g.id)).toEqual(['review', 'cosmetics', 'other']);
    expect(groups[0].items[0]).toMatchObject({ kind: 'emote.taken_down', label: 'Taken down', required: true });
    expect(groups[2].items[0].label).toBe('emote.new_thing');
    expect(kindInfo('x').label).toBe('x');
  });

  it('offers a chip per group that has notifications, with counts', () => {
    const chips = filterChips(['emote.approved', 'emote.rejected', 'cosmetic.granted', 'mystery']);
    expect(chips).toEqual([
      { id: 'all', label: 'All', count: 4 },
      { id: 'review', label: 'Your uploads', count: 2 },
      { id: 'cosmetics', label: 'Cosmetics', count: 1 },
      { id: 'other', label: 'Other', count: 1 },
    ]);
    expect(groupOf('editor.added')).toBe('editors');
    expect(filterChips([])).toEqual([{ id: 'all', label: 'All', count: 0 }]);
  });
});
