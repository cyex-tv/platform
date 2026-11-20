// The keyboard shortcut sheet's data. Every place that handles keys registers them here, next to
// the handler, while it is on screen (`$effect(() => registerShortcuts(GROUP))`), so the sheet
// (ShortcutSheet, opened with `?` or the footer's Keyboard shortcuts) lists what works on this
// page and nothing that doesn't. The site-wide keys are always there.
import { untrack } from 'svelte';
import { typing } from './search';

/**
 * One shortcut. `keys` lists the ways to press it (alternatives, read "or"); each way is a combo
 * of key names joined with `+`: "Ctrl+A", "Shift+Space", "/". `Mod` is Ctrl, or ⌘ on a Mac.
 */
export type Shortcut = { keys: string[]; label: string };
export type ShortcutGroup = { id: string; title: string; items: Shortcut[] };

/** Keys that work on every page. */
export const GLOBAL: ShortcutGroup = {
  id: 'global',
  title: 'Everywhere',
  items: [
    { keys: ['/'], label: 'Search emotes (or this page’s own search)' },
    { keys: ['?'], label: 'Show keyboard shortcuts' },
    { keys: ['Esc'], label: 'Close a menu, dialog or search' },
  ],
};

/** The header search, while its field has focus. */
export const HEADER_SEARCH: ShortcutGroup = {
  id: 'header-search',
  title: 'Search field',
  items: [
    { keys: ['↓', '↑'], label: 'Move through the results' },
    { keys: ['Enter'], label: 'Open the result, or search for what you typed' },
    { keys: ['Esc'], label: 'Close the results, then clear the field' },
  ],
};

class Registry {
  /** Registered groups in order, each with how many places hold it (a group shown twice is listed once). */
  #held = $state<{ group: ShortcutGroup; n: number }[]>([]);
  open = $state(false);

  /**
   * Adds a group until the returned function is called. Untracked: it is called from effects,
   * which must not come to depend on (and rerun for) the list they change.
   */
  register(group: ShortcutGroup): () => void {
    untrack(() => {
      const at = this.#held.find((h) => h.group.id === group.id);
      if (at) at.n += 1;
      else this.#held.push({ group, n: 1 });
    });
    let done = false;
    return () => {
      if (done) return;
      done = true;
      untrack(() => {
        const i = this.#held.findIndex((h) => h.group.id === group.id);
        if (i < 0) return;
        if (this.#held[i].n > 1) this.#held[i].n -= 1;
        else this.#held.splice(i, 1);
      });
    };
  }

  /** What the sheet lists: the page's groups first (newest page last), then the site-wide keys. */
  get groups(): ShortcutGroup[] {
    return [...this.#held.map((h) => h.group), GLOBAL, HEADER_SEARCH];
  }
}

export const shortcuts = new Registry();

export function registerShortcuts(group: ShortcutGroup): () => void {
  return shortcuts.register(group);
}

export function openShortcuts() {
  shortcuts.open = true;
}

/** Whether a key press asks for the sheet: `?` outside a field, without Ctrl, ⌘ or Alt. */
export function asksForSheet(e: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey' | 'target' | 'defaultPrevented'>): boolean {
  return e.key === '?' && !e.ctrlKey && !e.metaKey && !e.altKey && !e.defaultPrevented && !typing(e.target);
}

/** Splits a combo into the keys to draw, with Mod as Ctrl or ⌘. */
export function keyParts(combo: string, mac = isMac()): string[] {
  // "+" alone is a key; otherwise it joins keys.
  const parts = combo === '+' ? ['+'] : combo.split('+');
  return parts.map((k) => (k === 'Mod' ? (mac ? '⌘' : 'Ctrl') : k));
}

function isMac(): boolean {
  if (typeof navigator === 'undefined') return false;
  const platform = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData?.platform ?? navigator.platform ?? '';
  return /mac|iphone|ipad/i.test(platform);
}
