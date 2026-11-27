// What each permission is called in the role editor, what it lets someone do (the tooltip), and
// the group it sits in. The names are the API's (crates/shared/src/permissions.rs); a test keeps
// this list and that file in step.

export type PermissionInfo = { key: string; label: string; hint: string };
export type PermissionGroup = { id: string; label: string; items: PermissionInfo[] };

const p = (key: string, label: string, hint: string): PermissionInfo => ({ key, label, hint });

export const PERMISSION_GROUPS: PermissionGroup[] = [
  {
    id: 'emotes',
    label: 'Emotes',
    items: [
      p('emotes.upload', 'Upload emotes', 'Upload new emotes. Everyone has it through the User role; deny it on a role above that to stop the people who hold that role.'),
      p('emotes.review', 'Review uploads', 'Approve or deny the public listing of emotes waiting in the review queue, or reject them outright.'),
      p('emotes.review_personal', 'Review personal use', 'Decide which emotes may be used as personal emotes, shown in chat everywhere their owner talks. It opens the personal-use queue and is the personal part of Approve both.'),
      p('emotes.edit_any', 'Edit any emote', "Change the name, tags, flags and visibility of emotes other people uploaded."),
      p('emotes.flags', 'Edit emote flags', 'Including hidden ones: change the visibility and zero-width flag of any emote after review, for example take it off the listings so it is unlisted. The owner is told when the visibility changes.'),
      p('emotes.transfer', 'Transfer emotes', 'Move any emote to another account directly, with no request to accept. The old and the new owner are told.'),
      p('emotes.delete_any', 'Delete any emote', 'Delete emotes other people uploaded, as their owner can. They leave every set they are in.'),
      p('emotes.takedown', 'Take down emotes', 'Remove an emote everywhere it is used and stop its image from being uploaded again.'),
      p('review.settings', 'Edit reject reasons', 'Change the ready reasons reviewers pick from when they reject an upload.'),
    ],
  },
  {
    id: 'sets',
    label: 'Sets',
    items: [
      p('sets.edit_any', 'Edit any set', 'Add, remove and rename emotes in sets other people own.'),
      p('global_set.edit', 'Edit the global set', 'Change the emotes every viewer sees in every channel.'),
    ],
  },
  {
    id: 'users',
    label: 'Users',
    items: [
      p('users.view', 'Look up users', 'Open user pages: linked accounts, restrictions, uploads and roles.'),
      p('users.sessions', 'See sign-in sessions', 'See where a user is signed in and when each session was last used. Never shows tokens.'),
      p('users.notes', 'Staff notes', 'Read and write notes about a user that only staff can see.'),
      p('editors.manage_any', "Manage anyone's editors", "Add, change and remove the editors of any user's account on their behalf, as the owner can."),
    ],
  },
  {
    id: 'moderation',
    label: 'Moderation',
    items: [
      p('users.restrict_uploads', 'Stop uploads', 'Stop a user from uploading emotes, for a time or for good.'),
      p('users.restrict_tickets', 'Stop reports', 'Stop a user from filing reports and tickets, for a time or for good.'),
      p('users.block', 'Block accounts', 'Block a user from signing in at all. Ends every session they have.'),
      p('users.restrict', 'All restrictions', 'Everything in this group at once: stop uploads, stop reports and block accounts.'),
    ],
  },
  {
    id: 'reports',
    label: 'Tickets and reports',
    items: [
      p('reports.file', 'File reports', 'Report emotes and open tickets. Everyone has it through the User role; deny it on a role above that to stop the people who hold that role.'),
      p('reports.manage', 'Handle reports', 'See the report queue, and dismiss reports or act on them.'),
    ],
  },
  {
    id: 'cosmetics',
    label: 'Cosmetics',
    items: [
      p('cosmetics.manage', 'Upload and edit cosmetics', 'Upload badge and paint pictures, make new badges and paints, and change or delete any of them.'),
      p('cosmetics.grant', 'Give cosmetics', 'Give badges and paints to users, take them away, and see who owns what.'),
      p('entitlements.view', 'See where things came from', 'Open the entitlement graph: how each cosmetic and role a user holds was given, and by whom.'),
    ],
  },
  {
    id: 'shop',
    label: 'Shop and billing',
    items: [
      p('shop.manage', 'Manage shop items', 'Create, change, schedule and hide what the shop sells.'),
      p('shop.orders', 'Orders and refunds', 'See every order and refund it.'),
      p('billing.view', 'See billing', "See a user's orders and payments, without refunding them."),
      p('subscriptions.manage', 'Manage subscriptions', "See and change users' subscriptions."),
      p('codes.manage', 'Redeem codes', 'Make and turn off batches of redeem codes.'),
    ],
  },
  {
    id: 'roles',
    label: 'Roles',
    items: [
      p('roles.assign', 'Give roles', 'Give and take away roles below your own, without editing them.'),
      p('roles.manage', 'Make and edit roles', 'Create, edit and delete roles below your own, and give or take them away.'),
    ],
  },
  {
    id: 'system',
    label: 'System',
    items: [
      p('audit.view', 'Read the audit log', 'See every staff action: who did what, to what, and when.'),
      p('site.announce', 'Site announcements', 'Put a line under the header for everyone, with a link, start and end, and end it early.'),
    ],
  },
];

