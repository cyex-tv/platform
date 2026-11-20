# 0010: Roles and permissions

- Status: accepted
- Date: 2026/11/16

## Context
A moderation team exists from day one. Admins must create roles, choose a role's permissions and assign roles, and every change must be traceable. Fixed roles (user, moderator, admin) are not enough.

## Decision
- Tables:
  - `roles` (id, name, description, position, created_by, created_at)
  - `role_permissions` (role_id, permission)
  - `account_roles` (account_id, role_id, granted_by, granted_at)
- An account's permissions are the union of its roles' permissions.
- Permission names are constants in `crates/shared`, never free text:
  - `emotes.review`
  - `emotes.takedown`
  - `emotes.edit_any`
  - `sets.edit_any`
  - `global_set.edit`
  - `users.view`
  - `users.restrict`
  - `roles.manage`
  - `audit.view`

  Later phases add their own (`cosmetics.*`).
- Rules:
  - **Owner:** the account IDs in `CYEX_TV_OWNER_IDS` have every permission and can't be demoted through the API.
  - **No escalation:** you can only grant permissions you hold, and only edit or assign roles below your highest `position`.
  - **No lock-out:** the last holder of `roles.manage` (besides the owner) can't remove it from themselves.
- Every change to roles, permissions or assignments writes `audit_log` with before and after.
- Permission checks happen in one extractor in `crates/auth`. Each handler names the permission it needs, so a missing check is visible in review.

## Consequences
- Flexible enough for a growing team.
- Needs good admin pages (Phase 6) and full tests of the escalation rules (Phase 2).

## Amendment 2026/11/19: built-in roles, finer permissions, timed restrictions

Owner feedback (section F): a default "User" role that starts empty and can be edited, an "Admin" role, and an invisible role above every admin; nobody edits the role they hold; more and finer permissions with readable names, grouped; bans from uploading and from filing tickets, for a time or for good.

### Built-in roles
`roles.system` marks three roles (migration `default_roles`):

| Role | Position | Rules |
|---|---|---|
| Owner (`root`) | 1 000 000, the column's maximum | Held by the accounts in `CYEX_TV_OWNER_IDS` only, without an assignment row. Holds every permission in code (no permission rows), so permissions added later are included. Listed only to its holders; to anyone else it is `not_found`, even by ID. Never granted, edited or deleted through the API (`built_in_role`). |
| Admin (`admin`) | 1000 | Seeded with every permission that existed on 2026/11/19. Assigned like any role; can't be deleted. |
| User (`user`) | 0 | Every account holds it without an assignment, so its permissions are everyone's (`roles::permissions_of` adds them). Starts empty. Its name, description and permissions can change; it is never assigned or taken away, and stays at position 0. |

Custom roles sit between them: positions 1 to 999 999 (`bad_position` otherwise).

### New rules
- **Not your own role:** nobody edits or deletes a role they hold through an assignment, the owner included (`own_role`). Assigning already required a role strictly below your highest, so you can't give away a role at or above your own. The User role is exempt (everyone holds it); only those whose highest role is above 0 can edit it, with the usual no-escalation rule for what it grants.
- The lock-out rule stays; deleting or emptying your own manager role is now refused earlier by "not your own role".
- `roles` and `accountRoles` return `builtIn`, and for the caller `editable` and `assignable`, so pages follow the rules without repeating them.

### Finer permissions
Each narrows part of an older, broader permission; the broad one keeps working, so nothing that worked stops working.

| Permission | What it allows | Broader one that also allows it |
|---|---|---|
| `roles.assign` | give and take roles below yours (without editing roles) | `roles.manage` |
| `reports.manage` | the report queue, dismissing and acting on reports | `emotes.review`, `emotes.takedown` |
| `users.restrict_uploads` | stop uploads | `users.restrict` |
| `users.restrict_tickets` | stop reports and tickets | `users.restrict` |
| `users.block` | block the account (no sign-in, sessions end) | `users.restrict` |
| `users.sessions` | see an account's sessions | |
| `users.notes` | read and write staff notes | |
| `entitlements.view` | the entitlement graph | `cosmetics.grant` |
| `billing.view` | an account's orders and payments | `shop.orders` |
| `subscriptions.manage` | subscriptions (none exist yet) | |

The website names and groups them for people (Emotes, Sets, Users, Moderation, Tickets and reports, Cosmetics, Shop and billing, Roles, System) in `apps/web/src/lib/admin/permissions.ts`; a test there fails when a permission here has no label.

