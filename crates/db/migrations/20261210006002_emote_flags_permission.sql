-- A permission (ADR 0024): `emotes.flags`, change any emote's flags after review (visibility,
-- NSFW, zero-width). Additive: the built-in Admin role, which starts with every permission, gets
-- it; no other role changes.
INSERT INTO role_permissions (role_id, permission)
SELECT r.id, 'emotes.flags' FROM roles r WHERE r.system = 'admin'
ON CONFLICT DO NOTHING;
