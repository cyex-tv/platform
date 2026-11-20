// What the activity renderers (the profile feed, the emote page's feed) share: the icon for each
// kind of line and the rule for when a line says who did it. Pure, so it is tested.
import type { IconName } from '$lib/components/Icon.svelte';
import type { ActivityItem, ActivityKind } from './activity';

/** The icon of each kind of line. A new `ActivityKind` is a type error until it has one. */
export const ICON: Record<ActivityKind, IconName> = {
  added: 'plus',
  removed: 'minus',
  renamed: 'pencil',
  active: 'zap',
  cosmetics: 'brush',
  uploaded: 'upload',
  created: 'folder',
  setRenamed: 'pencil',
  editorAdded: 'user-plus',
  editorRemoved: 'user-minus',
};

/**
 * Whether a line says who did it ("by <name>"): never for the viewer's own doing (it would read
 * "by" your own name), and never for an editor leaving on their own.
 */
export function namesActor(i: Pick<ActivityItem, 'kind' | 'actor' | 'editor'>, viewerId: string | null | undefined): boolean {
  if (!i.actor) return false;
  if (viewerId && i.actor.id === viewerId) return false;
  return !(i.kind === 'editorRemoved' && i.actor.id === i.editor?.id);
}
