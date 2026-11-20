// The docs pages' keys, for the shortcut sheet: the /developers filter (behaviour.ts) and the /help
// search (its page, with `/` from the header passing to it through data-page-search).
import type { ShortcutGroup } from '$lib/shell/shortcuts.svelte';

export const DOCS_KEYS: ShortcutGroup = {
  id: 'docs',
  title: 'API docs',
  items: [
    { keys: ['Mod+K'], label: 'Find an endpoint' },
    { keys: ['Esc'], label: 'Clear the filter' },
  ],
};

export const HELP_KEYS: ShortcutGroup = {
  id: 'help',
  title: 'Help',
  items: [
    { keys: ['/'], label: 'Search help' },
    { keys: ['Esc'], label: 'Clear the search' },
  ],
};