const BY_KEY = new Map(PERMISSION_GROUPS.flatMap((g) => g.items.map((i) => [i.key, i] as const)));

/** A permission the editor doesn't know yet (a newer API): its key, made readable. */
export function permissionInfo(key: string): PermissionInfo {
  const known = BY_KEY.get(key);
  if (known) return known;
  const action = key.split('.').slice(1).join('.') || key;
  const label = action.replace(/[._]+/g, ' ').trim();
  return { key, label: label.charAt(0).toUpperCase() + label.slice(1), hint: key };
}

export const permissionLabel = (key: string) => permissionInfo(key).label;

/**
 * The groups for a list of permission names from the API: every known group with the names the
 * API has, in order, and any names it doesn't know in an "Other" group at the end.
 */
export function groupsFor(available: string[]): PermissionGroup[] {
  const have = new Set(available);
  const out = PERMISSION_GROUPS.map((g) => ({ ...g, items: g.items.filter((i) => have.has(i.key)) })).filter((g) => g.items.length);
  const other = available.filter((k) => !BY_KEY.has(k)).map(permissionInfo);
  if (other.length) out.push({ id: 'other', label: 'Other', items: other });
  return out;
}

export type GroupState = 'all' | 'some' | 'none';

/** How much of a group is picked: drives its checkbox (checked, indeterminate, empty). */
export function groupState(group: PermissionGroup, picked: Iterable<string>): GroupState {
  const set = new Set(picked);
  const n = group.items.filter((i) => set.has(i.key)).length;
  return n === 0 ? 'none' : n === group.items.length ? 'all' : 'some';
}

/**
 * Clicking a group's checkbox: if any item you may change is off, it turns every one you may
 * change on; otherwise it turns them all off. Items you can't change stay as they are.
 */
export function toggleGroup(group: PermissionGroup, picked: string[], canChange: (key: string) => boolean): string[] {
  const set = new Set(picked);
  const on = group.items.some((i) => canChange(i.key) && !set.has(i.key));
  for (const i of group.items) {
    if (!canChange(i.key)) continue;
    if (on) set.add(i.key);
    else set.delete(i.key);
  }
  // Keep the API's order: group order, then anything else that was picked.
  const order = PERMISSION_GROUPS.flatMap((g) => g.items.map((i) => i.key));
  return [...set].sort((a, b) => (order.indexOf(a) + 1 || 1e9) - (order.indexOf(b) + 1 || 1e9) || a.localeCompare(b));
}

/** Any of these opens the admin user pages (lookup and the account page); each part checks its own. */
export const USER_PAGE_PERMISSIONS = [
  'users.view',
  'users.restrict',
  'users.restrict_uploads',
  'users.restrict_tickets',
  'users.block',
  'users.sessions',
  'users.notes',
  'editors.manage_any',
  'emotes.edit_any',
  'emotes.delete_any',
  'emotes.flags',
  'roles.manage',
  'roles.assign',
  'cosmetics.grant',
  'entitlements.view',
  'billing.view',
];
