// An emote's three visibility states (ADR 0020), as the upload and edit forms offer them and the
// emote page marks them.
import type { EmoteVisibility } from '$lib/api';
import type { IconName } from '$lib/components/Icon.svelte';

export const VISIBILITY_OPTIONS: { value: EmoteVisibility; label: string; icon: IconName }[] = [
  { value: 'PUBLIC', label: 'Public', icon: 'users' },
  { value: 'UNLISTED', label: 'Unlisted', icon: 'link' },
  { value: 'PRIVATE', label: 'Private', icon: 'shield' },
];

export const VISIBILITY_HINT: Record<EmoteVisibility, string> = {
  PUBLIC: 'Anyone can find it and add it to their sets once a moderator approves it.',
  UNLISTED: 'Anyone with the link can open it and add it to their sets. Search, browse and your profile leave it out for others.',
  PRIVATE: 'Only you (and editors you allow) can add it to sets, and search leaves it out. It still shows in chat where your sets are active.',
};

/** The visibility of an emote as the API answers it; older answers only carry `flags.private`. */
export function visibilityOf(e: { visibility?: EmoteVisibility | null; flags: { private: boolean } }): EmoteVisibility {
  return e.visibility ?? (e.flags.private ? 'PRIVATE' : 'PUBLIC');
}
