import { describe, expect, it } from 'vitest';
import { asksForSheet, GLOBAL, HEADER_SEARCH, keyParts, registerShortcuts, shortcuts, type ShortcutGroup } from './shortcuts.svelte';
import { BROWSE_KEYS, SELECT_KEYS } from '$lib/components/browse/keys';
import { REVIEW_DETAIL_KEYS, REVIEW_KEYS } from '$lib/admin/review-keys';
import { EMOTE_KEYS } from '$lib/components/emote/keys';
import { DOCS_KEYS, HELP_KEYS } from '$lib/docs/keys';

const group = (id: string): ShortcutGroup => ({ id, title: id, items: [{ keys: ['X'], label: `${id} key` }] });
const ids = () => shortcuts.groups.map((g) => g.id);

describe('the shortcut registry', () => {
  it('always lists the site-wide keys, after the page’s own', () => {
    expect(ids()).toEqual(['global', 'header-search']);
    const off = registerShortcuts(group('page'));
    expect(ids()).toEqual(['page', 'global', 'header-search']);
    off();
    expect(ids()).toEqual(['global', 'header-search']);
  });

  it('a group leaves when the page that registered it goes', () => {
    const a = registerShortcuts(group('a'));
    const b = registerShortcuts(group('b'));
    expect(ids().slice(0, 2)).toEqual(['a', 'b']);
    a();
    expect(ids()).toEqual(['b', 'global', 'header-search']);
    b();
  });

  it('a group registered twice is listed once and stays until both let go', () => {
    const one = registerShortcuts(group('twice'));
    const two = registerShortcuts(group('twice'));
    expect(ids().filter((i) => i === 'twice')).toHaveLength(1);
    one();
    one(); // a second call does nothing
    expect(ids()).toContain('twice');
    two();
    expect(ids()).not.toContain('twice');
  });

  it('every group has unique ids and labels, and no empty key', () => {
    const all = [GLOBAL, HEADER_SEARCH, BROWSE_KEYS, SELECT_KEYS, REVIEW_KEYS, REVIEW_DETAIL_KEYS, EMOTE_KEYS, DOCS_KEYS, HELP_KEYS];
    expect(new Set(all.map((g) => g.id)).size).toBe(all.length);
    for (const g of all) {
      expect(g.items.length).toBeGreaterThan(0);
      expect(new Set(g.items.map((i) => i.label)).size).toBe(g.items.length);
      for (const i of g.items) for (const k of i.keys) expect(keyParts(k).every((p) => p.length > 0)).toBe(true);
    }
  });

  it('the review keys match the letters the queue handles', () => {
    const letters = REVIEW_KEYS.items.flatMap((i) => i.keys).filter((k) => /^[A-Z]$/.test(k));
    expect(letters.sort()).toEqual(['A', 'P', 'R', 'S', 'U', 'X', 'Z']);
  });
});

describe('keys and combos', () => {
  it('splits combos and names Mod for the platform', () => {
    expect(keyParts('Mod+A', false)).toEqual(['Ctrl', 'A']);
    expect(keyParts('Mod+A', true)).toEqual(['⌘', 'A']);
    expect(keyParts('Shift+Space')).toEqual(['Shift', 'Space']);
    expect(keyParts('/')).toEqual(['/']);
    expect(keyParts('+')).toEqual(['+']);
  });

  it('? opens the sheet, but not typed into a field or with a modifier', () => {
    const ev = (over: Partial<KeyboardEvent> = {}) =>
      ({ key: '?', ctrlKey: false, metaKey: false, altKey: false, defaultPrevented: false, target: null, ...over }) as KeyboardEvent;
    expect(asksForSheet(ev())).toBe(true);
    expect(asksForSheet(ev({ key: '/' }))).toBe(false);
    expect(asksForSheet(ev({ ctrlKey: true }))).toBe(false);
    expect(asksForSheet(ev({ altKey: true }))).toBe(false);
    expect(asksForSheet(ev({ defaultPrevented: true }))).toBe(false);
  });
});
