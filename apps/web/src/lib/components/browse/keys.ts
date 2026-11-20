// The emote grid's keys (BrowseGrid, TileMenu, the /search page's Select mode), for the shortcut sheet.
// Keep in step with the handlers: BrowseGrid `onkeydowncapture` and `oncontextmenu`, the page's `onkeydown`.
import type { ShortcutGroup } from '$lib/shell/shortcuts.svelte';
import { CAP } from './selection';

export const BROWSE_KEYS: ShortcutGroup = {
  id: 'browse',
  title: 'Emotes',
  items: [
    { keys: ['Tab'], label: 'Move from emote to emote' },
    { keys: ['Shift+F10', 'Menu'], label: 'Open the focused emote’s menu: add, copy, select' },
    { keys: ['↓', '↑'], label: 'Move through an open menu' },
  ],
};

export const SELECT_KEYS: ShortcutGroup = {
  id: 'select',
  title: 'Select mode',
  items: [
    { keys: ['Space'], label: 'Pick or unpick the focused emote' },
    { keys: ['Shift+Space'], label: 'Pick everything from the last pick to here' },
    { keys: ['Mod+A'], label: `Pick every emote shown, up to ${CAP}` },
    { keys: ['Esc'], label: 'Clear the picks, then leave Select mode' },
  ],
};
