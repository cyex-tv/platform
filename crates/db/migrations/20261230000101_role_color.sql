-- A role's colour: optional #rrggbb (lower case), shown as a dot on role chips and, for the
-- highest coloured public role, as the ring round the profile picture. Additive: every role
-- starts without one, so nothing changes until a role manager sets one (`updateRole`).
ALTER TABLE roles ADD COLUMN color text;
ALTER TABLE roles ADD CONSTRAINT roles_color_hex CHECK (color IS NULL OR color ~ '^#[0-9a-f]{6}$');
