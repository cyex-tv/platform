// Notification kinds in words: the groups Settings → Notifications and the filter on
// your profile's Notifications tab use, each kind's name and what it is about. The kinds are the API's
// (`myNotificationPrefs`, apps/api/src/site.rs); one the site doesn't know yet goes in Other.
// Pure, so it is tested.

export type KindInfo = { kind: string; label: string; hint: string };
export type KindGroup = { id: string; label: string; kinds: KindInfo[] };

const k = (kind: string, label: string, hint: string): KindInfo => ({ kind, label, hint });

export const KIND_GROUPS: KindGroup[] = [
  {
    id: 'review',
    label: 'Your uploads',
    kinds: [
      k('emote.approved', 'Approved', 'An emote you uploaded passed review.'),
      k('emote.rejected', 'Rejected', 'An emote you uploaded didn’t pass review, with the reason.'),
      k('emote.taken_down', 'Taken down', 'Staff removed one of your emotes everywhere. Always on.'),
    ],
  },
  {
    id: 'transfers',
    label: 'Hand-overs',
    kinds: [
      k('emote.transfer_requested', 'Requests', 'Someone asked to give your emote away. Retired: only an emote’s owner gives it away now.'),
      k('emote.transferred', 'Done', 'An emote moved to you or away from you.'),
      k('emote.transfer_declined', 'Declined', 'A hand-over you asked for was declined. Retired with requests.'),
      k('emote.transfer_cancelled', 'Withdrawn', 'A hand-over request you made or got was withdrawn.'),
    ],
  },
  {
    id: 'editors',
    label: 'Editors',
    kinds: [
      k('editor.added', 'Added', 'You became an editor, or someone became yours.'),
      k('editor.changed', 'Changed', 'What an editor may do changed.'),
      k('editor.removed', 'Removed', 'An editor was removed.'),
    ],
  },
  {
    id: 'cosmetics',
    label: 'Cosmetics',
    kinds: [
      k('cosmetic.granted', 'New badge or paint', 'You got a badge or a paint: bought, from a code or from staff.'),
      k('cosmetic.revoked', 'Taken away', 'A badge or a paint was taken away, for example after a refund.'),
    ],
  },
];

/**
 * Kinds nothing sends any more: editors can't ask for a transfer (ADR 0014, amendment
 * 2026/11/22). Old ones still read as words and filter under Hand-overs; Settings has no switch
 * for them.
 */
export const RETIRED_KINDS: ReadonlySet<string> = new Set(['emote.transfer_requested', 'emote.transfer_declined']);

const GROUP_OF = new Map(KIND_GROUPS.flatMap((g) => g.kinds.map((i) => [i.kind, g.id] as const)));
const INFO = new Map(KIND_GROUPS.flatMap((g) => g.kinds.map((i) => [i.kind, i] as const)));

/** A kind's words; an unknown kind gets its own name. */
export function kindInfo(kind: string): KindInfo {
  return INFO.get(kind) ?? { kind, label: kind, hint: kind };
}

/** The group a kind is filtered under: one of KIND_GROUPS, or `other`. */
export function groupOf(kind: string): string {
  return GROUP_OF.get(kind) ?? 'other';
}

export type Pref = { kind: string; notify: boolean; toast: boolean; required: boolean };

/** The API's preferences in the site's groups and order; kinds it doesn't know go in Other, retired ones nowhere. */
export function groupPrefs(prefs: readonly Pref[]): { id: string; label: string; items: (Pref & KindInfo)[] }[] {
  prefs = prefs.filter((p) => !RETIRED_KINDS.has(p.kind));
  const by = new Map(prefs.map((p) => [p.kind, p]));
  const out = KIND_GROUPS.map((g) => ({
    id: g.id,
    label: g.label,
    items: g.kinds.flatMap((i) => {
      const p = by.get(i.kind);
      return p ? [{ ...i, ...p }] : [];
    }),
  })).filter((g) => g.items.length);
  const other = prefs.filter((p) => !INFO.has(p.kind)).map((p) => ({ ...kindInfo(p.kind), ...p }));
  if (other.length) out.push({ id: 'other', label: 'Other', items: other });
  return out;
}

/** The filter chips for a list: All, then each group that has notifications, with its count. */
export function filterChips(kinds: readonly string[]): { id: string; label: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const kind of kinds) counts.set(groupOf(kind), (counts.get(groupOf(kind)) ?? 0) + 1);
  const chips = [{ id: 'all', label: 'All', count: kinds.length }];
  for (const g of KIND_GROUPS) if (counts.get(g.id)) chips.push({ id: g.id, label: g.label, count: counts.get(g.id)! });
  if (counts.get('other')) chips.push({ id: 'other', label: 'Other', count: counts.get('other')! });
  return chips;
}
