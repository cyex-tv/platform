// An emote's three visibility states (ADR 0020): creators choose public or private (the two
// flags below); staff also take an emote off the listings (unlisted). The emote page marks them.
import type { EmoteVisibility } from '$lib/api';
import type { IconName } from '$lib/components/Icon.svelte';

export const VISIBILITY_OPTIONS: { value: EmoteVisibility; label: string; icon: IconName }[] = [
  { value: 'PUBLIC', label: 'Public', icon: 'users' },
  { value: 'UNLISTED', label: 'Unlisted', icon: 'link' },
  { value: 'PRIVATE', label: 'Private', icon: 'shield' },
];

/**
 * The two flags a creator sets (upload and edit, ADR 0020 amendment 2026/11/22): everything else
 * is public by default. Unlisted is for moderators only, and NSFW emotes aren't allowed.
 */
export const FLAG_HINT = {
  overlaying: 'Drawn over the emote before it (zero-width), like a hat or rain.',
  private: 'Only for your own channel: nobody else can find it or add it to their sets.',
} as const;

/** The same states as staff see them on someone else's emote (the Emote flags panel, ADR 0024). */
export const STAFF_VISIBILITY_HINT: Record<EmoteVisibility, string> = {
  PUBLIC: 'Listed: search, browse and the creator’s page show it.',
  UNLISTED: 'Off the listings: anyone with the link can still open it and add it to their sets.',
  PRIVATE: 'Only its owner (and editors they allow) can add it to sets. It still shows in chat where it is already in a set.',
};

/** The visibility of an emote as the API answers it; older answers only carry `flags.private`. */
export function visibilityOf(e: { visibility?: EmoteVisibility | null; flags: { private: boolean } }): EmoteVisibility {
  return e.visibility ?? (e.flags.private ? 'PRIVATE' : 'PUBLIC');
}