### Timed restrictions
An account has three holds: uploads (`restricted_at`), tickets (`tickets_blocked_at`, new: no reports) and the whole account (`blocked_at`). Each has an end (`*_until`, NULL for good; migration `timed_restrictions`). Every read treats a hold whose end has passed as absent, in Rust (`accounts::get` and every query using its column list) and in SQL (`blocked_until <= now()` beside each `blocked_at IS NULL`), so nothing has to sweep. Holds left behind by an identity (`identity_holds`) carry their end and the tickets hold; merging two keeps the longer one.

`placeRestriction(accountId, { kind, until, reason })` and `liftRestriction(accountId, kind, reason)` need `users.restrict` or the kind's own permission; the target must be below the caller's highest role, and the owner can't be restricted. A reason (1 to 500 characters) is required; ends are in the future and at most ten years out. Every one placed or lifted is a row in `account_restrictions` (who, why, until, lifted by) and an audit row (`account.restriction_placed`, `account.restriction_lifted`). The older `restrictAccount` switch still works, for good and without a reason, and is recorded there too.

### Amendment 2026/11/19, part 2: the staff account dashboard and the entitlement graph
- **Staff notes** (migration `staff_notes`): `staffNotes`, `addStaffNote`, `deleteStaffNote`, all `users.notes`. Notes have an author and a time; only the author (or the owner) deletes one, and a deleted note keeps its row. Audited (`account.note_added`, `account.note_deleted`).
- **Sessions:** `accountSessions(accountId)` with `users.sessions`, the same shape as `mySessions`; never tokens.
- **Billing:** `orders(accountId)` also takes `billing.view` when narrowed to one account; every order and refunds still need `shop.orders`. `accountSubscriptions(accountId)` (`billing.view`, `subscriptions.manage` or `shop.orders`) returns past, current and future subscriptions; none exist yet, so it is empty until they do. The shape (`plan`, `state`, `startsAt`, `endsAt`, `source`) is fixed now so pages can be built against it.
- **Entitlement graph:** `entitlementGraph(accountId)` and `entitlementHolders(kind, id)` (`entitlements.view` or `cosmetics.grant`) return grants: what (badge, paint, role), where it came from (`STAFF`, `PURCHASE` with the order, `CODE` with the batch, `EVENT`, `DEFAULT` for the User role, `OWNER` for the top role), who gave it, when, and when and by whom it was taken away. Built from the entitlement tables, `shop_order_entitlements` of refunded orders, code redemptions, `account_roles` and the audit log (`cosmetic.granted`, `cosmetic.revoked`, `role.assigned`, `role.unassigned`, `role.deleted`). The top role shows only to the owners. A grant made before the audit log knew of it has no time.

### Amendment 2026/11/19, part 3: acting on anyone's emotes and editors
The owner asked for role permissions to edit anyone's emotes and editors. `emotes.edit_any` already renamed, retagged and changed the flags and visibility of anyone's emote, so it stays as it is; two permissions are new, so no role gains anything it didn't have (the built-in Admin role, which starts with every permission, gets both; migration `global_emote_editor_perms`):

| Permission | What it allows | Audited as |
|---|---|---|
| `emotes.delete_any` | delete anyone's emote, as its owner can (it leaves every set) | `emote.deleted`, with the emote as it was and `"via": "emotes.delete_any"` |
| `editors.manage_any` | list, add, change and remove any account's editors on its behalf, with every flag, as the owner can | `account.editor_added`, `_updated`, `_removed` on the owner's account, with `"via": "editors.manage_any"` |

`accountEmotes(accountId)` also takes `emotes.edit_any` and `emotes.delete_any`, so holders can find what to act on. The admin account page shows an Editors panel with `editors.manage_any`, and the emote page shows its edit form and delete button to holders of the matching permission.

### Amendment 2026/11/20: security review fixes
- **`editors.manage_any` respects the owner and seniority.** Staff manage another account's editors only when that account is not one of the owners (`CYEX_TV_OWNER_IDS`; `owner_protected`) and its most senior role is below the staff member's own highest role (`not_below_you`, the same rule as restricting). This covers reading the list too. Below that, staff keep what part 3 gave them: every flag, every set, audited with `"via": "editors.manage_any"`. Before this, an Admin could make themselves an editor of the owner's account with every flag.
- **The User role and staff permissions.** Every account holds the User role, so a staff permission there is everyone's. Adding a permission to it needs the owner unless the permission is on `permissions::USER_ROLE_ALLOWED` (permissions fine for everyone; empty today, since every permission is a staff one): `owner_only` otherwise. Renaming it and taking permissions away stay open to anyone who may edit it.
- **Not your own holds.** Nobody places, changes or lifts a restriction on their own account (`own_account`), whatever they hold: lifting one would undo what someone more senior decided.
