-- Two permissions (ADR 0010, amendment 2026/11/19, round 2): `emotes.delete_any` (delete anyone's
-- emote) and `editors.manage_any` (manage anyone's account editors). Additive: the built-in Admin
-- role, which starts with every permission, gets both; no other role changes.
INSERT INTO role_permissions (role_id, permission)
SELECT r.id, p
  FROM roles r, unnest(ARRAY['emotes.delete_any', 'editors.manage_any']) AS t (p)
 WHERE r.system = 'admin'
ON CONFLICT DO NOTHING;
