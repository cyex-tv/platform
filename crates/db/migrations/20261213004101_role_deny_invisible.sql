-- Deny, and invisible roles (ADR 0010, amendment 2026/11/22). Additive: two columns and two rows.
--
-- - role_permissions.effect: a role now says Allow or Deny for a permission; no row is Inherit.
--   An account's permission is decided by its most senior role that names it (Deny wins at equal
--   positions; the built-in User role, position 0, comes last). Every existing row was a grant,
--   so the default backfills them all as 'allow' and nothing anyone can do changes.
-- - roles.invisible: never shown publicly (profiles, user cards), even when `public` is on, and
--   left out wherever roles rank people. Staff with roles.manage and the holders still see it.
--   The owners' top role is invisible already (nobody else is shown it), so it says so.
-- - Two everyday permissions, emotes.upload and reports.file, which the User role allows: every
--   account keeps uploading and reporting until a more senior role denies them.
ALTER TABLE role_permissions
    ADD COLUMN effect text NOT NULL DEFAULT 'allow'
    CONSTRAINT role_permissions_effect CHECK (effect IN ('allow', 'deny'));

ALTER TABLE roles ADD COLUMN invisible boolean NOT NULL DEFAULT false;
UPDATE roles SET invisible = true WHERE system = 'root';

INSERT INTO role_permissions (role_id, permission, effect)
SELECT r.id, p, 'allow'
  FROM roles r, unnest(ARRAY['emotes.upload', 'reports.file']) AS t (p)
 WHERE r.system = 'user'
ON CONFLICT DO NOTHING;
