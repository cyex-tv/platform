// The emote page's keys (EmoteView's tabs), for the shortcut sheet.
import type { ShortcutGroup } from '$lib/shell/shortcuts.svelte';

export const EMOTE_KEYS: ShortcutGroup = {
  id: 'emote',
  title: 'Emote page',
  items: [
    { keys: ['←', '→'], label: 'Switch tabs (Channels, Activity, More), with a tab focused' },
    { keys: ['Home', 'End'], label: 'First or last tab' },
  ],
};
