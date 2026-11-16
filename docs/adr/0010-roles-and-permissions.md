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
