-- Account editors with `emote_upload` upload emotes owned by the account they edit (ADR 0014,
-- amended 2026/11/19). Additive: one nullable column, set only when the uploader isn't the owner.
--
-- - `uploaded_by` names the editor who made the upload; NULL means the owner did (every existing
--   row). The public feed shows an editor's upload "by" them, and the hourly upload limit counts it
--   against the editor as well as the owner.
-- - A deleted uploader leaves the emote with its owner (SET NULL), as `owner_account_id` does.
ALTER TABLE emotes ADD COLUMN uploaded_by uuid REFERENCES accounts (id) ON DELETE SET NULL;
CREATE INDEX emotes_uploaded_by_created ON emotes (uploaded_by, created_at) WHERE uploaded_by IS NOT NULL;
