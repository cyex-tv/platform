// The review queue's keys (/admin/review and its detail dialog, ReviewDetail), for the shortcut
// sheet. Keep in step with the page's `onkeydown` and ReviewDetail's.
import type { ShortcutGroup } from '$lib/shell/shortcuts.svelte';

export const REVIEW_KEYS: ShortcutGroup = {
  id: 'review',
  title: 'Review queue',
  items: [
    { keys: ['Arrow keys'], label: 'Move between uploads' },
    { keys: ['Enter', 'Space'], label: 'Open the upload' },
    { keys: ['A'], label: 'Approve' },
    { keys: ['U'], label: 'Approve as unlisted' },
    { keys: ['P'], label: 'Approve for its owner only' },
    { keys: ['R'], label: 'Reject, with a reason' },
    { keys: ['S'], label: 'Skip: on to the next, no verdict' },
    { keys: ['X'], label: 'Select or unselect, for a verdict on many' },
    { keys: ['Shift+X', 'Shift+Click'], label: 'Select everything from the last pick to here' },
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
    { keys: ['A', 'U', 'P', 'R'], label: 'Approve, unlisted, owner only, reject' },
    { keys: ['S'], label: 'Skip to the next, no verdict' },
    { keys: ['Esc'], label: 'Back to the queue' },
  ],
};
