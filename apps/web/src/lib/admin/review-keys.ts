// The review queue's keys (/admin/review and its detail dialog, ReviewDetail), for the shortcut
// sheet. Keep in step with the page's `onkeydown` and ReviewDetail's.
import type { ShortcutGroup } from '$lib/shell/shortcuts.svelte';

export const REVIEW_KEYS: ShortcutGroup = {
  id: 'review',
  title: 'Review queue',
  items: [
    { keys: ['Arrow keys'], label: 'Move between uploads' },
    { keys: ['Enter', 'Space'], label: 'Open the upload' },
    { keys: ['A'], label: 'Approve both: public listing and personal use (in the personal queue: approve personal use)' },
    { keys: ['L'], label: 'Approve the public listing only' },
    { keys: ['P'], label: 'Approve personal use only' },
    { keys: ['D'], label: 'Deny the public listing, with a reason (in the personal queue: deny personal use)' },
    { keys: ['N'], label: 'Deny personal use, with a reason' },
    { keys: ['R'], label: 'Reject the upload outright, with a reason' },
    { keys: ['S'], label: 'Skip: on to the next, no verdict' },
    { keys: ['X'], label: 'Select or unselect, for a verdict on many' },
    { keys: ['Shift+X', 'Shift+Click'], label: 'Select everything from the last pick to here, or unselect it all if this one is selected' },
    { keys: ['Mod+A'], label: 'Select every upload on this page (up to 100)' },
    { keys: ['Z'], label: 'Undo the last approval while its notice shows' },
    { keys: ['Esc'], label: 'Clear the selection' },
  ],
};

export const REVIEW_DETAIL_KEYS: ShortcutGroup = {
  id: 'review-detail',
  title: 'An open upload',
  items: [
    { keys: ['←', '→'], label: 'Previous or next upload' },
    { keys: ['A', 'L', 'P'], label: 'Approve both, the listing only, personal use only' },
    { keys: ['D', 'N', 'R'], label: 'Deny the listing, deny personal use, reject' },
    { keys: ['S'], label: 'Skip to the next, no verdict' },
    { keys: ['Esc'], label: 'Back to the queue' },
  ],
};
