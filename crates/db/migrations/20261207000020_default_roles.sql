-- Built-in roles (ADR 0010, amendment 2026/11/19). Additive: a new column, an index and three rows.
--
-- - 'user':  "User", position 0. Every account holds it without an assignment, so its
--            permissions are everyone's. Starts empty; its name, description and permissions can
--            change, nothing else.
-- - 'admin': "Admin", position 1000. Assigned like any role; can't be deleted.
-- - 'root':  the top role, position 1000000 (the highest the column allows). Held by the owners
--            (CYEX_TV_OWNER_IDS) only, holds every permission, is never listed to anyone else and
--            can't be granted, edited or deleted through the API. It has no permission rows: its
--            holders get every permission in code, so permissions added later are included.
ALTER TABLE roles ADD COLUMN system text CHECK (system IN ('user', 'admin', 'root'));
CREATE UNIQUE INDEX roles_system ON roles (system) WHERE system IS NOT NULL;

-- A role already called "User" or "Admin" becomes the built-in one. Admin keeps its holders;
-- User loses its assignments below, since every account holds it now.
UPDATE roles SET system = 'user', position = 0
 WHERE lower(name) = 'user' AND NOT EXISTS (SELECT 1 FROM roles WHERE system = 'user');
UPDATE roles SET system = 'admin'
 WHERE lower(name) = 'admin' AND NOT EXISTS (SELECT 1 FROM roles WHERE system = 'admin');

INSERT INTO roles (id, name, description, position, system)
SELECT gen_random_uuid(), 'User', 'Every account has this role. Its permissions are everyone''s.', 0, 'user'
 WHERE NOT EXISTS (SELECT 1 FROM roles WHERE system = 'user');

INSERT INTO roles (id, name, description, position, system)
SELECT gen_random_uuid(), 'Admin', 'Runs cyex.tv day to day.', 1000, 'admin'
 WHERE NOT EXISTS (SELECT 1 FROM roles WHERE system = 'admin');

-- The top role's name never clashes with an existing role.
INSERT INTO roles (id, name, description, position, system)
SELECT gen_random_uuid(),
       CASE WHEN EXISTS (SELECT 1 FROM roles WHERE lower(name) = 'owner') THEN 'Owner (built in)' ELSE 'Owner' END,
       'Every permission. Held by the owners only and shown to nobody else.', 1000000, 'root'
 WHERE NOT EXISTS (SELECT 1 FROM roles WHERE system = 'root');

-- Admin starts with every permission that exists today (crates/shared/src/permissions.rs); the
-- owner can narrow it in the role editor.
INSERT INTO role_permissions (role_id, permission)
SELECT r.id, p
  FROM roles r,
       unnest(ARRAY[
         'emotes.review', 'emotes.takedown', 'emotes.edit_any', 'sets.edit_any', 'global_set.edit',
         'users.view', 'users.restrict', 'roles.manage', 'audit.view', 'cosmetics.manage',
         'cosmetics.grant', 'shop.manage', 'shop.orders', 'codes.manage', 'roles.assign',
         'reports.manage', 'users.restrict_uploads', 'users.restrict_tickets', 'users.block',
         'users.sessions', 'users.notes', 'entitlements.view', 'billing.view',
         'subscriptions.manage'
       ]) AS t (p)
 WHERE r.system = 'admin'
   AND NOT EXISTS (SELECT 1 FROM role_permissions rp WHERE rp.role_id = r.id)
ON CONFLICT DO NOTHING;

-- The root role holds no permission rows (see above), and nobody is assigned the 'user' or 'root'
-- role: 'user' is implicit, 'root' comes from the owner list.
DELETE FROM role_permissions WHERE role_id IN (SELECT id FROM roles WHERE system = 'root');
DELETE FROM account_roles WHERE role_id IN (SELECT id FROM roles WHERE system IN ('user', 'root'));
