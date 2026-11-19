// Who can open an emote set: listed (on the profile), unlisted (by link) or private. One place for
// the options, the plain hint line under them and the small mark on a set card.
import type { IconName } from '$lib/components/Icon.svelte';
import type { SetKind, SetVisibility } from '$lib/gql/graphql';

export type VisibilityOption = { value: SetVisibility; label: string; icon: IconName; disabled?: boolean };

/** The control's options. A personal set can't be private (the API refuses it). */
export function visibilityOptions(kind: SetKind | string): VisibilityOption[] {
  return [
    { value: 'LISTED', label: 'Listed', icon: 'users' },
    { value: 'UNLISTED', label: 'Unlisted', icon: 'link' },
    { value: 'PRIVATE', label: 'Private', icon: 'lock', disabled: kind === 'PERSONAL' },
  ];
}

/** A new set's choice before anyone picks: as the API, channel sets listed and personal ones unlisted. */
export function defaultVisibility(kind: SetKind | string): SetVisibility {
  return kind === 'PERSONAL' ? 'UNLISTED' : 'LISTED';
}

const HINTS: Record<SetVisibility, string> = {
  LISTED: 'Shows on the profile, and anyone can open it.',
  UNLISTED: 'Not on the profile. Anyone with the link can open it.',
  PRIVATE: 'Only the owner and editors can open it, and chat while a channel shows it.',
};

export const visibilityHint = (v: SetVisibility): string => HINTS[v];

/** The small mark on a set card; listed sets (the usual case) carry none. */
export function setMark(v: SetVisibility | null | undefined): { icon: IconName; label: string } | null {
  if (v === 'UNLISTED') return { icon: 'link', label: 'Unlisted' };
  if (v === 'PRIVATE') return { icon: 'lock', label: 'Private' };
  return null;
}
